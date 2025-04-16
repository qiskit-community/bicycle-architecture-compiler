use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

use bicycle_isa::{BicycleISA, Pauli, TGateData, TwoBases};
use gross_code_cliffords::native_measurement::NativeMeasurement;
use log::{debug, info};

use crate::small_angle::SingleRotation;
use crate::{architecture::PathArchitecture, operation::Operation};

use crate::{language, small_angle};

use gross_code_cliffords::{CompleteMeasurementTable, MeasurementTableBuilder, PauliString};

use BicycleISA::{Automorphism, JointMeasure, Measure, TGate};

/// Statically store a database to look up measurement implementations on the gross code
/// by sequences of native measurements.
/// Access is read-only and thread safe
static MEASUREMENT_IMPLS: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
    caching_logic()
        .expect("(De)serializing and/or generating a new measurement table should succeed")
});

fn caching_logic() -> Result<CompleteMeasurementTable, Box<dyn Error>> {
    let path = Path::new("tmp/measurement_table");
    try_deserialize(path).or_else(|_| try_create_cache(path))
}

fn try_deserialize(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
    debug!("Attempting to deserialize measurement table");
    let read = std::fs::read(path)?;
    let table = bitcode::deserialize::<CompleteMeasurementTable>(&read)?;
    Ok(table)
}

fn try_create_cache(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
    // Generate new cache file
    info!("Could not deserialize measurement table. Generating new table. This may take a while");
    let parent_path = path.parent().ok_or("Parent path does not exist")?;
    std::fs::create_dir_all(parent_path)?;
    let mut f = File::create(path).expect("Should be able to open the measurement_table file");
    let native_measurements = NativeMeasurement::all();
    let mut table = MeasurementTableBuilder::new(native_measurements);
    table.build();
    let table = table
        .complete()
        .expect("The measurement table should be complete");
    let serialized = bitcode::serialize(&table).expect("The table should be serializable");
    f.write_all(&serialized)
        .expect("The serialized table should be writable to the cache");
    Ok(table)
}

/// Find implementation of a general Pauli measurement on a code module using the pivot
fn general_measurement(
    pivot_basis: Pauli,
    data_basis: &[Pauli],
) -> (&NativeMeasurement, Vec<&NativeMeasurement>) {
    let mut local_basis = vec![pivot_basis];
    local_basis.extend_from_slice(data_basis);
    let local_basis: [Pauli; 12] = local_basis
        .try_into()
        .expect("Should have a length 12 vector");

    let p: PauliString = (&local_basis).into();
    MEASUREMENT_IMPLS.implementation(p)
}

/// Construct GHZ state on a path architecture from start to end
fn ghz_meas(start: usize, blocks: usize) -> Vec<Operation> {
    assert!(blocks > 0);
    let end = start + blocks;
    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();

    let mut ops = vec![];
    // Perform ZZ measurements on adjacent blocks. Alternating even then odd blocks.
    for r in (start..(end - 1))
        .step_by(2)
        .chain(((start + 1)..(end - 1)).step_by(2))
    {
        let op = vec![(r, JointMeasure(z1)), (r + 1, JointMeasure(z1))];
        ops.push(op);
    }

    ops
}

/// Compile a native measurement, including conjugating state preparation and measurement
fn rotation_instructions(native_measurement: &NativeMeasurement) -> Vec<BicycleISA> {
    let mut ops = vec![];
    let ps: [Pauli; 12] = (&native_measurement.measures()).into();
    let (p0, p1) = ps[0]
        .anticommuting()
        .expect("Pivot measurement should not be identity.");
    ops.push(Measure(TwoBases::new(p0, Pauli::I).unwrap()));
    ops.extend(native_measurement.implementation());
    ops.push(Measure(TwoBases::new(p1, Pauli::I).unwrap()));
    ops
}

/// Extend basis to a multiple of 11
fn extend_basis(basis: &mut Vec<Pauli>) {
    while basis.len() % 11 != 0 {
        basis.push(Pauli::I);
    }

    assert!(basis.len() % 11 == 0);
}

/// Compile a Pauli measurement to ISA instructions
pub fn compile_measurement(
    architecture: &PathArchitecture,
    mut basis: Vec<Pauli>,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];
    let n = architecture.data_blocks();

    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
    let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();

    extend_basis(&mut basis);

    // Find implementation for each block
    let rotation_impls: Vec<_> = basis
        .chunks_exact(11)
        .map(|paulis| {
            // Only apply a controlled-Pauli if its non-trivial
            if paulis.iter().all(|p| *p == Pauli::I) {
                None
            } else {
                Some(general_measurement(Pauli::Y, paulis))
            }
        })
        .collect();

    // Apply rotations to blocks that have nontrivial rotations (requires use of pivot)
    for (block_i, (_, rots)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in rots {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    // Prepare initial state
    for block_i in 0..n {
        ops.push(vec![(block_i, Measure(x1))]);
    }

    // Apply native measurements on nontrivial blocks
    for (block_i, (meas, _)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas.implementation() {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Find the range for which we need to prepare a GHZ state
    let first_nontrivial = rotation_impls
        .iter()
        .position(|rot| !rot.is_none())
        .unwrap();
    let last_nontrivial = rotation_impls
        .iter()
        .rposition(|rot| !rot.is_none())
        .unwrap();
    ops.extend(ghz_meas(
        first_nontrivial,
        last_nontrivial - first_nontrivial + 1,
    ));

    // Uncompute GHZ
    for (block_i, opt) in rotation_impls.iter().enumerate() {
        match opt {
            None => ops.push(vec![(block_i, Measure(x1))]), // was trivial
            Some(_) => ops.push(vec![(block_i, Measure(y1))]),
        }
    }

    // Undo rotations on non-trivial blocks
    for (block_i, (_, rots)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in rots {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    ops
}

/// Compile a Pauli rotation of some rational angle to Operations
pub fn compile_rotation(
    architecture: &PathArchitecture,
    mut basis: Vec<Pauli>,
    angle: f64,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];
    let n = architecture.data_blocks();
    assert!(n > 0);
    extend_basis(&mut basis);

    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
    let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();

    // Find implementation for each block
    let rotation_impls: Vec<_> = basis
        .chunks_exact(11)
        .enumerate()
        .map(|(block_i, paulis)| {
            // Only apply a controlled-Pauli if its non-trivial
            if paulis.iter().all(|p| *p == Pauli::I) {
                None
            } else {
                let pivot_basis = if block_i < n { Pauli::Y } else { Pauli::X };
                Some(general_measurement(pivot_basis, paulis))
            }
        })
        .collect();

    // Apply pre-rotations on all blocks if they are non-trivial
    for (block_i, (_, rots)) in rotation_impls
        .iter()
        .enumerate()
        // Skip None values
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in rots {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    // Prepare pivot qubits
    for i in 0..(n - 1) {
        ops.push(vec![(i, Measure(x1))]);
    }
    ops.push(vec![(n, Measure(y1))]);

    // Apply native measurements on nontrivial blocks
    for (block_i, (meas, _)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas.implementation() {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Find the range for which we need to prepare a GHZ state
    let first_nontrivial = rotation_impls
        .iter()
        .position(|rot| !rot.is_none())
        .unwrap_or(n - 1);
    // Prepare GHZ up to and including the magic block
    ops.extend(ghz_meas(first_nontrivial, n - first_nontrivial));

    // Apply small-angle X(φ) rotation on block n
    // TODO: Ignore compile-time Clifford corrections
    let (rots, _cliffords) = small_angle::synthesize_angle_x(angle, 1e-18);
    for rot in rots {
        let tgate_data = match rot {
            SingleRotation::Z { dagger } => TGateData::new(Pauli::Z, false, dagger),
            SingleRotation::X { dagger } => TGateData::new(Pauli::X, false, dagger),
        }
        .unwrap();
        ops.push(vec![(n - 1, TGate(tgate_data))]);
    }

    // Uncompute GHZ state by local measurements on all data blocks (even if they had trivial rotations)
    for (block_i, opt) in rotation_impls.iter().enumerate() {
        match opt {
            None => ops.push(vec![(block_i, Measure(x1))]),
            Some(_) => ops.push(vec![(block_i, Measure(y1))]),
        }
    }
    // The last block also uncomputes by Z measurement
    ops.push(vec![(n - 1, Measure(z1))]);

    // Undo rotations on non-trivial blocks
    for (block_i, (_, rots)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in rots {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    ops
}

/// Compile an iterator of PbcOperations to an iterator over Bicycle ISA instructions.
pub fn compile<T>(architecture: PathArchitecture, ops: T) -> impl Iterator<Item = Operation>
where
    T: Iterator<Item = language::PbcOperation>,
{
    ops.flat_map(move |op| op.compile(&architecture))
}

#[cfg(test)]
mod tests {

    use std::f64::consts::PI;

    use crate::operation::Operations;

    use super::*;

    use bicycle_isa::Pauli::{I, X, Y, Z};

    const CLIFF_ANGLE: f64 = std::f64::consts::PI / 4.0;

    /// Convert a native measurement to a list of Instructions
    fn native_instructions(block: usize, native_measurement: &NativeMeasurement) -> Vec<Operation> {
        native_measurement
            .implementation()
            .into_iter()
            .map(|isa| vec![(block, isa)])
            .collect()
    }

    /// A helper function for setting up test cases by finding native gates
    #[test]
    fn find_native_gates() {
        for i in 1..4_u32.pow(11) {
            let x = (i << 1) | 1; // Set X_0
            let p: PauliString = PauliString(x);
            let (_, rots) = MEASUREMENT_IMPLS.implementation(p);

            let x_bits = p.0 & ((1 << 12) - 1);
            let z_bits = (p.0 & !((1 << 12) - 1)) >> 12;
            let any_bits = x_bits | z_bits;

            if rots.len() == 0 && any_bits.count_ones() == 1 {
                println!("{}", p);
            }
        }
    }

    #[test]
    fn measurement_impls_loading() {
        let p = PauliString(0b11);
        let impl1 = MEASUREMENT_IMPLS.implementation(p);
        let impl2 = MEASUREMENT_IMPLS.implementation(p);
        assert_eq!(impl1, impl2);
    }

    #[test]
    fn test_extend_basis() {
        let mut basis = vec![Y];
        extend_basis(&mut basis);
        let expected = vec![Y, I, I, I, I, I, I, I, I, I];
        assert_eq!(expected, basis);

        let mut basis = vec![I, I, I, I, I, Y];
        extend_basis(&mut basis);
        let expected = vec![I, I, I, I, I, Y, I, I, I, I];
        assert_eq!(expected, basis);
    }

    #[test]
    fn test_ghz_meas() {
        let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
        let arch = PathArchitecture { data_blocks: 2 };

        let ops = ghz_meas(0, arch.data_blocks());

        // One joint operation
        let joint_ops: Vec<_> = ops.iter().filter(|op| op.len() == 2).collect();
        assert_eq!(1, joint_ops.len());

        let zz_meas = vec![(0, JointMeasure(z1)), (1, JointMeasure(z1))];
        assert_eq!(&zz_meas, joint_ops[0]);
    }

    #[test]
    fn compile_native_joint_measurement() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 2 };
        let basis0 = [I, I, X, I, I, I, I, I, I, I, I];
        let basis1 = [X];
        let mut basis = basis0.to_vec();
        basis.append(&mut basis1.to_vec());
        let expected_basis1 = [X, I, I, I, I, I, I, I, I, I, I];

        let (meas0, rot0) = general_measurement(Pauli::X, &basis0);
        let (meas1, rot1) = general_measurement(Pauli::X, &expected_basis1);
        assert!(rot0.is_empty());
        assert!(rot1.is_empty());

        let ops = Operations(compile_measurement(&arch, basis));
        println!("Compiled: {}", ops);

        // One joint operation
        let joint_ops: Vec<_> = ops.0.iter().filter(|op| op.len() == 2).collect();
        assert_eq!(1, joint_ops.len());

        let mut expected = ghz_meas(0, arch.data_blocks());

        expected.append(&mut native_instructions(0, meas0));
        expected.append(&mut native_instructions(1, meas1));

        expected.append(&mut vec![
            vec![(0, Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Measure(TwoBases::new(Z, I).unwrap()))],
        ]);
        let expected = Operations(expected);

        println!("Expected {}", expected);

        for (op0, op1) in expected.0.iter().zip(ops.0.iter()) {
            assert_eq!(op0, op1);
        }

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_joint_measurement() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 2 };
        // Requires 1 rotation
        let paulis0 = [I, I, I, I, X, I, I, I, I, I, I];
        let mut basis = paulis0.to_vec();
        basis.push(X);
        let basis1 = [X, I, I, I, I, I, I, I, I, I, I];

        let ops = Operations(compile_measurement(&arch, basis));
        println!("Compiled: {}", ops);

        let (meas0, rots0) = general_measurement(Pauli::X, &paulis0);
        let (meas1, rots1) = general_measurement(Pauli::X, &basis1);
        assert!(rots0.len() == 1);
        assert!(rots1.is_empty());
        let rot = rots0[0];

        // start with rotation on block 0
        let mut expected: Vec<_> = rotation_instructions(rot)
            .into_iter()
            .map(|op| vec![(0_usize, op)])
            .collect();

        expected.append(&mut ghz_meas(0, arch.data_blocks()));
        expected.append(&mut native_instructions(0, meas0));
        expected.append(&mut native_instructions(1, meas1));
        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Measure(TwoBases::new(Z, I).unwrap()))],
        ]);
        // Unrotate
        expected.extend(
            rotation_instructions(rot)
                .into_iter()
                .map(|op| vec![(0, op)]),
        );
        let expected = Operations(expected);
        println!("Expected {}", expected);

        for (op0, op1) in expected.0.iter().zip(ops.0.iter()) {
            assert_eq!(op0, op1);
        }

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_native_x_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 1 };
        // XXI... is a native measurement so we don't need to apply rotations
        let basis = vec![X];
        let expected_basis = [X, X, I, I, I, I, I, I, I, I, I];

        let ops = Operations(compile_rotation(&arch, basis, CLIFF_ANGLE));
        println!("Compiled: {}", ops);

        let mut expected = ghz_meas(0, arch.data_blocks());

        expected.push(vec![(
            0,
            TGate(TGateData::new(Pauli::X, false, false).unwrap()),
        )]);

        expected.push(vec![(0, Measure(TwoBases::new(Z, I).unwrap()))]);
        let expected = Operations(expected);
        println!("Expected: {}", expected);

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_x_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 1 };
        // IIIIYIIIIIIX requires one rotation
        // I also leave out the dual pivot (in I)
        let basis = [I, I, I, I, I, Y, I, I, I, I];

        let ops = Operations(compile_rotation(&arch, basis.to_vec(), CLIFF_ANGLE));
        println!("Compiled: {}", ops);

        let mut expected = ghz_meas(0, arch.data_blocks());
        let mut expected_basis = vec![X];
        expected_basis.extend_from_slice(&basis);
        expected.push(vec![(
            0,
            TGate(TGateData::new(Pauli::X, false, false).unwrap()),
        )]);

        expected.push(vec![(0, Measure(TwoBases::new(Z, I).unwrap()))]);

        let expected = Operations(expected);
        println!("Expected: {}", expected);

        for (op0, op1) in expected.0.iter().zip(&ops.0) {
            assert_eq!(op0, op1);
        }

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_interblock_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 2 };

        // IIIIIXIIIIIX is native
        let block0_pauli = [I, I, I, I, I, X, I, I, I, I, I];
        let mut basis = block0_pauli.to_vec();
        let block1_pauli = [X, I, I, I, I, I, I, I, I, I];
        let mut basis1 = block1_pauli[0..1].to_vec();
        basis.append(&mut basis1);
        dbg!(&basis);

        let (meas0, rots0) = general_measurement(Pauli::X, &block0_pauli);
        assert!(rots0.is_empty());

        let ops = Operations(compile_rotation(&arch, basis, CLIFF_ANGLE));
        println!("Compiled: {}", ops);

        // Prepare GHZ
        let mut expected: Vec<Operation> = ghz_meas(0, arch.data_blocks());
        // Native measure block 0
        let meas1_impl = meas0
            .implementation()
            .map(|isa| vec![(0, isa.try_into().unwrap())]);
        expected.extend(meas1_impl);

        // Insert rotation on block 1
        let mut expected_basis = vec![X];
        expected_basis.extend_from_slice(&block1_pauli);
        expected.push(vec![(
            1,
            TGate(TGateData::new(Pauli::X, false, false).unwrap()),
        )]);

        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Measure(TwoBases::new(Z, I).unwrap()))],
        ]);

        let expected = Operations(expected);

        println!("Expected: {}", expected);

        for (op1, op2) in expected.0.iter().zip(&ops.0) {
            assert_eq!(op1, op2);
        }

        assert_eq!(ops, expected);

        Ok(())
    }

    #[test]
    fn test_compile_rotation_blocks() {
        let basis = vec![X, X, I, I, I, I, I, I, I, I, I, I];

        let architecture = &PathArchitecture { data_blocks: 2 };

        let compiled = compile_rotation(architecture, basis, CLIFF_ANGLE);
        let compiled_ops = Operations(compiled);
        println!("{compiled_ops}");

        let basis0 = vec![X, X, I, I, I, I, I, I, I, I, I];
        let (meas0, rot0) = general_measurement(Pauli::X, &basis0);
        assert!(rot0.is_empty());

        let mut expected = ghz_meas(0, architecture.data_blocks());
        expected.append(&mut native_instructions(0, meas0));

        let mut expected_basis = [I; 11];
        expected_basis[0] = X;
        expected.push(vec![(
            1,
            TGate(TGateData::new(Pauli::X, false, false).unwrap()),
        )]);

        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Measure(TwoBases::new(Z, I).unwrap()))],
        ]);

        let expected = Operations(expected);

        assert_eq!(expected, compiled_ops);
    }
}

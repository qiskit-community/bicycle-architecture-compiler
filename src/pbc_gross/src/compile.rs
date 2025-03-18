use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

use bicycle_isa::{Pauli, TwoBases};
use gross_code_cliffords::native_measurement::NativeMeasurement;
use log::{debug, info};

use crate::operation::{Instruction, RotationData};
use crate::{architecture::PathArchitecture, operation::Operation};

use crate::language;

use gross_code_cliffords::{CompleteMeasurementTable, MeasurementTableBuilder, PauliString};

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

/// Find implementation of a controlled-rotation by using the pivot
fn controlled_rotation(basis: &[Pauli]) -> (&NativeMeasurement, Vec<&NativeMeasurement>) {
    let mut local_basis = vec![Pauli::X];
    local_basis.extend_from_slice(basis);
    let local_basis: [Pauli; 12] = local_basis
        .try_into()
        .expect("Should have a length 12 vector");

    let p: PauliString = (&local_basis).into();
    MEASUREMENT_IMPLS.implementation(p)
}

/// Construct GHZ state spanning the entire architecture
fn ghz_prep(architecture: &PathArchitecture) -> Vec<Operation> {
    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
    let n = architecture.data_blocks();

    let mut ops = vec![];
    // Init pivots in |+>|0...0>
    ops.push(vec![(0, Instruction::Measure(x1))]);
    for block in 1..n {
        ops.push(vec![(block, Instruction::Measure(z1))]);
    }
    // Perform ZZ measurements on adjacent blocks. Alternating even then odd blocks.
    for r in (0..n - 1).step_by(2).chain((1..n - 1).step_by(2)) {
        let op = vec![
            (r, Instruction::JointMeasure(z1)),
            (r + 1, Instruction::JointMeasure(z1)),
        ];
        ops.push(op);

        // TODO pauli corrections?
    }
    ops
}

/// Compile a native measurement, including conjugating state preparation and measurement
fn rotation_instructions(native_measurement: &NativeMeasurement) -> Vec<Instruction> {
    let mut ops = vec![];
    let ps: [Pauli; 12] = (&native_measurement.measures()).into();
    let (p0, p1) = ps[0]
        .anticommuting()
        .expect("Pivot measurement should not be identity.");
    ops.push(Instruction::Measure(TwoBases::new(p0, Pauli::I).unwrap()));
    for native_impl in native_measurement.implementation() {
        ops.push(
            native_impl
                .try_into()
                .expect("Should be able to convert native measurement isa instructions"),
        );
    }
    ops.push(Instruction::Measure(TwoBases::new(p1, Pauli::I).unwrap()));
    ops
}

/// Extend basis to a multiple of 11, excluding one qubit for dual pivot
/// Dual pivot on injection block is set to I
fn extend_basis(basis: &mut Vec<Pauli>) {
    while (basis.len() + 1) % 11 != 0 {
        basis.push(Pauli::I);
    }
}

/// Compile a Pauli measurement to ISA instructions
pub fn compile_measurement(
    architecture: &PathArchitecture,
    mut basis: Vec<Pauli>,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];

    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();

    extend_basis(&mut basis);
    // Insert identity rotation on the dual pivot
    basis.insert(basis.len() - 6, Pauli::I);
    assert!(basis.len() % 11 == 0);

    // Find implementation for each block
    let rotation_impls: Vec<_> = basis
        .chunks_exact(11)
        .map(|paulis| {
            // Only apply a controlled-Pauli if its non-trivial
            if paulis.iter().all(|p| *p == Pauli::I) {
                None
            } else {
                Some(controlled_rotation(paulis))
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

    ops.extend(ghz_prep(architecture));

    // Apply local measurements on each data block
    for (block_i, rotation_impl) in rotation_impls.iter().enumerate() {
        match rotation_impl {
            // If the rotation was trivial on this block, we just measure out the GHZ state using an X measurement.
            None => ops.push(vec![(block_i, Instruction::Measure(x1))]),
            // Otherwise, we need to apply the local rotation and do a Z measurement.
            Some((meas, _)) => {
                for native_impl in meas.implementation() {
                    ops.push(vec![(
                        block_i,
                        native_impl.try_into().expect(
                            "Should be able to convert native measurement BicycleISA instructions",
                        ),
                    )]);
                }
            }
        }
    }

    // Uncompute GHZ
    // TODO: Must measure blocks in X that have trivial basis
    for (block_i, _) in rotation_impls.iter().enumerate() {
        ops.push(vec![(block_i, Instruction::Measure(z1))]);
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

    // Find implementation for each block
    let chunk_iter = basis.chunks_exact(11);
    let magic_basis = chunk_iter.remainder();
    let rotation_impls: Vec<_> = chunk_iter
        .map(|paulis| {
            // Only apply a controlled-Pauli if its non-trivial, except for the last block,
            // which needs to apply P(φ) in any case
            if paulis.iter().all(|p| *p == Pauli::I) {
                None
            } else {
                Some(controlled_rotation(paulis))
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

    // Prepare GHZ state
    ops.extend(ghz_prep(architecture));

    // Apply native measurements on nontrivial blocks
    for (block_i, (meas, _)) in rotation_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas.implementation().map(|isa| {
            isa.try_into()
                .expect("Should be able to convert native measurement instructions")
        }) {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Apply small-angle X⊗P rotation on block n
    let mut small_rotation_basis = vec![Pauli::X];
    small_rotation_basis.extend_from_slice(magic_basis);
    ops.push(vec![(
        n - 1,
        Instruction::Rotation(RotationData {
            basis: small_rotation_basis
                .try_into()
                .expect("The rotation basis should have 11 Paulis"),
            angle,
        }),
    )]);
    // TODO: Propagate Cliffords

    // Uncompute GHZ state by local measurements on all data blocks (even if they had trivial rotations)
    for (block_i, opt) in rotation_impls.iter().enumerate() {
        match opt {
            None => ops.push(vec![(block_i, Instruction::Measure(x1))]),
            Some(_) => ops.push(vec![(block_i, Instruction::Measure(z1))]),
        }
    }
    // The last block also uncomputes by Z measurement
    ops.push(vec![(n - 1, Instruction::Measure(z1))]);

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

    /// Convert a native measurement to a list of Instructions
    fn native_instructions(
        block: usize,
        native_measurement: &NativeMeasurement,
    ) -> Vec<Vec<(usize, Instruction)>> {
        native_measurement
            .implementation()
            .into_iter()
            .map(|isa| vec![(block, isa.try_into().unwrap())])
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

            if rots.len() == 0 && any_bits.count_ones() == 2 {
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
    fn test_ghz_prep() {
        let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
        let arch = PathArchitecture { data_blocks: 2 };

        let ops = ghz_prep(&arch);

        // One joint operation
        let joint_ops: Vec<_> = ops.iter().filter(|op| op.len() == 2).collect();
        assert_eq!(1, joint_ops.len());

        let zz_meas = vec![
            (0, Instruction::JointMeasure(z1)),
            (1, Instruction::JointMeasure(z1)),
        ];
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

        let (meas0, rot0) = controlled_rotation(&basis0);
        let (meas1, rot1) = controlled_rotation(&expected_basis1);
        assert!(rot0.is_empty());
        assert!(rot1.is_empty());

        let ops = Operations(compile_measurement(&arch, basis));
        println!("Compiled: {}", ops);

        // One joint operation
        let joint_ops: Vec<_> = ops.0.iter().filter(|op| op.len() == 2).collect();
        assert_eq!(1, joint_ops.len());

        let mut expected = ghz_prep(&arch);

        expected.append(&mut native_instructions(0, meas0));
        expected.append(&mut native_instructions(1, meas1));

        expected.append(&mut vec![
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
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

        let (meas0, rots0) = controlled_rotation(&paulis0);
        let (meas1, rots1) = controlled_rotation(&basis1);
        assert!(rots0.len() == 1);
        assert!(rots1.is_empty());
        let rot = rots0[0];

        // start with rotation on block 0
        let mut expected: Vec<_> = rotation_instructions(rot)
            .into_iter()
            .map(|op| vec![(0_usize, op)])
            .collect();

        expected.append(&mut ghz_prep(&arch));
        expected.append(&mut native_instructions(0, meas0));
        expected.append(&mut native_instructions(1, meas1));
        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
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
        let angle = PI / 4.;

        let ops = Operations(compile_rotation(&arch, basis, angle));
        println!("Compiled: {}", ops);

        let mut expected = ghz_prep(&arch);

        expected.push(vec![(
            0,
            Instruction::Rotation(RotationData {
                basis: expected_basis,
                angle,
            }),
        )]);

        expected.push(vec![(
            0,
            Instruction::Measure(TwoBases::new(Z, I).unwrap()),
        )]);
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

        let angle = PI / 16.;
        let ops = Operations(compile_rotation(&arch, basis.to_vec(), angle));
        println!("Compiled: {}", ops);

        let mut expected = ghz_prep(&arch);
        let mut expected_basis = vec![X];
        expected_basis.extend_from_slice(&basis);
        expected.push(vec![(
            0,
            Instruction::Rotation(RotationData {
                basis: expected_basis.try_into().expect("Should have 11 elements"),
                angle,
            }),
        )]);

        expected.push(vec![(
            0,
            Instruction::Measure(TwoBases::new(Z, I).unwrap()),
        )]);

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

        let (meas0, rots0) = controlled_rotation(&block0_pauli);
        assert!(rots0.is_empty());
        let angle = -PI / 32.;

        let ops = Operations(compile_rotation(&arch, basis, angle));
        println!("Compiled: {}", ops);

        // Prepare GHZ
        let mut expected: Vec<Vec<(usize, Instruction)>> = ghz_prep(&arch);
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
            Instruction::Rotation(RotationData {
                basis: expected_basis.try_into().expect("Should have 11 elements"),
                angle,
            }),
        )]);

        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
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
        let angle = -0.125;
        let architecture = &PathArchitecture { data_blocks: 2 };

        let compiled = compile_rotation(architecture, basis, angle);
        let compiled_ops = Operations(compiled);
        println!("{compiled_ops}");

        let basis0 = vec![X, X, I, I, I, I, I, I, I, I, I];
        let (meas0, rot0) = controlled_rotation(&basis0);
        assert!(rot0.is_empty());

        let mut expected = ghz_prep(architecture);
        expected.append(&mut native_instructions(0, meas0));

        let mut expected_basis = [I; 11];
        expected_basis[0] = X;
        expected.push(vec![(
            1,
            Instruction::Rotation(RotationData {
                basis: expected_basis,
                angle,
            }),
        )]);

        // Uncompute GHZ
        expected.append(&mut vec![
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
        ]);

        let expected = Operations(expected);

        assert_eq!(expected, compiled_ops);
    }
}

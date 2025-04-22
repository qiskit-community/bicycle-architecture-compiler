use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::LazyLock;

use bicycle_isa::{BicycleISA, Pauli, TGateData, TwoBases};
use gross_code_cliffords::native_measurement::NativeMeasurement;
use log::{debug, info};

use crate::language::AnglePrecision;
use crate::small_angle::SingleRotation;
use crate::{architecture::PathArchitecture, operation::Operation};

use crate::{language, small_angle};

use gross_code_cliffords::{
    CompleteMeasurementTable, MeasurementTableBuilder, NativeMeasurementImpl, PauliString,
};

use BicycleISA::{JointMeasure, Measure, TGate};

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
) -> (NativeMeasurementImpl, Vec<NativeMeasurementImpl>) {
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
fn rotation_instructions(native_measurement: &NativeMeasurementImpl) -> [BicycleISA; 5] {
    let mut ops = [BicycleISA::CSSInitPlus; 5];
    let pivot_pauli = native_measurement.measures().get_pauli(0);
    let (p0, p1) = pivot_pauli
        .anticommuting()
        .expect("Pivot measurement should not be identity.");
    ops[0] = Measure(TwoBases::new(p0, Pauli::I).unwrap());
    ops[1..4].copy_from_slice(&native_measurement.implementation());
    ops[4] = Measure(TwoBases::new(p1, Pauli::I).unwrap());
    ops
}

/// Extend basis to a multiple of 11
fn extend_basis<T>(basis: T) -> Vec<Pauli>
where
    T: IntoIterator<Item = Pauli>,
{
    let mut basis: Vec<Pauli> = basis.into_iter().collect();
    while basis.len() % 11 != 0 {
        basis.push(Pauli::I);
    }

    assert!(basis.len() % 11 == 0);
    basis
}

/// Compile a Pauli measurement to ISA instructions
pub fn compile_measurement(architecture: &PathArchitecture, basis: Vec<Pauli>) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];
    let n = architecture.data_blocks();

    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
    let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();

    let basis = extend_basis(basis);

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
    assert!(rotation_impls.len() <= n);

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
    // TODO: Prepare state only on qubits that are in the range of the measurement
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
    basis: Vec<Pauli>,
    angle: AnglePrecision,
    accuracy: AnglePrecision,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];
    let n = architecture.data_blocks();
    assert!(n > 0);
    let basis = extend_basis(basis);

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
                let pivot_basis = if block_i < n - 1 { Pauli::Y } else { Pauli::X };
                Some(general_measurement(pivot_basis, paulis))
            }
        })
        .collect();
    assert!(rotation_impls.len() <= n);

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
    ops.push(vec![(n - 1, Measure(y1))]);

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
        .position(|support| !support.is_none())
        .unwrap_or(n - 1);
    // Prepare GHZ up to and including the magic block
    ops.extend(ghz_meas(first_nontrivial, n - first_nontrivial));

    // Apply small-angle X(φ) rotation on block n
    // TODO: Ignore compile-time Clifford corrections
    let (rots, _cliffords) = small_angle::synthesize_angle_x(angle, accuracy);
    for rot in rots {
        let tgate_data = match rot {
            SingleRotation::Z { dagger } => TGateData::new(Pauli::Z, false, dagger),
            SingleRotation::X { dagger } => TGateData::new(Pauli::X, false, dagger),
        }
        .unwrap();
        ops.push(vec![(n - 1, TGate(tgate_data))]);
    }

    // Uncompute GHZ state by local measurements on all data blocks (even if they had trivial rotations)
    for (block_i, opt) in rotation_impls.iter().enumerate().take(n - 1) {
        match opt {
            None => ops.push(vec![(block_i, Measure(x1))]),
            Some(_) => ops.push(vec![(block_i, Measure(y1))]),
        }
    }
    // The last block uncomputes by Z measurement
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

#[cfg(test)]
mod tests {

    use crate::operation::Operations;

    use super::*;

    use bicycle_isa::Pauli::{I, X, Y, Z};

    use rand::{
        distr::{Distribution, StandardUniform},
        seq::IndexedRandom,
    };

    static CLIFF_ANGLE: LazyLock<AnglePrecision> =
        LazyLock::new(|| AnglePrecision::PI / AnglePrecision::lit("4.0"));
    const ACCURACY: AnglePrecision = AnglePrecision::lit("1e-10");

    /// Convert a native measurement to a list of Operations
    fn native_instructions(
        block: usize,
        native_measurement: &NativeMeasurementImpl,
    ) -> Vec<Operation> {
        native_measurement
            .implementation()
            .into_iter()
            .map(|isa| vec![(block, isa)])
            .collect()
    }

    fn find_random_native_measurement(pivot_basis: Pauli) -> NativeMeasurementImpl<'static> {
        let mut native_measurements: Vec<NativeMeasurementImpl> = vec![];
        for i in 1..4_usize.pow(11) {
            let mut bits = i;
            let mut ps: Vec<Pauli> = vec![];
            for _ in 0..11 {
                let p_bits = bits & 3;
                bits >>= 2;
                ps.push(
                    p_bits
                        .try_into()
                        .expect("Should be able to convert 2 bits to Pauli"),
                );
            }
            assert_eq!(11, ps.len());

            let pauli_arr: [Pauli; 12] = std::iter::once(pivot_basis)
                .chain(ps.into_iter())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap();
            let p: PauliString = (&pauli_arr).into();
            assert_eq!(pauli_arr, <[Pauli; 12]>::from(&p));

            let (meas, rots) = MEASUREMENT_IMPLS.implementation(p);
            if rots.is_empty() {
                native_measurements.push(meas);
            }
        }

        *native_measurements.choose(&mut rand::rng()).unwrap()
    }

    fn find_native_gates() {
        let arr = [I; 12];
        let mut p: PauliString = (&arr).into();

        for i in 1..4_u32.pow(11) {
            let x = (i << 1) | 1; // Set X_0
            let p: PauliString = PauliString(x);
            let (_, rots) = MEASUREMENT_IMPLS.implementation(p);

            let x_bits = p.0 & ((1 << 12) - 1);
            let z_bits = (p.0 & !((1 << 12) - 1)) >> 12;
            let any_bits = x_bits | z_bits;

            if rots.is_empty() && any_bits.count_ones() == 1 {
                println!("{}", p);
            }
        }
    }

    /// Generate random non-trivial PauliStrings acting on 11 qubits
    fn random_nontrivial_paulistrings() -> impl Iterator<Item = PauliString> {
        StandardUniform
            .sample_iter(rand::rng())
            .map(|p: PauliString| p.zero_pivot())
            .filter(|p| p.0 != 0)
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
        basis = extend_basis(basis);
        let expected = vec![Y, I, I, I, I, I, I, I, I, I, I];
        assert_eq!(expected, basis);

        let mut basis = vec![I, I, I, I, I, Y];
        basis = extend_basis(basis);
        let expected = vec![I, I, I, I, I, Y, I, I, I, I, I];
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

    mod measurement {

        use super::*;

        /// State prep for nontrivial measurement
        fn prep() -> impl Iterator<Item = Operation> {
            std::iter::repeat(Measure(TwoBases::new(Pauli::X, Pauli::I).unwrap()))
                .enumerate()
                .map(|e| vec![e])
        }

        /// State prep for nontrivial measurement
        fn unprep() -> impl Iterator<Item = Operation> {
            std::iter::repeat(Measure(TwoBases::new(Pauli::Y, Pauli::I).unwrap()))
                .enumerate()
                .map(|e| vec![e])
        }

        #[test]
        fn compile_native_joint_measurement() -> Result<(), Box<dyn Error>> {
            let arch = PathArchitecture { data_blocks: 2 };
            let meas0 = find_random_native_measurement(Y);
            let basis0: [Pauli; 12] = (&meas0.measures()).into();
            let meas1 = find_random_native_measurement(Y);
            let basis1: [Pauli; 12] = (&meas1.measures()).into();
            // Drop pivots
            let basis: Vec<Pauli> = basis0[1..]
                .iter()
                .chain(basis1[1..].iter())
                .copied()
                .collect();

            let ops = Operations(compile_measurement(&arch, basis));
            println!("Compiled: {}", ops);

            // One joint operation
            let joint_ops: Vec<_> = ops.0.iter().filter(|op| op.len() == 2).collect();
            assert_eq!(1, joint_ops.len());

            let mut expected: Vec<Operation> = prep().take(2).collect();
            expected.append(&mut native_instructions(0, &meas0));
            expected.append(&mut native_instructions(1, &meas1));
            expected.extend(ghz_meas(0, arch.data_blocks()));
            expected.extend(unprep().take(2));

            let expected = Operations(expected);

            println!("Expected {}", expected);

            for (op0, op1) in expected.0.iter().zip(ops.0.iter()) {
                assert_eq!(op0, op1);
            }

            assert_eq!(expected, ops);

            Ok(())
        }

        #[test]
        fn compile_multiblock() -> Result<(), Box<dyn Error>> {
            for blocks in 2..10 {
                let arch = PathArchitecture {
                    data_blocks: blocks,
                };
                // Requires 1 rotation
                let mut ps: Vec<_> = random_nontrivial_paulistrings().take(blocks).collect();
                for p in ps.iter_mut() {
                    p.set_pauli(0, Y);
                }
                let implementations: Vec<_> = ps
                    .iter()
                    .map(|p| MEASUREMENT_IMPLS.implementation(*p))
                    .collect();
                let basis: Vec<Pauli> = ps
                    .iter()
                    // Drop the pivot Pauli
                    .flat_map(|p| <[Pauli; 12]>::from(p).into_iter().skip(1))
                    .collect();

                let ops = Operations(compile_measurement(&arch, basis));
                println!("Compiled: {}", ops);

                let mut expected: Vec<Operation> = vec![];

                // pre-rotations
                for (block_i, (_, rots)) in implementations.iter().enumerate() {
                    for rot in rots {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }

                expected.extend(prep().take(blocks));

                // measurements
                for (block_i, (meas, _)) in implementations.iter().enumerate() {
                    expected.extend(native_instructions(block_i, meas).into_iter());
                }
                expected.append(&mut ghz_meas(0, arch.data_blocks()));
                expected.extend(unprep().take(blocks));
                // post-rotations
                for (block_i, (_, rots)) in implementations.iter().enumerate() {
                    for rot in rots {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }
                let expected = Operations(expected);
                println!("Expected {}", expected);

                for (op0, op1) in expected.0.iter().zip(ops.0.iter()) {
                    assert_eq!(op0, op1);
                }

                assert_eq!(expected, ops);
            }

            Ok(())
        }
    }

    mod rotation {

        use super::*;

        /// State prep for nontrivial rotation
        fn prep(blocks: usize) -> impl Iterator<Item = Operation> {
            let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();
            let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
            let mut out = vec![x1; blocks];
            out[blocks - 1] = y1;
            out.into_iter().map(Measure).enumerate().map(|e| vec![e])
        }

        /// State measurement for nontrivial rotation
        fn unprep(blocks: usize) -> impl Iterator<Item = Operation> {
            let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();
            let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
            let mut out = vec![y1; blocks];
            out[blocks - 1] = z1;
            out.into_iter().map(Measure).enumerate().map(|e| vec![e])
        }

        #[test]
        fn compile_native_rotation() -> Result<(), Box<dyn Error>> {
            let arch = PathArchitecture { data_blocks: 1 };
            let meas = find_random_native_measurement(Pauli::X);

            let ps: [Pauli; 12] = (&meas.measures()).into();
            let basis: Vec<Pauli> = ps[1..].to_vec();
            dbg!(&basis);

            let ops = Operations(compile_rotation(&arch, basis, *CLIFF_ANGLE, ACCURACY));
            println!("Compiled: {}", ops);

            let mut expected: Vec<_> = prep(1).collect();
            expected.extend(meas.implementation().map(|isa| vec![(0, isa)]));
            expected.push(vec![(
                0,
                TGate(TGateData::new(Pauli::X, false, false).unwrap()),
            )]);
            expected.extend(unprep(1));
            let expected = Operations(expected);
            println!("Expected: {}", expected);

            assert_eq!(expected, ops);

            Ok(())
        }

        #[test]
        fn compile_multiblock() -> Result<(), Box<dyn Error>> {
            for blocks in 2..10 {
                let arch = PathArchitecture {
                    data_blocks: blocks,
                };
                let mut ps: Vec<_> = random_nontrivial_paulistrings().take(blocks).collect();
                for p in ps.iter_mut().take(blocks - 1) {
                    p.set_pauli(0, Y);
                }
                ps[blocks - 1].set_pauli(0, X);

                let implementations: Vec<_> = ps
                    .iter()
                    .map(|p| MEASUREMENT_IMPLS.implementation(*p))
                    .collect();
                let basis: Vec<Pauli> = ps
                    .iter()
                    // Drop the pivot Pauli
                    .flat_map(|p| <[Pauli; 12]>::from(p).into_iter().skip(1))
                    .collect();

                let ops = Operations(compile_rotation(&arch, basis, *CLIFF_ANGLE, ACCURACY));
                println!("Compiled: {}", ops);

                let mut expected: Vec<Operation> = vec![];

                // pre-rotations
                for (block_i, (_, rots)) in implementations.iter().enumerate() {
                    for rot in rots {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }

                expected.extend(prep(blocks));

                // measurements
                for (block_i, (meas, _)) in implementations.iter().enumerate() {
                    expected.extend(native_instructions(block_i, meas).into_iter());
                }
                expected.append(&mut ghz_meas(0, arch.data_blocks()));
                expected.push(vec![(
                    blocks - 1,
                    TGate(TGateData::new(Pauli::X, false, false).unwrap()),
                )]);
                expected.extend(unprep(blocks));

                // post-rotations
                for (block_i, (_, rots)) in implementations.iter().enumerate() {
                    for rot in rots {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }
                let expected = Operations(expected);
                println!("Expected {}", expected);

                for (i, (op0, op1)) in expected.0.iter().zip(ops.0.iter()).enumerate() {
                    assert_eq!(op0, op1, "Unequal at index {i}");
                }

                assert_eq!(expected, ops);
            }

            Ok(())
        }
    }
}

use bicycle_isa::{BicycleISA, Pauli, TGateData, TwoBases};
use gross_code_cliffords::decomposition::NativeMeasurementImpl;

use crate::language::AnglePrecision;
use crate::small_angle::SingleRotation;
use crate::{architecture::PathArchitecture, operation::Operation};

use crate::basis_changer::BasisChanger;
use crate::small_angle;

use gross_code_cliffords::{CompleteMeasurementTable, PauliString};

use BicycleISA::{JointMeasure, Measure, TGate};

// Statically store a database to look up measurement implementations on the gross code
// by sequences of native measurements.
// Access is read-only and thread safe
// static ABC: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
//     caching_logic()
//         .expect("(De)serializing and/or generating a new measurement table should succeed")
// });

// fn caching_logic() -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     let path = Path::new("tmp/measurement_table");
//     try_deserialize(path).or_else(|_| try_create_cache(path))
// }

// fn try_deserialize(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     debug!("Attempting to deserialize measurement table");
//     let read = std::fs::read(path)?;
//     let table = bitcode::deserialize::<CompleteMeasurementTable>(&read)?;
//     Ok(table)
// }

// fn try_create_cache(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     // Generate new cache file
//     info!("Could not deserialize measurement table. Generating new table. This may take a while");
//     let parent_path = path.parent().ok_or("Parent path does not exist")?;
//     std::fs::create_dir_all(parent_path)?;
//     let mut f = File::create(path).expect("Should be able to open the measurement_table file");
//     let native_measurements = NativeMeasurement::all();
//     let mut table = MeasurementTableBuilder::new(native_measurements, GROSS_MEASUREMENT);
//     table.build();
//     let table = table
//         .complete()
//         .expect("The measurement table should be complete");
//     let serialized = bitcode::serialize(&table).expect("The table should be serializable");
//     f.write_all(&serialized)
//         .expect("The serialized table should be writable to the cache");
//     Ok(table)
// }

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

fn select_basis_change(p_expected: Pauli, p_pivot: Pauli) -> BasisChanger {
    match (p_expected, p_pivot) {
        (Pauli::Z, Pauli::Z) | (Pauli::X, Pauli::X) | (Pauli::Y, Pauli::Y) => {
            BasisChanger::default()
        }
        (Pauli::Y, Pauli::X) => BasisChanger::new(Pauli::Y, p_pivot, Pauli::Z).unwrap(),
        (Pauli::Y, Pauli::Z) => BasisChanger::new(Pauli::Y, p_pivot, Pauli::X).unwrap(),
        (Pauli::X, Pauli::Z) => BasisChanger::new(p_pivot, Pauli::Y, Pauli::X).unwrap(),
        (Pauli::X, Pauli::Y) => BasisChanger::new(p_pivot, Pauli::Z, Pauli::X).unwrap(),
        (Pauli::Z, Pauli::Y) => unreachable!(), // Cannot change joint ZZ to ZY.
        (Pauli::Z, Pauli::X) => BasisChanger::new(Pauli::Z, Pauli::Y, p_pivot).unwrap(),
        (_, Pauli::I) => unreachable!(),
        (Pauli::I, _) => unreachable!(),
    }
}

/// Stores the basis change that is applied to each block
struct BlockBases(pub Vec<BasisChanger>);

impl BlockBases {
    fn change_basis(&self, op: Operation) -> Operation {
        op.into_iter()
            .map(|(block_i, isa)| (block_i, self.0[block_i].change_isa(isa)))
            .collect()
    }
}

/// Compile a Pauli measurement to ISA instructions
pub fn compile_measurement(
    architecture: &PathArchitecture,
    measurement_table: &CompleteMeasurementTable,
    basis: Vec<Pauli>,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];
    let n = architecture.data_blocks();

    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();
    let y1 = TwoBases::new(Pauli::Y, Pauli::I).unwrap();

    let basis = extend_basis(basis);

    // Find implementation for each block
    let block_instrs = basis.chunks_exact(11).map(|paulis| {
        // Only apply a controlled-Pauli if its non-trivial
        if paulis.iter().all(|p| *p == Pauli::I) {
            (None, BasisChanger::default())
        } else {
            let mut ps = vec![Pauli::I];
            ps.extend_from_slice(paulis);
            let p: PauliString = (&ps[..]).try_into().unwrap();
            let meas_impl = measurement_table.min_data(p);

            // Y |-> p_pivot.
            let p_pivot = meas_impl.measures().get_pauli(0);
            let changer = select_basis_change(Pauli::Y, p_pivot);
            (Some(meas_impl), changer)
        }
    });

    let (meas_impls, basis_changes): (Vec<_>, Vec<_>) = block_instrs.unzip();
    let block_basis = BlockBases(basis_changes);
    assert!(meas_impls.len() <= n);

    // Apply rotations to blocks that have nontrivial rotations (requires use of pivot)
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in meas_impl.rotations() {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    // Prepare initial state
    // TODO: Prepare state only on qubits that are in the range of the measurement
    ops.extend(
        (0..n)
            .map(|block_i| vec![(block_i, Measure(x1))])
            .map(|o| block_basis.change_basis(o)),
    );

    // Apply native measurements on nontrivial blocks
    // Do _not_ change basis
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas_impl.base_measurement().implementation() {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Find the range for which we need to prepare a GHZ state
    let first_nontrivial = meas_impls.iter().position(|rot| !rot.is_none()).unwrap();
    let last_nontrivial = meas_impls.iter().rposition(|rot| !rot.is_none()).unwrap();
    let mut middle_ops = ghz_meas(first_nontrivial, last_nontrivial - first_nontrivial + 1);

    // Uncompute GHZ
    for (block_i, opt) in meas_impls.iter().enumerate() {
        match opt {
            None => middle_ops.push(vec![(block_i, Measure(x1))]), // was trivial
            Some(_) => middle_ops.push(vec![(block_i, Measure(y1))]),
        }
    }
    // Change basis on middle ops
    ops.extend(
        middle_ops
            .into_iter()
            .map(|op| block_basis.change_basis(op)),
    );

    // Undo rotations on non-trivial blocks
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in meas_impl.rotations() {
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
    measurement_table: &CompleteMeasurementTable,
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
    let block_instrs = basis.chunks_exact(11).enumerate().map(|(block_i, paulis)| {
        // Only apply a controlled-Pauli if its non-trivial
        if paulis.iter().all(|p| *p == Pauli::I) {
            (None, BasisChanger::default())
        } else {
            let mut ps = vec![Pauli::I];
            ps.extend_from_slice(paulis);
            let p: PauliString = (&ps[..]).try_into().unwrap();
            let meas_impl = measurement_table.min_data(p);

            let p_pivot = meas_impl.measures().get_pauli(0);

            let changer = if block_i < n - 1 {
                // Y |-> p_pivot.
                select_basis_change(Pauli::Y, p_pivot)
            } else {
                // magic module next to factory
                // X |-> p_pivot
                select_basis_change(Pauli::X, p_pivot)
            };

            (Some(measurement_table.min_data(p)), (changer))
        }
    });
    let (meas_impls, basis_changes): (Vec<_>, Vec<_>) = block_instrs.unzip();
    let block_basis = BlockBases(basis_changes);
    assert!(meas_impls.len() <= n);

    // Apply pre-rotations on all blocks if they are non-trivial
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        // Skip None values
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in meas_impl.rotations() {
            ops.extend(
                rotation_instructions(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    // Prepare pivot qubits

    ops.extend(
        (0..(n - 1))
            .map(|block_i| vec![(block_i, Measure(x1))])
            .chain(std::iter::once(vec![(n - 1, Measure(y1))]))
            .map(|op| block_basis.change_basis(op)),
    );

    // Apply native measurements on nontrivial blocks
    // Do _not_ apply basis change
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas_impl.base_measurement().implementation() {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Find the range for which we need to prepare a GHZ state
    let first_nontrivial = meas_impls
        .iter()
        .position(|support| !support.is_none())
        .unwrap_or(n - 1);
    // Prepare GHZ up to and including the magic block
    let mut middle_ops = ghz_meas(first_nontrivial, n - first_nontrivial);

    // Apply small-angle X(φ) rotation on block n
    // TODO: Ignore compile-time Clifford corrections
    let (rots, _cliffords) = small_angle::synthesize_angle_x(angle, accuracy);
    for rot in rots {
        let tgate_data = match rot {
            SingleRotation::Z { dagger } => TGateData::new(Pauli::Z, false, dagger),
            SingleRotation::X { dagger } => TGateData::new(Pauli::X, false, dagger),
        }
        .unwrap();
        middle_ops.push(vec![(n - 1, TGate(tgate_data))]);
    }

    // Uncompute GHZ state by local measurements on all data blocks (even if they had trivial rotations)
    for (block_i, opt) in meas_impls.iter().enumerate().take(n - 1) {
        match opt {
            None => middle_ops.push(vec![(block_i, Measure(x1))]),
            Some(_) => middle_ops.push(vec![(block_i, Measure(y1))]),
        }
    }
    // The last block uncomputes by Z measurement
    middle_ops.push(vec![(n - 1, Measure(z1))]);

    // Change basis on middle_ops
    ops.extend(
        middle_ops
            .into_iter()
            .map(|op| block_basis.change_basis(op)),
    );

    // Undo rotations on non-trivial blocks
    for (block_i, meas_impl) in meas_impls
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for nat_measure in meas_impl.rotations() {
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

    static GROSS_TABLE: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
        let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
        builder.build();
        builder.complete().expect("Table building should succeed")
    });

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

    fn find_random_native_measurement(
        measurement_table: &CompleteMeasurementTable,
        pivot_basis: Pauli,
    ) -> NativeMeasurementImpl {
        let mut native_measurements = vec![];
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
            assert_eq!(pauli_arr, <[Pauli; 12]>::from(p));

            let meas_impl = measurement_table.implementation(p);
            if meas_impl.rotations().is_empty() {
                native_measurements.push(*meas_impl.base_measurement());
            }
        }

        *native_measurements.choose(&mut rand::rng()).unwrap()
    }

    // #[allow(dead_code)]
    // fn find_native_gates() {
    //     for i in 1..4_u32.pow(11) {
    //         let x = (i << 1) | 1; // Set X_0
    //         let p: PauliString = PauliString(x);
    //         let meas_impl = MEASUREMENT_IMPLS.implementation(p);

    //         let x_bits = p.0 & ((1 << 12) - 1);
    //         let z_bits = (p.0 & !((1 << 12) - 1)) >> 12;
    //         let any_bits = x_bits | z_bits;

    //         if meas_impl.rotations().is_empty() && any_bits.count_ones() == 1 {
    //             println!("{}", p);
    //         }
    //     }
    // }

    /// Generate random non-trivial PauliStrings acting on 11 qubits
    fn random_nontrivial_paulistrings() -> impl Iterator<Item = PauliString> {
        StandardUniform
            .sample_iter(rand::rng())
            .map(|p: PauliString| p.zero_pivot())
            .filter(|p| p.0 != 0)
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

    #[test]
    fn basis_change() {
        for p_expected in [X, Y, Z] {
            for p_pivot in [X, Y, Z] {
                if p_expected == Z && p_pivot == Y {
                    continue;
                }
                let changer = select_basis_change(p_expected, p_pivot);

                assert!(changer.change_pauli(Z) != Y);
                assert_eq!(p_pivot, changer.change_pauli(p_expected));
            }
        }
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
            let meas0 = find_random_native_measurement(&GROSS_TABLE, Y);
            let basis0: [Pauli; 12] = meas0.measures().into();
            let meas1 = find_random_native_measurement(&GROSS_TABLE, Y);
            let basis1: [Pauli; 12] = meas1.measures().into();
            // Drop pivots
            let basis: Vec<Pauli> = basis0[1..]
                .iter()
                .chain(basis1[1..].iter())
                .copied()
                .collect();

            let ops = Operations(compile_measurement(&arch, &GROSS_TABLE, basis));
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
                let ps: Vec<_> = random_nontrivial_paulistrings().take(blocks).collect();
                let implementations: Vec<_> = ps.iter().map(|p| GROSS_TABLE.min_data(*p)).collect();
                let change_bases: Vec<_> = implementations
                    .iter()
                    .map(|meas_impl| {
                        let p_pivot = meas_impl.measures().get_pauli(0);
                        // Expect Y ⊗ P
                        select_basis_change(Pauli::Y, p_pivot)
                    })
                    .collect();
                let block_basis = BlockBases(change_bases);
                let basis: Vec<Pauli> = ps
                    .into_iter()
                    // Drop the pivot Pauli
                    .flat_map(|p| <[Pauli; 12]>::from(p).into_iter().skip(1))
                    .collect();

                let ops = Operations(compile_measurement(&arch, &GROSS_TABLE, basis));
                println!("Compiled: {}", ops);

                let mut expected: Vec<Operation> = vec![];

                // pre-rotations
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    for rot in meas_impl.rotations() {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }

                expected.extend(prep().take(blocks).map(|op| block_basis.change_basis(op)));

                // measurements
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    expected.extend(
                        native_instructions(block_i, meas_impl.base_measurement()).into_iter(),
                    );
                }
                expected.extend(
                    ghz_meas(0, arch.data_blocks())
                        .into_iter()
                        .map(|op| block_basis.change_basis(op)),
                );
                expected.extend(unprep().take(blocks).map(|op| block_basis.change_basis(op)));
                // post-rotations
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    for rot in meas_impl.rotations() {
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
            let meas = find_random_native_measurement(&GROSS_TABLE, Pauli::X);

            let ps: [Pauli; 12] = meas.measures().into();
            let basis: Vec<Pauli> = ps[1..].to_vec();
            dbg!(&basis);

            let ops = Operations(compile_rotation(
                &arch,
                &GROSS_TABLE,
                basis,
                *CLIFF_ANGLE,
                ACCURACY,
            ));
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
                let ps: Vec<_> = random_nontrivial_paulistrings().take(blocks).collect();
                let implementations: Vec<_> = ps.iter().map(|p| GROSS_TABLE.min_data(*p)).collect();
                let block_bases: Vec<_> = implementations
                    .iter()
                    .enumerate()
                    .map(|(block_i, meas_impl)| {
                        let p_pivot = meas_impl.measures().get_pauli(0);
                        if block_i < blocks - 1 {
                            select_basis_change(Y, p_pivot)
                        } else {
                            select_basis_change(X, p_pivot)
                        }
                    })
                    .collect();
                let block_basis = BlockBases(block_bases);

                let basis: Vec<Pauli> = ps
                    .into_iter()
                    // Drop the pivot Pauli
                    .flat_map(|p| <[Pauli; 12]>::from(p).into_iter().skip(1))
                    .collect();

                let ops = Operations(compile_rotation(
                    &arch,
                    &GROSS_TABLE,
                    basis,
                    *CLIFF_ANGLE,
                    ACCURACY,
                ));
                println!("Compiled: {}", ops);

                let mut expected: Vec<Operation> = vec![];

                // pre-rotations
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    for rot in meas_impl.rotations() {
                        let operations = rotation_instructions(rot)
                            .into_iter()
                            .map(|instr| vec![(block_i, instr)]);
                        expected.extend(operations);
                    }
                }

                expected.extend(prep(blocks).map(|op| block_basis.change_basis(op)));

                // measurements
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    expected.extend(
                        native_instructions(block_i, meas_impl.base_measurement()).into_iter(),
                    );
                }

                let mut middle_ops = ghz_meas(0, arch.data_blocks());
                middle_ops.push(vec![(
                    blocks - 1,
                    TGate(TGateData::new(Pauli::X, false, false).unwrap()),
                )]);
                middle_ops.extend(unprep(blocks));
                expected.extend(
                    middle_ops
                        .into_iter()
                        .map(|op| block_basis.change_basis(op)),
                );

                // post-rotations
                for (block_i, meas_impl) in implementations.iter().enumerate() {
                    for rot in meas_impl.rotations() {
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

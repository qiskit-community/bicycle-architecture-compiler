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

/// Compile a native measurement
fn native_measurement_ops(native_measurement: &NativeMeasurement) -> Vec<Instruction> {
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

/// Compile a Pauli measurement to ISA instructions
pub fn compile_measurement(
    architecture: &PathArchitecture,
    mut basis: Vec<Pauli>,
) -> Vec<Operation> {
    let mut ops: Vec<Operation> = vec![];

    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();

    // Extend basis to a multiple of 11, excluding one qubit for dual pivot
    while (basis.len() + 1) % 11 != 0 {
        basis.push(Pauli::I);
    }
    // Insert identity rotation on the dual pivot
    basis.insert(basis.len() - 11, Pauli::I);
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
                native_measurement_ops(nat_measure)
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

                ops.push(vec![(block_i, Instruction::Measure(z1))]);
            }
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
                native_measurement_ops(nat_measure)
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

    let z1 = TwoBases::new(Pauli::Z, Pauli::I).unwrap();
    let x1 = TwoBases::new(Pauli::X, Pauli::I).unwrap();

    // Extend basis to a multiple of 11, excluding one qubit for dual pivot
    while (basis.len() + 1) % 11 != 0 {
        basis.push(Pauli::I);
    }
    // Insert identity rotation on the dual pivot
    basis.insert(basis.len() - 10, Pauli::I);
    assert!(basis.len() % 11 == 0);

    // Find implementation for each block
    let rotation_impls: Vec<_> = basis
        .chunks_exact(11)
        .enumerate()
        .map(|(block_i, paulis)| {
            // Only apply a controlled-Pauli if its non-trivial, except for the last block,
            // which needs to apply X⊗P(φ) in any case
            if block_i < n - 1 && paulis.iter().all(|p| *p == Pauli::I) {
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
                native_measurement_ops(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)]),
            )
        }
    }

    // Prepare GHZ state
    ops.extend(ghz_prep(architecture));

    // Apply small-angle rotation on block n
    // Conjugate with automorphism
    let (meas, _) = rotation_impls.last().unwrap().as_ref().unwrap();
    ops.push(vec![(n - 1, Instruction::Automorphism(meas.automorphism))]);
    // TODO: Propagate Cliffords
    ops.push(vec![(
        n - 1,
        Instruction::Rotation(RotationData::new(Pauli::X, angle).unwrap()),
    )]);
    // let (rotations, _) = small_angle::synthesize_angle_x(angle, 1e-6);
    // for rotation in rotations {
    //     match rotation {
    //         SingleRotation::Z { dagger } => ops.push(vec![(
    //             n - 1,
    //             BicycleISA::TGate(TGateData::new(Pauli::Z, false, dagger).unwrap()),
    //         )]),
    //         SingleRotation::X { dagger } => ops.push(vec![(
    //             n - 1,
    //             BicycleISA::TGate(TGateData::new(Pauli::X, false, dagger).unwrap()),
    //         )]),
    //     }
    // }
    ops.push(vec![(
        n - 1,
        Instruction::Automorphism(meas.automorphism.inv()),
    )]);

    // Apply native measurements on nontrivial blocks, except block 1
    for (block_i, (meas, _)) in rotation_impls
        .iter()
        .enumerate()
        // Skip last block
        .take(n - 1)
        .filter_map(|(i, opt)| opt.as_ref().map(|val| (i, val)))
    {
        for isa in meas.implementation().map(|isa| {
            isa.try_into()
                .expect("Should be able to convert native measurement instructions")
        }) {
            ops.push(vec![(block_i, isa)]);
        }
    }

    // Uncompute GHZ state by local measurements on all data blocks (even if they had trivial rotations)
    // The last block also uncomputes by Z measurement
    for (block_i, opt) in rotation_impls.iter().enumerate() {
        match opt {
            None => ops.push(vec![(block_i, Instruction::Measure(x1))]),
            Some(_) => ops.push(vec![(block_i, Instruction::Measure(z1))]),
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
                native_measurement_ops(nat_measure)
                    .into_iter()
                    .map(|op| vec![(block_i, op)])
                    .rev(),
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

    use crate::operation;

    use super::*;

    use bicycle_isa::{
        AutomorphismData,
        Pauli::{I, X, Y, Z},
    };

    #[test]
    fn measurement_impls_loading() {
        let p = PauliString(0b11);
        let impl1 = MEASUREMENT_IMPLS.implementation(p);
        let impl2 = MEASUREMENT_IMPLS.implementation(p);
        assert_eq!(impl1, impl2);
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
    fn compile_joint_measurement() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 2 };
        let basis = vec![
            Z, Z, Z, I, I, I, I, I, I, I, // 10 logical ops
            Z, //
        ];

        let ops = compile_measurement(&arch, basis);
        // let mut buf = String::new();
        // operation::fmt_operations(&ops, &mut buf)?;
        // println!("{}", buf);

        // One joint operation
        let joint_ops: Vec<_> = ops.iter().filter(|op| op.len() == 2).collect();
        assert_eq!(1, joint_ops.len());

        for op in ops {
            println!("{:?}", op);
        }
        Ok(())
    }

    // #[test]
    // fn find_native_gates() {
    //     for i in 1..4_u32.pow(11) {
    //         let x = (i << 1) | 1; // Set X_0
    //         let p: PauliString = PauliString(x);
    //         let (meas, rots) = MEASUREMENT_IMPLS.implementation(p);

    //         let x_bits = p.0 & ((1 << 12) - 1);
    //         let z_bits = (p.0 & !((1 << 12) - 1)) >> 12;
    //         let any_bits = x_bits | z_bits;

    //         if rots.len() == 1 && any_bits.count_ones() == 2 {
    //             println!("{}", p);
    //         }
    //     }
    // }

    #[test]
    fn compile_native_x_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 1 };
        // XIIXII... is a native measurement so we don't need to apply rotations
        let basis = vec![I, X];
        let ops = compile_rotation(&arch, basis, PI / 4.);

        // let p: PauliString = (&[X, I, I, X, I, I, I, I, I, I, I, I]).into();
        // dbg!(MEASUREMENT_IMPLS.implementation(p));

        // let mut buf = String::new();
        // operation::fmt_operations(&ops, &mut buf)?;
        // println!("{}", buf);

        assert_eq!(ops.len(), 5);

        let aut = AutomorphismData::new(2, 3);
        let expected = vec![
            vec![(0, Instruction::Measure(TwoBases::new(X, I).unwrap()))],
            vec![(0, Instruction::Automorphism(aut))],
            vec![(
                0,
                Instruction::Rotation(RotationData::new(X, PI / 4.).unwrap()),
            )],
            vec![(0, Instruction::Automorphism(aut.inv()))],
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
        ];

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_x_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 1 };
        // IIIIXIIIIIIX requires one rotation
        // I also leave out the dual pivot (in I)
        // The conjugating rotation is implemented by the native measurement IIIIYIIIIIXX,
        // so the state preparation is in Z then Y.
        let basis = vec![I, I, I, I, I, X, I, I, I, I];
        let rot = PI / 16.;
        let ops = compile_rotation(&arch, basis, rot);

        // let p: PauliString = (&[X, I, I, I, I, I, I, X, I, I, I, I]).into();
        // let meas_impl = MEASUREMENT_IMPLS.implementation(p);
        // dbg!(&meas_impl);
        // let rot_impl = meas_impl.1[0].measures();
        // println!("{}", rot_impl);

        // let mut buf = String::new();
        // operation::fmt_operations(&ops, &mut buf)?;
        // println!("{}", buf);

        // Auts are the same by coincidence
        let aut_meas = AutomorphismData::new(0, 4);
        let aut_rot = AutomorphismData::new(0, 4);

        let expected = vec![
            // Rotate block
            // State prep in Z, see above
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_rot))],
            vec![(0, Instruction::Measure(TwoBases::new(Y, Z).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_rot.inv()))],
            vec![(0, Instruction::Measure(TwoBases::new(Y, I).unwrap()))],
            // Apply non-clifford rotation
            vec![(0, Instruction::Measure(TwoBases::new(X, I).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_meas))],
            vec![(0, Instruction::Rotation(RotationData::new(X, rot).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_meas.inv()))],
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            // Unrotate block
            vec![(0, Instruction::Measure(TwoBases::new(Y, I).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_rot.inv()))],
            vec![(0, Instruction::Measure(TwoBases::new(Y, Z).unwrap()))],
            vec![(0, Instruction::Automorphism(aut_rot))],
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
        ];

        for (op0, op1) in expected.iter().zip(&ops) {
            assert_eq!(op0, op1);
        }

        assert_eq!(expected, ops);

        Ok(())
    }

    #[test]
    fn compile_native_interblock_rotation() -> Result<(), Box<dyn Error>> {
        let arch = PathArchitecture { data_blocks: 2 };

        // IYYYYYXXIIIX is native
        let mut basis = vec![I, I, I, X, X, Y, Y, Y, Y, Y, I];
        basis.append(&mut vec![I; 10]);

        let aut = AutomorphismData::new(4, 1);
        let no_aut = AutomorphismData::new(0, 0);
        let rot = -PI / 32.;

        let ops = compile_rotation(&arch, basis, rot);

        let mut buf = String::new();
        operation::fmt_operations(&ops, &mut buf)?;
        println!("{}", buf);

        let expected = vec![
            // Prepare GHZ
            vec![(0, Instruction::Measure(TwoBases::new(X, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![
                (0, Instruction::JointMeasure(TwoBases::new(Z, I).unwrap())),
                (1, Instruction::JointMeasure(TwoBases::new(Z, I).unwrap())),
            ],
            // Native measure block 1
            vec![(1, Instruction::Automorphism(no_aut))],
            vec![(1, Instruction::Rotation(RotationData::new(X, rot).unwrap()))],
            vec![(1, Instruction::Automorphism(no_aut))],
            // Native measure block 0
            vec![(0, Instruction::Automorphism(aut))],
            vec![(0, Instruction::Measure(TwoBases::new(Y, Z).unwrap()))],
            vec![(0, Instruction::Automorphism(aut.inv()))],
            // Uncompute GHZ
            vec![(0, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
            vec![(1, Instruction::Measure(TwoBases::new(Z, I).unwrap()))],
        ];

        for (op1, op2) in expected.iter().zip(&ops) {
            assert_eq!(op1, op2);
        }

        assert_eq!(ops, expected);

        Ok(())
    }
}

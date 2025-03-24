use bicycle_isa::AutomorphismData;

use crate::operation::{Instruction, Operation, RotationData};

/// Try to merge two Operations
fn merge_operations(op0: Operation, op1: Operation) -> (Operation, Option<Operation>) {
    // Try to merge the operations, but store the originals to revert if needed
    let merged = op0
        .iter()
        .copied()
        .zip(op1.iter().copied())
        .map(|((block0, instr0), (block1, instr1))| {
            if block0 == block1 {
                let (new_instr0, new_instr1) = merge_instructions(instr0, instr1);
                ((block0, new_instr0), new_instr1.map(|i| (block1, i)))
            } else {
                ((block0, instr0), Some((block1, instr1)))
            }
        })
        .collect::<Vec<_>>();

    // If all operations were element-wise successfully merged, use the new results
    if merged.iter().all(|m| m.1.is_none()) {
        // Flatten the Nones
        (merged.into_iter().map(|m| m.0).collect(), None)
    } else {
        (op0, Some(op1))
    }
}

/// Try to merge two instructions
fn merge_instructions(
    instr0: Instruction,
    instr1: Instruction,
) -> (Instruction, Option<Instruction>) {
    match (instr0, instr1) {
        (Instruction::Automorphism(aut0), Instruction::Automorphism(aut1)) => {
            (Instruction::Automorphism(aut0 * aut1), None)
        }
        (Instruction::Measure(_), Instruction::Measure(_))
        | (Instruction::JointMeasure(_), Instruction::JointMeasure(_)) => {
            if instr0 == instr1 {
                (instr0, None)
            } else {
                (instr0, Some(instr1))
            }
        }
        (Instruction::Rotation(rot0), Instruction::Rotation(rot1)) => {
            let (new_rot0, new_rot1) = rot0.partial_mul(rot1);
            (
                Instruction::Rotation(new_rot0),
                new_rot1.map(Instruction::Rotation),
            )
        }
        _ => (instr0, Some(instr1)),
    }
}

/// Remove measurements that are repeated on the same block
/// Note: This considers only single-block measurements for simplicity
pub fn remove_duplicate_measurements(
    ops: impl IntoIterator<Item = Operation>,
) -> impl Iterator<Item = Operation> {
    remove_duplicate_measurement_chunked(ops.into_iter().map(|op| vec![op])).flatten()
}

/// Remove measurements that are repeated but respect the chunk boundaries as they are given
pub fn remove_duplicate_measurement_chunked(
    chunked_ops: impl IntoIterator<Item = impl IntoIterator<Item = Operation>>,
) -> impl Iterator<Item = Vec<Operation>> {
    let mut history: Vec<Option<Instruction>> = Vec::new();

    chunked_ops.into_iter().map(move |ops_chunk| {
        ops_chunk
            .into_iter()
            .filter(|ops_list| {
                for (i, instr) in ops_list {
                    history.resize_with(history.len().max(i + 1), Default::default);

                    if let Instruction::Measure(_) = instr {
                        if history[*i] == Some(*instr) {
                            return false;
                        }
                    }
                    // Copy seen instructions into history
                    // Cannot reference because that would make instructions immutable
                    history[*i] = Some(*instr);
                }
                true
            })
            .collect()
    })
}

pub trait Identity {
    fn is_identity(&self) -> bool;
}

impl Identity for RotationData {
    fn is_identity(&self) -> bool {
        // Close enough to identity?
        self.angle.abs() < 1e-10
    }
}

impl Identity for AutomorphismData {
    fn is_identity(&self) -> bool {
        self.get_x() == 0 && self.get_y() == 0
    }
}

impl Identity for Instruction {
    fn is_identity(&self) -> bool {
        match self {
            Instruction::Rotation(rot) => rot.is_identity(),
            Instruction::Automorphism(aut) => aut.is_identity(),
            Instruction::Measure(_) | Instruction::JointMeasure(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use bicycle_isa::TwoBases;

    use super::*;
    use bicycle_isa::Pauli::{I, X, Y, Z};

    #[test]
    fn remove_duplicate_meas() {
        let meas = Instruction::Measure(TwoBases::new(X, Z).unwrap());
        let ops = vec![vec![(3, meas)], vec![(3, meas)]];

        let res: Vec<_> = remove_duplicate_measurements(ops).collect();
        let expected = vec![vec![(3, meas)]];
        assert_eq!(expected, res);
    }

    #[test]
    fn remove_duplicat_meas2() {
        let meas = Instruction::Measure(TwoBases::new(X, Z).unwrap());
        let ops = vec![vec![(3, meas)], vec![(0, meas)], vec![(3, meas)]];

        let res: Vec<_> = remove_duplicate_measurements(ops).collect();
        let expected = vec![vec![(3, meas)], vec![(0, meas)]];
        assert_eq!(expected, res);
    }
}

use bicycle_isa::AutomorphismData;

use crate::operation::{Instruction, Operation, RotationData};

/// Simplify single-block operations
/// Note: We do not simplify Operations acting on more than one block due to complexity of data structures,
/// but it could be done
struct DuplicateRemovalIter<I> {
    iter: I,
    // The history stores at most 2 previous instructions for each block
    history: Vec<Vec<Instruction>>,
    new_op: Option<Operation>,
}

impl<I> DuplicateRemovalIter<I> {
    pub fn new(iter: I) -> Self {
        DuplicateRemovalIter {
            iter,
            history: Default::default(),
            new_op: Default::default(),
        }
    }

    /// Allocate more entries in the history when the number of blocks gets larger
    fn resize(&mut self, new_len: usize) {
        self.history.resize(new_len, Vec::with_capacity(2));
    }
}

impl<I: Iterator<Item = Operation>> Iterator for DuplicateRemovalIter<I> {
    type Item = Vec<Operation>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut new_op = None;
        loop {
            new_op = self.iter.next();

            if new_op.is_none() {
                break;
            }

            let ops = new_op.unwrap();
            let max_len = ops.iter().map(|op| op.0).max().unwrap();
            self.resize(max_len);

            if ops.len() > 1 {
                break;
            }

            let (block_i, instr) = ops.pop().unwrap();
            // If there is a last instruction, then try merge the new instruction with that one
            let (first_instr, sec_instr) = self.history[block_i]
                .pop()
                .map(|hist_instr| merge_instructions(hist_instr, instr))
                // If there isn't, then just insert the new one
                .unwrap_or_else(|| (instr, None));
            self.history[block_i].push(first_instr);
            // Simplify history
            self.history[block_i] = self.history[block_i]
                .into_iter()
                .filter(|instr| !instr.is_identity())
                .collect();

            if let Some(instr2) = sec_instr {
                match instr2 {
                    Instruction::Automorphism(_) => self.history[block_i].push(instr2),
                    _ => break,
                }
                assert!(self.history[block_i].len() <= 2, "History too long");
            }
        }

        // Need to flush history if new_op stored
        if let Some(op) = self.new_op.take() {
            // Take out all elements from the history on the blocks the new_op acts on
            let history: Vec<Operation> = op
                .iter()
                .map(|hist_op| hist_op.0)
                .flat_map(|i| self.history[i].into_iter().map(|instr| (i, instr)))
                .map(|o| vec![o])
                .collect();

            // Emit joint operations immediately
            if op.len() > 1 {
                history.push(op);
            } else {
                let (block_i, instr) = op.pop().unwrap();
                self.history[block_i].push(instr);
            }
            Some(history)
        } else {
            if let Some(new_op) = self.iter.next() {
            } else {
                // Flush history
                for i in 0..self.history.len() {
                    if let Some(instr) = self.history[i].pop() {
                        return Some(vec![(i, instr)]);
                    }
                }
            }
            None
        }
    }
}

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

pub fn remove_duplicates(
    ops: impl IntoIterator<Item = Operation>,
) -> impl Iterator<Item = Operation> {
    ops.into_iter().scan(
        Vec::<(Option<Instruction>, AutomorphismData)>::new(),
        |prev, vec_ops| {
            for (block_i, op) in vec_ops {
                // Allocate vector if too small
                if block_i >= prev.len() {
                    prev.resize_with(block_i, Default::default);
                }

                match op {
                    // Merge subsequent automorphisms
                    Instruction::Automorphism(aut) => prev[block_i].1 *= aut,
                    Instruction::Measure(bases) => {
                        if prev[block_i] == (Some(op), Default::default()) {
                            // Discard this measurement
                            continue;
                        } else {
                        }
                    }
                    _ => {
                        println!("yes");
                    }
                }
            }

            None
        },
    )
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

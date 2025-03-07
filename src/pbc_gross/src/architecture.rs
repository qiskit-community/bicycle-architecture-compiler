use crate::operation::Operation;

/// Consists of blocks plus one magic state factory at the end of the path
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PathArchitecture {
    pub data_blocks: usize,
}

impl PathArchitecture {
    pub fn data_blocks(&self) -> usize {
        self.data_blocks
    }

    // Check if a qubit is valid within a block
    fn valid_qubit(qubit: u8) -> bool {
        qubit <= 11
    }

    pub fn validate_operation(&self, op: &Operation) -> bool {
        // Check that operations act on successive blocks
        if op.len() == 1 {
            true
        } else {
            op[0].0.abs_diff(op[1].0) == 1
        }
    }
}

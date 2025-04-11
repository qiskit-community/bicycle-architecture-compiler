use crate::operation::Operation;

/// Consists of blocks plus one magic state factory at the end of the path
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PathArchitecture {
    pub data_blocks: usize,
}

impl PathArchitecture {
    pub fn for_qubits(qubits: usize) -> Self {
        let data_blocks = (qubits + 1).div_ceil(11);

        Self { data_blocks }
    }

    pub fn data_blocks(&self) -> usize {
        self.data_blocks
    }

    pub fn qubits(&self) -> usize {
        self.data_blocks * 11 - 1
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

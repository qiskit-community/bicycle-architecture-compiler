// Copyright contributors to the Bicycle Architecture Compiler project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::operation::Operation;

/// Consists of blocks plus one magic state factory at the end of the path
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PathArchitecture {
    pub data_blocks: usize,
}

impl PathArchitecture {
    pub fn for_qubits(qubits: usize) -> Self {
        let data_blocks = qubits.div_ceil(11);

        Self { data_blocks }
    }

    pub fn data_blocks(&self) -> usize {
        self.data_blocks
    }

    pub fn qubits(&self) -> usize {
        self.data_blocks * 11
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

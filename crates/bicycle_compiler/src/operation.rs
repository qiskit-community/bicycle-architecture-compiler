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

use std::fmt::Display;

use bicycle_common::BicycleISA;
use serde::{Deserialize, Serialize};

// Could expand this into single block and joint block operations,
// but I think, effectively, we want to just be able to verify if an operation fits the architecture.
pub type Operation = Vec<(usize, BicycleISA)>;

/// Pretty print an Operation
pub fn fmt_operation(op: &Operation, f: &mut dyn std::fmt::Write) -> std::fmt::Result {
    let mut s = String::from("[");
    s += &op
        .iter()
        .map(|(i, isa)| format!("({i},{isa})"))
        .collect::<Vec<_>>()
        .join(",");
    s += "]";
    write!(f, "{}", s)
}

/// Wrapper for a vector of operations for pretty printing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operations(pub Vec<Operation>);

impl Display for Operations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[")?;
        for (i, op) in self.0.iter().enumerate() {
            write!(f, "\t{i}:")?;
            fmt_operation(op, f)?;
            writeln!(f)?;
        }

        write!(f, "]")
    }
}

use std::fmt::Display;

use bicycle_isa::BicycleISA;
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

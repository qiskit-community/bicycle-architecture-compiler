use bicycle_isa::Pauli;

use crate::{architecture::PathArchitecture, compile, operation::Operation};

/// A PBC program operation
/// Consider replacing the angle with a rational to improve precision.
/// But f64 has 52-bit mantissa, so seems sufficient for all practical purposes.
#[derive(Debug, Clone, PartialEq)]
pub enum PbcOperation {
    Measurement {
        basis: Vec<Pauli>,
        flip_result: bool,
    },
    Rotation {
        basis: Vec<Pauli>,
        angle: f64,
    },
}

impl PbcOperation {
    pub fn compile(&self, architecture: &PathArchitecture) -> Vec<Operation> {
        match self {
            // TODO: use flip_result to flip the sign of measurements
            PbcOperation::Measurement { basis, .. } => {
                compile::compile_measurement(architecture, basis.to_vec())
            }
            PbcOperation::Rotation { basis, angle } => {
                compile::compile_rotation(architecture, basis.to_vec(), *angle)
            }
        }
    }
}

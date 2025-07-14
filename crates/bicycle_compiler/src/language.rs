// Copyright contributors to the Bicycle Architecture Compiler project

use std::fmt::Display;

use bicycle_common::Pauli;
use fixed::types::I32F96;

use bicycle_cliffords::CompleteMeasurementTable;
use serde::{Deserialize, Serialize};

use crate::{architecture::PathArchitecture, compile, operation::Operation};

pub type AnglePrecision = I32F96;

/// A PBC program operation
/// Consider replacing the angle with a rational to improve precision.
/// But f64 has 52-bit mantissa, so seems sufficient for all practical purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PbcOperation {
    Measurement {
        basis: Vec<Pauli>,
        flip_result: bool,
    },
    Rotation {
        basis: Vec<Pauli>,
        angle: AnglePrecision,
    },
}

impl PbcOperation {
    pub fn rotation(basis: Vec<Pauli>, angle: f64) -> Self {
        Self::Rotation {
            basis,
            angle: AnglePrecision::from_num(angle),
        }
    }
    pub fn compile(
        &self,
        architecture: &PathArchitecture,
        measurement_table: &CompleteMeasurementTable,
        accuracy: AnglePrecision,
    ) -> Vec<Operation> {
        match self {
            // TODO: use flip_result to flip the sign of measurements
            PbcOperation::Measurement { basis, .. } => {
                compile::compile_measurement(architecture, measurement_table, basis.to_vec())
            }
            PbcOperation::Rotation { basis, angle } => compile::compile_rotation(
                architecture,
                measurement_table,
                basis.to_vec(),
                *angle,
                accuracy,
            ),
        }
    }

    pub fn basis(&self) -> &Vec<Pauli> {
        match self {
            PbcOperation::Measurement {
                basis,
                flip_result: _,
            } => basis,
            PbcOperation::Rotation { basis, angle: _ } => basis,
        }
    }
}

impl Display for PbcOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PbcOperation::Measurement { basis, flip_result } => {
                write!(
                    f,
                    "Measurement([{}],",
                    basis
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )?;
                if *flip_result {
                    write!(f, "flipped)")
                } else {
                    write!(f, "regular)")
                }
            }
            PbcOperation::Rotation { basis, angle } => {
                write!(
                    f,
                    "Rotation([{}],{})",
                    basis
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                    angle
                )
            }
        }
    }
}

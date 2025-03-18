use std::fmt::Display;

use bicycle_isa::{AutomorphismData, BicycleISA, Pauli, TwoBases};
use gross_code_cliffords::native_measurement::NativeMeasurement;
use serde::{Deserialize, Serialize};

/// Single-qubit rotation on the pivot
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotationData {
    pub basis: [Pauli; 11],
    pub angle: f64,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeRotation {
    pub native_measurement: NativeMeasurement,
    pub angle: f64,
}

/// A simplified instruction set (compared to the Bicycle ISA) that is output from the PBC compiler
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Automorphism generators with x in {0,...,5} and y in {0,1,2} and x+y>0
    Automorphism(AutomorphismData),

    // Measurements
    // Measure qubits 1 and 7 with specified Paulis, one of which must not be identity
    Measure(TwoBases),
    // Measure qubits 1 and 7 in a joint operation with another block, one of which must not be identity.
    JointMeasure(TwoBases),

    // Magic
    Rotation(RotationData), // Apply exp(iπ/8 P), where P is a list of 11 Paulis
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Automorphism(data) => write!(f, "aut({},{})", data.get_x(), data.get_y()),
            Instruction::Measure(bases) => {
                write!(f, "meas({},{})", bases.get_basis_1(), bases.get_basis_7())
            }
            Instruction::JointMeasure(bases) => {
                write!(f, "jMeas({},{})", bases.get_basis_1(), bases.get_basis_7())
            }
            Instruction::Rotation(rot) => {
                write!(
                    f,
                    "rot([{}],{:.4})",
                    rot.basis
                        .iter()
                        .map(|p| format!("{}", p))
                        .collect::<Vec<_>>()
                        .join(","),
                    rot.angle
                )
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InstructionConversionError {
    PrimedGate,
    InvalidISA,
}

impl Display for InstructionConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PrimedGate => write!(f, "Cannot convert T gate on dual pivot."),
            Self::InvalidISA => write!(
                f,
                "This BicycleISA instruction does not have a representation as an Instruction"
            ),
        }
    }
}

impl std::error::Error for InstructionConversionError {}

impl TryFrom<BicycleISA> for Instruction {
    type Error = InstructionConversionError;

    /// Try to convert a BicycleISA instruction to an Instruction
    /// Note that a TGate cannot be converted since it acts on the pivot implicitly,
    /// whereas an Instruction::Rotation acts only on data.
    fn try_from(value: BicycleISA) -> Result<Self, Self::Error> {
        match value {
            BicycleISA::Automorphism(a) => Ok(Instruction::Automorphism(a)),
            BicycleISA::Measure(b) => Ok(Instruction::Measure(b)),
            BicycleISA::JointMeasure(b) => Ok(Instruction::JointMeasure(b)),
            _ => Err(InstructionConversionError::InvalidISA),
        }
    }
}

// Could expand this into single block and joint block operations,
// but I think, effectively, we want to just be able to verify if an operation fits the architecture.
pub type Operation = Vec<(usize, Instruction)>;

/// Pretty print an Operation
pub fn fmt_operation(op: &Operation, f: &mut dyn std::fmt::Write) -> std::fmt::Result {
    let mut s = String::from("[");
    for (i, isa) in op {
        s += &format!("({}, {}),", i, isa);
    }
    s += "]";
    write!(f, "{}", s)
}

/// Wrapper for a vector of operations for pretty printing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operations(pub Vec<Operation>);

impl Display for Operations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "[")?;
        for op in &self.0 {
            write!(f, "\t")?;
            fmt_operation(op, f)?;
            writeln!(f)?;
        }

        write!(f, "]")
    }
}

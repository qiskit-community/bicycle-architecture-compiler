extern crate nalgebra as na;
use na::Matrix6;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Pauli {
    I,
    X,
    Z,
    Y,
}

impl TryFrom<&char> for Pauli {
    type Error = String;

    fn try_from(value: &char) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase() {
            'i' => Ok(Pauli::I),
            'x' => Ok(Pauli::X),
            'z' => Ok(Pauli::Z),
            'y' => Ok(Pauli::Y),
            c => Err(format!("Cannot convert {} to Pauli", c)),
        }
    }
}

/// Specify what automorphism to perform.
/// Since each automorphism has order 6, the x and y parameters wrapped to be in {0,1,...,5}.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct AutomorphismData {
    x: u8,
    y: u8,
}

impl AutomorphismData {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x: x % 6, y: y % 6 }
    }

    pub fn get_x(&self) -> u8 {
        self.x
    }

    pub fn get_y(&self) -> u8 {
        self.y
    }

    /// Compute the inverse automorphism
    pub fn inv(&self) -> Self {
        AutomorphismData::new(6 - self.x, 6 - self.y)
    }

    /// Generate the parity map associated with this automorphism on the Gross code
    pub fn parity_map_gross(&self) -> Matrix6<u32> {
        let mx_array: [u32; 36] = [
            1, 1, 0, 1, 1, 1, //
            0, 0, 0, 0, 1, 0, //
            0, 0, 0, 0, 1, 1, //
            1, 1, 1, 1, 0, 1, //
            0, 1, 0, 0, 1, 0, //
            1, 1, 0, 0, 1, 0,
        ];
        let my_array: [u32; 36] = [
            1, 0, 1, 0, 1, 0, //
            1, 0, 1, 0, 0, 1, //
            1, 1, 0, 0, 1, 0, //
            0, 0, 0, 0, 1, 0, //
            0, 0, 0, 1, 1, 0, //
            1, 0, 0, 0, 0, 1,
        ];

        let mx = Matrix6::from_row_slice(&mx_array);
        let my = Matrix6::from_row_slice(&my_array);

        Matrix6::pow(&mx, self.x.into()) * Matrix6::pow(&my, self.y.into()).map(|v| v % 2)
    }

    /// Generate the parity map associated with this automorphism on the Disgusting code
    pub fn parity_map_disgusting(&self) -> Matrix6<u32> {
        // Disgusting code
        let mx_array: [u32; 36] = [
            0, 0, 1, 0, 1, 0, //
            0, 0, 0, 0, 1, 1, //
            0, 0, 1, 1, 1, 0, //
            1, 0, 1, 1, 0, 0, //
            0, 1, 1, 0, 0, 0, //
            0, 0, 1, 0, 1, 1, //
        ];

        let my_array: [u32; 36] = [
            1, 0, 1, 1, 1, 0, //
            1, 0, 1, 0, 0, 0, //
            0, 1, 0, 1, 1, 0, //
            0, 0, 0, 1, 1, 1, //
            1, 0, 0, 1, 1, 0, //
            1, 0, 0, 0, 1, 0, //
        ];
        let mx = Matrix6::from_row_slice(&mx_array);
        let my = Matrix6::from_row_slice(&my_array);

        Matrix6::pow(&mx, self.x.into()) * Matrix6::pow(&my, self.y.into()).map(|v| v % 2)
    }
}

/// Measure two qubits independently in the same basis, which must be X or Z
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ParallelMeasureData {
    p: Pauli,
}

impl ParallelMeasureData {
    pub fn new(p: Pauli) -> Option<Self> {
        match p {
            Pauli::X | Pauli::Z => Some(ParallelMeasureData { p }),
            _ => None,
        }
    }

    pub fn get_basis(&self) -> Pauli {
        self.p
    }
}

/// Measure in two bases, one of which must not be identity
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TwoBases {
    p1: Pauli,
    p7: Pauli,
}

impl TwoBases {
    pub fn new(p1: Pauli, p7: Pauli) -> Option<Self> {
        match (p1, p7) {
            (Pauli::I, Pauli::I) => None,
            _ => Some(TwoBases { p1, p7 }),
        }
    }

    pub fn get_basis_1(&self) -> Pauli {
        self.p1
    }

    pub fn get_basis_7(&self) -> Pauli {
        self.p7
    }
}

/// Store what kind of T gate is being implemented.  Must be in X or Z basis.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TGateData {
    basis: Pauli,
    pub primed: bool,
}

impl TGateData {
    pub fn new(basis: Pauli, primed: bool) -> Option<Self> {
        match basis {
            Pauli::X | Pauli::Z => Some(TGateData { basis, primed }),
            _ => None,
        }
    }

    pub fn get_basis(&self) -> Pauli {
        self.basis
    }
}

// See also docs/compiler_worshop_isa.pdf for an explanation of these instructions
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BicycleISA {
    SyndromeCycle, // Syndrome cycle
    CSSInitZero,   // Initialize the block in |0>^12
    CSSInitPlus,   // Initialize the block in |+>^12
    DestructiveZ,  // Measure all qubits in Z and infer logical Z measurements
    DestructiveX,  // Measure all qubits in X and infer logical X measurements
    // Automorphism generators with x in {0,...,5} and y in {0,1,2} and x+y>0
    Automorphism(AutomorphismData),

    // Measurements
    // Measure qubits 1 and 7 with specified Paulis, one of which must not be identity
    Measure(TwoBases),
    // Measure qubits 1 and 7 in a joint operation with another block, one of which must not be identity.
    JointMeasure(TwoBases),
    // Independently measure qubit 1 and qubit 7 in the X or the Z basis
    ParallelMeasure(ParallelMeasureData),

    // Entanglement between two blocks
    JointBellInit, // Initialize two codes into 12 Bell states via rotating donut method
    JointTransversalCX, // Transversal CX using rotating donut

    // Magic
    InitT,            // Initialization into 8 physical-noise |T> states
    TGate(TGateData), // Apply exp(iπ/8 P), with P in {X, X', Z, Z'}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_bases() {
        assert_eq!(None, TwoBases::new(Pauli::I, Pauli::I));
        assert_eq!(
            Some(TwoBases {
                p1: Pauli::X,
                p7: Pauli::Z
            }),
            TwoBases::new(Pauli::X, Pauli::Z)
        );
    }
}

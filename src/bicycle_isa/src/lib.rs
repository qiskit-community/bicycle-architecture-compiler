use std::{
    fmt::Display,
    ops::{Mul, MulAssign},
};

use rand::distr::{Distribution, StandardUniform};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum Pauli {
    #[default]
    I,
    X,
    Z,
    Y,
}

impl Pauli {
    /// Give the Paulis that anticommute with this Pauli.
    pub fn anticommuting(&self) -> Option<(Self, Self)> {
        match self {
            Self::I => None,
            Self::X => Some((Self::Z, Self::Y)),
            Self::Z => Some((Self::X, Self::Y)),
            Self::Y => Some((Self::X, Self::Z)),
        }
    }
}

impl Display for Pauli {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Distribution<Pauli> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Pauli {
        let i = rng.random_range(0..=3);
        match i {
            0 => Pauli::I,
            1 => Pauli::Z,
            2 => Pauli::X,
            3 => Pauli::Y,
            _ => unreachable!("RNG number out of range"),
        }
    }
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

impl TryFrom<usize> for Pauli {
    type Error = String;

    /// Convert a integer in [0,3] to a Pauli
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Pauli::I),
            1 => Ok(Pauli::X),
            2 => Ok(Pauli::Z),
            3 => Ok(Pauli::Y),
            _ => Err(format!("Cannot  convert {} to Pauli", value)),
        }
    }
}

/// The group of shift automorphisms is defined in Yod+25 Sec. A.2 ("Tour de gross")
///
/// This group is isomorphic to Z6 x Z6, the direct product of the cyclic group of order six
/// with itself. `AutomorphismData`, together with methods implemented for it, is an
/// implementation of Z6 x Z6.  The exception is the method, `nr_generators`, which is
/// particular to the BB architecture. This method returns the number of generators required
/// to implement an element of the group. But we are interested in a particular generating
/// set, rather than, say, a minimal generating set. The generating set defined in Yod+15 is
/// chosen because its elements are the easiest to implement as circuits. Thus,
/// `nr_generators` gives an indication of resources required to implement a particular
/// shift automorphism as a product of elementary elements.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
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

    /// Calculate the number of automorphism generators (defined in Yod+25) necessary
    /// to implement this automorphism group element.
    pub fn nr_generators(&self) -> u64 {
        match (self.x, self.y) {
            (0, 0) => 0,
            (3, 3) => 1,
            (3, _) | (_, 3) => 2,
            _ => 1,
        }
    }

    /// Compute the inverse automorphism
    pub fn inv(&self) -> Self {
        AutomorphismData::new(6 - self.x, 6 - self.y)
    }
}

impl Mul for AutomorphismData {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl MulAssign for AutomorphismData {
    fn mul_assign(&mut self, rhs: Self) {
        *self = Self::new(self.x + rhs.x, self.y + rhs.y);
    }
}

impl Distribution<AutomorphismData> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> AutomorphismData {
        let x = rng.random_range(0..=5);
        let y = rng.random_range(0..=5);
        AutomorphismData::new(x, y)
    }
}

/// Measure two qubits independently in the same basis, which must be X or Z
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
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

impl Distribution<TwoBases> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> TwoBases {
        let mut out = None;
        while out.is_none() {
            let p1 = StandardUniform.sample(rng);
            let p7 = StandardUniform.sample(rng);
            out = TwoBases::new(p1, p7);
        }
        out.unwrap()
    }
}

/// Store what kind of T gate is being implemented.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct TGateData {
    basis: Pauli,
    pub primed: bool,  // Applied to the primed pivot (qubit 7)
    pub adjoint: bool, // Take the dagger; Rotation by -π/4
}

impl TGateData {
    pub fn new(basis: Pauli, primed: bool, adjoint: bool) -> Option<Self> {
        match basis {
            Pauli::I => None,
            Pauli::X | Pauli::Z | Pauli::Y => Some(TGateData {
                basis,
                primed,
                adjoint,
            }),
        }
    }

    pub fn get_basis(&self) -> Pauli {
        self.basis
    }
}

impl Distribution<TGateData> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> TGateData {
        let p = if rng.random() { Pauli::X } else { Pauli::Z };
        TGateData::new(p, rng.random(), rng.random()).unwrap()
    }
}

// See also docs/compiler_worshop_isa.pdf for an explanation of these instructions
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
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

impl Display for BicycleISA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BicycleISA::SyndromeCycle => write!(f, "sc"),
            BicycleISA::CSSInitZero => write!(f, "init0"),
            BicycleISA::CSSInitPlus => write!(f, "init+"),
            BicycleISA::DestructiveZ => write!(f, "measZ"),
            BicycleISA::DestructiveX => write!(f, "measX"),
            BicycleISA::Automorphism(data) => write!(f, "aut({},{})", data.get_x(), data.get_y()),
            BicycleISA::Measure(bases) => {
                write!(f, "meas({},{})", bases.get_basis_1(), bases.get_basis_7())
            }
            BicycleISA::JointMeasure(bases) => {
                write!(f, "jMeas({},{})", bases.get_basis_1(), bases.get_basis_7())
            }
            BicycleISA::ParallelMeasure(basis) => write!(f, "pMeas({})", basis.get_basis()),
            BicycleISA::JointBellInit => write!(f, "jBell"),
            BicycleISA::JointTransversalCX => write!(f, "jCnot"),
            BicycleISA::InitT => write!(f, "initT"),
            BicycleISA::TGate(basis) => {
                let prime = if basis.primed { "'" } else { "" };
                let dagger = if basis.adjoint { "†" } else { "" };
                write!(f, "T({}", basis.get_basis())?;
                write!(f, "{}", prime)?;
                write!(f, "{}", dagger)?;
                write!(f, ")")
            }
        }
    }
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

    #[test]
    fn automorphism_generators() {
        assert_eq!(0, AutomorphismData::new(0, 0).nr_generators());
        assert_eq!(1, AutomorphismData::new(3, 3).nr_generators());
        assert_eq!(1, AutomorphismData::new(1, 8).nr_generators());
        assert_eq!(2, AutomorphismData::new(3, 5).nr_generators());
        assert_eq!(2, AutomorphismData::new(8, 3).nr_generators());
    }
}

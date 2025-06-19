// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use bicycle_isa::{BicycleISA, Pauli, TGateData, TwoBases};

/// An object that permutes the non-trivial Pauli basis of the pivot qubit
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BasisChanger {
    x: Pauli,
    y: Pauli,
    z: Pauli,
}

impl BasisChanger {
    pub fn new(x: Pauli, y: Pauli, z: Pauli) -> Result<Self, &'static str> {
        if x == y || y == z || z == x {
            return Err("Basis must be unique");
        }
        Ok(Self { x, y, z })
    }

    pub fn change_isa(&self, instr: BicycleISA) -> BicycleISA {
        match instr {
            BicycleISA::Measure(bases) => BicycleISA::Measure(self.two_bases(bases)),
            BicycleISA::JointMeasure(bases) => BicycleISA::JointMeasure(self.two_bases(bases)),
            BicycleISA::TGate(data) => BicycleISA::TGate(
                TGateData::new(
                    self.change_pauli(data.get_basis()),
                    data.primed,
                    data.adjoint,
                )
                .unwrap(),
            ),
            BicycleISA::Automorphism(_) => instr,
            _ => unimplemented!(),
        }
    }

    pub fn two_bases(&self, bases: TwoBases) -> TwoBases {
        TwoBases::new(self.change_pauli(bases.get_basis_1()), bases.get_basis_7()).unwrap()
    }

    pub fn change_pauli(&self, p: Pauli) -> Pauli {
        match p {
            Pauli::I => Pauli::I,
            Pauli::Z => self.z,
            Pauli::X => self.x,
            Pauli::Y => self.y,
        }
    }
}

impl Default for BasisChanger {
    fn default() -> Self {
        Self {
            x: Pauli::X,
            y: Pauli::Y,
            z: Pauli::Z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bicycle_isa::AutomorphismData;
    use Pauli::{X, Y, Z};

    #[test]
    fn test_change_pauli() {
        let changer = BasisChanger::new(Y, Z, X).unwrap();

        assert_eq!(Z, changer.change_pauli(Y));
        assert_eq!(Y, changer.change_pauli(X));
    }

    #[test]
    fn test_change_instr() {
        let changer = BasisChanger::new(Y, Z, X).unwrap();

        assert_eq!(
            BicycleISA::Measure(TwoBases::new(Y, Z).unwrap()),
            changer.change_isa(BicycleISA::Measure(TwoBases::new(X, Z).unwrap()))
        );

        assert_eq!(
            BicycleISA::JointMeasure(TwoBases::new(Z, X).unwrap()),
            changer.change_isa(BicycleISA::JointMeasure(TwoBases::new(Y, X).unwrap()))
        );
    }

    #[test]
    fn test_invariant() {
        let changer = BasisChanger::new(Z, X, Y).unwrap();

        for x in 0..6 {
            for y in 0..6 {
                let aut = AutomorphismData::new(x, y);
                let isa = BicycleISA::Automorphism(aut);
                assert_eq!(isa, changer.change_isa(isa));
            }
        }
    }
}

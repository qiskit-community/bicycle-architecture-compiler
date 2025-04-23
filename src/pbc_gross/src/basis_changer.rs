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
            BicycleISA::Measure(bases) | BicycleISA::JointMeasure(bases) => {
                BicycleISA::JointMeasure(self.two_bases(bases))
            }
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

    fn two_bases(&self, bases: TwoBases) -> TwoBases {
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

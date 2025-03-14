use std::fmt::Display;

use crate::pauli_rotation::PauliString;
use nalgebra::{stack, SMatrix, Vector6};

use bicycle_isa::{AutomorphismData, BicycleISA, Pauli, TwoBases};
use serde::{Deserialize, Serialize};

/// A measurement that can be performed on the code by conjugating one base measurement with automorphisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeMeasurement {
    pub logical: TwoBases,
    pub automorphism: AutomorphismData,
}

impl NativeMeasurement {
    /// The PauliString this NativeMeasurement measures.
    #[allow(clippy::toplevel_ref_arg)]
    pub fn measures(&self) -> PauliString {
        let one = Vector6::identity();
        let zero = Vector6::zeros();

        let (x1, z1) = match self.logical.get_basis_1() {
            Pauli::I => (zero, zero),
            Pauli::X => (one, zero),
            Pauli::Z => (zero, one),
            Pauli::Y => (one, one),
        };

        let (x7, z7) = match self.logical.get_basis_7() {
            Pauli::I => (zero, zero),
            Pauli::X => (one, zero),
            Pauli::Z => (zero, one),
            Pauli::Y => (one, one),
        };

        let vec = stack![x1; x7; z1; z7];

        let aut = self.automorphism.parity_map_gross();
        let inv = self.automorphism.inv().parity_map_gross();

        let mat: SMatrix<u32, 24, 24> =
            stack![aut, 0, 0, 0; 0, aut, 0, 0; 0, 0, inv, 0; 0, 0, 0, inv];

        let result = (mat * vec).map(|v| v % 2);

        // Convert to array and then to PauliString
        let arr: [u32; 24] = result.into();
        (&arr).into()
    }

    /// Construct all base measurements, i.e. measurements without automorphisms applied.
    pub fn base_measurements() -> impl Iterator<Item = NativeMeasurement> {
        NativeMeasurement::all_bases()
            .into_iter()
            .map(|basis| NativeMeasurement {
                logical: basis,
                automorphism: AutomorphismData::new(0, 0),
            })
    }

    /// Construct all native measurements
    pub fn all() -> Vec<NativeMeasurement> {
        let mut res = vec![];
        for x in 0..=5 {
            for y in 0..=5 {
                let aut = AutomorphismData::new(x, y);

                for base in NativeMeasurement::base_measurements() {
                    res.push(NativeMeasurement {
                        automorphism: aut,
                        ..base
                    });
                }
            }
        }

        res
    }

    fn all_bases() -> Vec<TwoBases> {
        let paulis = [Pauli::I, Pauli::X, Pauli::Z, Pauli::Y];

        let mut out = vec![];
        for p1 in &paulis {
            for p7 in &paulis {
                let two = TwoBases::new(*p1, *p7);
                if let Some(t) = two {
                    out.push(t);
                }
            }
        }

        out
    }

    pub fn implementation(&self) -> [BicycleISA; 3] {
        [
            BicycleISA::Automorphism(self.automorphism),
            BicycleISA::Measure(self.logical),
            BicycleISA::Automorphism(self.automorphism.inv()),
        ]
    }
}

impl Display for NativeMeasurement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NativeMeasurement({}) = ({} conjugated with {})",
            self.measures(),
            BicycleISA::Measure(self.logical),
            BicycleISA::Automorphism(self.automorphism)
        )
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;
    use Pauli::{I, X, Y, Z};

    use super::*;

    #[test]
    fn measurement_pivot() {
        let meas = TwoBases::new(Pauli::X, Pauli::Z).unwrap();
        let res = NativeMeasurement {
            logical: meas,
            automorphism: AutomorphismData::new(0, 0),
        };
        println!("{}", res.measures());
    }

    #[test]
    fn valid_paulistrings() {
        for native in NativeMeasurement::all() {
            let p = native.measures();
            assert!(
                p.0 < 2_u32.pow(24),
                "PauliString {:?} integer is too large",
                p
            );
        }
    }

    #[test]
    fn all_bases() {
        let bases = NativeMeasurement::all_bases();
        assert_eq!(15, bases.len());
    }

    #[test]
    fn all_base_measurements() {
        let base: Vec<_> = NativeMeasurement::base_measurements().collect();
        assert_eq!(15, base.len())
    }

    #[test]
    fn all_native() {
        let all_native = NativeMeasurement::all();
        assert_eq!(15 * 36, all_native.len());

        let set: HashSet<_> = all_native.iter().map(|n| n.measures()).collect();
        assert_eq!(15 * 36, set.len());
    }

    fn paulis_support(ps: &[Pauli; 12]) -> (bool, bool) {
        (
            ps[0..6].iter().any(|p| p != &I),
            ps[6..].iter().any(|p| p != &I),
        )
    }

    /// Test that the support of a native measurement on the primal / dual block
    /// does not "spill over" to the dual/primal block.
    #[test]
    fn pivot_duality() {
        let nontrivial_paulis = [X, Y, Z];

        for pauli in nontrivial_paulis {
            let logicals = [
                TwoBases::new(pauli, I).unwrap(),
                TwoBases::new(I, pauli).unwrap(),
            ];

            for (support_i, logical) in logicals.into_iter().enumerate() {
                let expected_support = (support_i == 0, support_i == 1);
                for x in 0..=5 {
                    for y in 0..=5 {
                        let automorphism = AutomorphismData::new(x, y);
                        let native_meas = NativeMeasurement {
                            logical,
                            automorphism,
                        };
                        dbg!(native_meas);
                        println!("{}", native_meas.measures());
                        let paulis: [Pauli; 12] = (&native_meas.measures()).into();

                        assert_eq!(expected_support, paulis_support(&paulis));
                    }
                }
            }
        }
    }
}

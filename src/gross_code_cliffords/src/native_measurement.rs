// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use std::fmt::Display;

use bicycle_isa::{AutomorphismData, BicycleISA, Pauli, TwoBases};
use rand::distr::{Distribution, StandardUniform};
use serde::{Deserialize, Serialize};

/// A measurement that can be performed on the code by conjugating one base measurement with automorphisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeMeasurement {
    pub logical: TwoBases,
    pub automorphism: AutomorphismData,
}

impl NativeMeasurement {
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
            "NativeMeasurement = ({} conjugated with {})",
            BicycleISA::Measure(self.logical),
            BicycleISA::Automorphism(self.automorphism)
        )
    }
}

impl Distribution<NativeMeasurement> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> NativeMeasurement {
        NativeMeasurement {
            logical: StandardUniform.sample(rng),
            automorphism: StandardUniform.sample(rng),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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
}

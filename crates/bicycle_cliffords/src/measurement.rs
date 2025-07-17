// Copyright contributors to the Bicycle Architecture Compiler project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::{Debug, Display};

use bicycle_common::{AutomorphismData, Pauli};
use nalgebra::{matrix, stack, SMatrix, Vector6};

use crate::{native_measurement::NativeMeasurement, PauliString};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CodeMeasurement {
    pub mx: SMatrix<u32, 6, 6>, // 6x6 matrix in F_2. Use u32 to avoid overflow.
    pub my: SMatrix<u32, 6, 6>,
}

impl CodeMeasurement {
    /// The PauliString a NativeMeasurement measures
    #[allow(clippy::toplevel_ref_arg)]
    pub fn measures(&self, native_measurement: &NativeMeasurement) -> PauliString {
        let one = Vector6::identity();
        let zero = Vector6::zeros();

        let (x1, z1) = match native_measurement.logical.get_basis_1() {
            Pauli::I => (zero, zero),
            Pauli::X => (one, zero),
            Pauli::Z => (zero, one),
            Pauli::Y => (one, one),
        };

        let (x7, z7) = match native_measurement.logical.get_basis_7() {
            Pauli::I => (zero, zero),
            Pauli::X => (one, zero),
            Pauli::Z => (zero, one),
            Pauli::Y => (one, one),
        };

        let vec = stack![x1; x7; z1; z7];

        // Compute action of automorphism on the Paulis
        let action = |a: AutomorphismData| {
            (self.mx.pow(a.get_x().into()) * self.my.pow(a.get_y().into())).map(|v| v % 2)
        };
        let aut = action(native_measurement.automorphism);
        let inv = action(native_measurement.automorphism.inv());
        let mat: SMatrix<_, 24, 24> =
            stack![aut, 0, 0, 0; 0, aut, 0, 0; 0, 0, inv, 0; 0, 0, 0, inv];

        let result = (mat * vec).map(|v| v % 2);
        // Convert to array and then to PauliString
        let arr: [_; 24] = result.into();
        (&arr).into()
    }
}

pub const GROSS_MEASUREMENT: CodeMeasurement = CodeMeasurement {
    mx: matrix![
        0, 1, 0, 1, 0, 0; //
        0, 1, 0, 0, 0, 1; //
        0, 0, 1, 1, 0, 0; //
        1, 1, 0, 1, 1, 0; //
        0, 1, 0, 0, 1, 0; //
        1, 1, 1, 1, 0, 1; //
    ],
    my: matrix![
        1, 0, 0, 0, 0, 1; //
        1, 1, 1, 0, 0, 1; //
        0, 0, 0, 0, 1, 0; //
        0, 1, 0, 0, 0, 0; //
        0, 1, 1, 0, 0, 1; //
        0, 0, 1, 1, 0, 1; //
    ],
};

pub const TWOGROSS_MEASUREMENT: CodeMeasurement = CodeMeasurement {
    mx: matrix![
        0, 1, 1, 1, 0, 1; //
        1, 0, 1, 0, 1, 1; //
        1, 0, 1, 0, 1, 0; //
        1, 0, 1, 1, 1, 1; //
        0, 1, 1, 1, 1, 1; //
        1, 0, 0, 1, 1, 0; //
    ],
    my: matrix![
        1, 1, 1, 1, 1, 0; //
        1, 1, 0, 1, 1, 1; //
        0, 1, 1, 0, 0, 0; //
        1, 0, 0, 0, 1, 0; //
        1, 0, 0, 1, 1, 1; //
        1, 0, 0, 0, 0, 1; //
    ],
};

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum MeasurementChoices {
    Gross,
    TwoGross,
}

impl MeasurementChoices {
    pub fn measurement(&self) -> CodeMeasurement {
        match self {
            Self::Gross => GROSS_MEASUREMENT,
            Self::TwoGross => TWOGROSS_MEASUREMENT,
        }
    }
}

impl Display for MeasurementChoices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gross => write!(f, "gross"),
            Self::TwoGross => write!(f, "two-gross"),
        }
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use bicycle_common::TwoBases;

    use super::*;

    use Pauli::{I, X, Y, Z};

    /// Test that the support of a native measurement on the primal / dual block
    /// does not "spill over" to the dual/primal block.
    #[test]
    fn pivot_duality_gross() {
        test_duality(GROSS_MEASUREMENT);
    }

    #[test]
    fn pivot_duality_2gross() {
        test_duality(TWOGROSS_MEASUREMENT);
    }

    fn paulis_support(ps: &[Pauli; 12]) -> (bool, bool) {
        (
            ps[0..6].iter().any(|p| p != &I),
            ps[6..].iter().any(|p| p != &I),
        )
    }

    fn test_duality(code: CodeMeasurement) {
        for pauli in [X, Y, Z] {
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
                        let paulis: [Pauli; 12] = code.measures(&native_meas).into();

                        assert_eq!(expected_support, paulis_support(&paulis));
                    }
                }
            }
        }
    }

    #[test]
    fn all_native_rotations_gross() {
        all_native_rotations(GROSS_MEASUREMENT);
    }
    #[test]
    fn all_native_rotations_two_gross() {
        all_native_rotations(TWOGROSS_MEASUREMENT);
    }

    fn all_native_rotations(code: CodeMeasurement) {
        let base = NativeMeasurement::all();
        let rots: HashSet<_> = base
            .into_iter()
            .map(|m| code.measures(&m).zero_pivot())
            .collect();

        // Some code to print native rotations
        // let mut sorted_rots: Vec<_> = rots.into_iter().collect();
        // sorted_rots.sort();
        // println!("{}", sorted_rots.len());

        // for rot in &sorted_rots {
        //     println!("{rot}");
        // }
        assert_eq!(511, rots.len());
    }
    #[test]
    fn all_native_gross() {
        all_native(GROSS_MEASUREMENT);
    }
    #[test]
    fn all_native_two_gross() {
        all_native(TWOGROSS_MEASUREMENT);
    }

    fn all_native(code: CodeMeasurement) {
        let all_native = NativeMeasurement::all();
        assert_eq!(15 * 36, all_native.len());

        let set: HashSet<_> = all_native.iter().map(|n| code.measures(n)).collect();

        // Some code to print the native measurements
        // let mut ms: Vec<_> = set.iter().collect();
        // ms.sort();

        // for m in ms {
        //     println!("{m}");
        // }
        assert_eq!(15 * 36, set.len());
    }

    #[test]
    fn valid_paulistrings_gross() {
        valid_paulistrings(GROSS_MEASUREMENT);
    }

    #[test]
    fn valid_paulistring_two_gross() {
        valid_paulistrings(TWOGROSS_MEASUREMENT);
    }

    fn valid_paulistrings(code: CodeMeasurement) {
        for native in NativeMeasurement::all() {
            let p = code.measures(&native);
            assert!(
                p.0 < 2_u32.pow(24),
                "PauliString {:?} integer is too large",
                p
            );
        }
    }

    // Check that the order of the automorphism generators is 6
    #[test]
    fn automorphism_order() {
        for m in [GROSS_MEASUREMENT, TWOGROSS_MEASUREMENT] {
            let mx = m.mx;
            let my = m.my;
            assert_eq!(mx, mx.pow(7).map(|v| v % 2));
            assert_eq!(my, my.pow(7).map(|v| v % 2));
        }
    }
}

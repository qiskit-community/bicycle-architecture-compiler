use std::fmt::Debug;

use bicycle_isa::{AutomorphismData, Pauli};
use nalgebra::{stack, Matrix6, SMatrix, Vector6};

use crate::{native_measurement::NativeMeasurement, PauliString};

pub trait Measurement: Debug {
    /// The PauliString a NativeMeasurement measures
    #[allow(clippy::toplevel_ref_arg)]
    fn measures(&self, native_measurement: &NativeMeasurement) -> PauliString {
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
        let (mx, my) = self.automorphism_generators();
        let action = |a: AutomorphismData| {
            (mx.pow(a.get_x().into()) * my.pow(a.get_y().into())).map(|v| v % 2)
        };
        let aut = action(native_measurement.automorphism);
        let inv = action(native_measurement.automorphism.inv());
        let mat: SMatrix<u32, 24, 24> =
            stack![aut, 0, 0, 0; 0, aut, 0, 0; 0, 0, inv, 0; 0, 0, 0, inv];

        let result = (mat * vec).map(|v| v % 2);
        // Convert to array and then to PauliString
        let arr: [u32; 24] = result.into();
        (&arr).into()
    }

    fn automorphism_generators(&self) -> (Matrix6<u32>, Matrix6<u32>);
}

#[derive(Debug, Clone, Copy)]
pub struct GrossCode;

impl Measurement for GrossCode {
    /// Generate the parity map associated with this automorphism on the Gross code
    fn automorphism_generators(&self) -> (Matrix6<u32>, Matrix6<u32>) {
        let mx_array: [u32; 36] = [
            0, 1, 0, 1, 0, 0, //
            0, 1, 0, 0, 0, 1, //
            0, 0, 1, 1, 0, 0, //
            1, 1, 0, 1, 1, 0, //
            0, 1, 0, 0, 1, 0, //
            1, 1, 1, 1, 0, 1, //
        ];
        let my_array: [u32; 36] = [
            1, 0, 0, 0, 0, 1, //
            1, 1, 1, 0, 0, 1, //
            0, 0, 0, 0, 1, 0, //
            0, 1, 0, 0, 0, 0, //
            0, 1, 1, 0, 0, 1, //
            0, 0, 1, 1, 0, 1, //
        ];

        let mx = Matrix6::from_row_slice(&mx_array);
        let my = Matrix6::from_row_slice(&my_array);

        (mx, my)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TwoGrossCode;

impl Measurement for TwoGrossCode {
    fn automorphism_generators(&self) -> (Matrix6<u32>, Matrix6<u32>) {
        let mx_array: [u32; 36] = [
            0, 1, 1, 1, 0, 1, //
            1, 0, 1, 0, 1, 1, //
            1, 0, 1, 0, 1, 0, //
            1, 0, 1, 1, 1, 1, //
            0, 1, 1, 1, 1, 1, //
            1, 0, 0, 1, 1, 0, //
        ];

        let my_array: [u32; 36] = [
            1, 1, 1, 1, 1, 0, //
            1, 1, 0, 1, 1, 1, //
            0, 1, 1, 0, 0, 0, //
            1, 0, 0, 0, 1, 0, //
            1, 0, 0, 1, 1, 1, //
            1, 0, 0, 0, 0, 1, //
        ];
        let mx = Matrix6::from_row_slice(&mx_array);
        let my = Matrix6::from_row_slice(&my_array);

        (mx, my)
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use bicycle_isa::TwoBases;

    use super::*;

    use Pauli::{I, X, Y, Z};

    /// Test that the support of a native measurement on the primal / dual block
    /// does not "spill over" to the dual/primal block.
    #[test]
    fn pivot_duality_gross() {
        test_duality(GrossCode);
    }

    #[test]
    fn pivot_duality_2gross() {
        test_duality(TwoGrossCode);
    }

    fn paulis_support(ps: &[Pauli; 12]) -> (bool, bool) {
        (
            ps[0..6].iter().any(|p| p != &I),
            ps[6..].iter().any(|p| p != &I),
        )
    }

    fn test_duality(code: impl Measurement) {
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
        all_native_rotations(GrossCode);
    }
    #[test]
    fn all_native_rotations_two_gross() {
        all_native_rotations(TwoGrossCode);
    }

    fn all_native_rotations(code: impl Measurement) {
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
        all_native(GrossCode);
    }
    #[test]
    fn all_native_two_gross() {
        all_native(TwoGrossCode);
    }

    fn all_native(code: impl Measurement) {
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
        valid_paulistrings(GrossCode);
    }

    #[test]
    fn valid_paulistring_two_gross() {
        valid_paulistrings(TwoGrossCode);
    }

    fn valid_paulistrings(code: impl Measurement) {
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
        let (mx, my) = GrossCode.automorphism_generators();
        assert_eq!(mx, mx.pow(7).map(|v| v % 2));
        assert_eq!(my, my.pow(7).map(|v| v % 2));

        let (mx, my) = TwoGrossCode.automorphism_generators();
        assert_eq!(mx, mx.pow(7).map(|v| v % 2));
        assert_eq!(my, my.pow(7).map(|v| v % 2));
    }
}

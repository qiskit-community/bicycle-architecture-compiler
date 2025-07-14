// Copyright contributors to the Bicycle Architecture Compiler project

use std::{
    array::TryFromSliceError,
    fmt,
    ops::{Index, Mul},
};

use bicycle_common::Pauli;
use rand::distr::{Distribution, StandardUniform};
use serde::{Deserialize, Serialize};

/// Represent a string of 12 Paulis
/// Consider using bitvec's bitarray to store Pauli rotations instead of reimplementing the bit twiddling.
/// We store the qubits in little-endian order, i.e.,
/// the bits 0 and 12 store qubit 0's X and Z operators, respectively.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PauliString(pub u32);

impl PauliString {
    pub fn rotation(bits: u32) -> PauliString {
        let z_bits = bits >> 11;
        let x_bits = bits & ((1 << 11) - 1);
        PauliString((z_bits << 13) | (x_bits << 1)) // Skip the pivot bits
    }

    fn get_bit(&self, index: usize) -> bool {
        (self.0 >> index) & 1 == 1
    }

    fn set_bit(&mut self, index: usize, value: bool) {
        if value {
            self.0 |= 1 << index;
        } else {
            self.0 &= !(1 << index);
        }
    }

    pub fn get_pauli(&self, i: usize) -> Pauli {
        match (self[i], self[i + 12]) {
            (true, true) => Pauli::Y,
            (true, false) => Pauli::X,
            (false, true) => Pauli::Z,
            (false, false) => Pauli::I,
        }
    }

    /// Set the ith index of this PauliString to p
    pub fn set_pauli(&mut self, i: usize, p: Pauli) {
        assert!(i <= 11);
        self.set_bit(i, p == Pauli::X || p == Pauli::Y);
        self.set_bit(i + 12, p == Pauli::Z || p == Pauli::Y);
    }

    // Check if the logical operator has support on the pivot qubit (0)
    pub fn has_pivot_support(&self) -> bool {
        self.pivot_bits().0 != 0
    }

    /// Check if the logical operator has support on a logical qubits (1 through 11)
    pub fn has_logical_support(&self) -> bool {
        self.logical_bits().0 != 0
    }

    /// Check if measurement has support on pivot and on at least one logical qubit.
    pub fn non_trivial_support(&self) -> bool {
        self.has_pivot_support() && self.has_logical_support()
    }

    pub fn commutes_with(self, rhs: PauliString) -> bool {
        let self_z = self.0 >> 12;
        let self_x = self.0 ^ (self_z << 12);
        let self_transpose = self_x << 12 | self_z;

        (self_transpose & rhs.0).count_ones() % 2 == 0
    }

    /// Assuming self is a measurement, apply rhs from the left if it anti-commutes with self.
    pub fn conjugate_with(self, rhs: PauliString) -> PauliString {
        if self.commutes_with(rhs) {
            self
        } else {
            self * rhs
        }
    }

    /// Return Pauli string on qubit 1
    pub fn pivot_bits(self) -> PauliString {
        let bits = self.0 & (1 | (1 << 12));
        PauliString(bits)
    }

    /// Set pivot bits to 0
    /// In effect we set the first Pauli to identity
    pub fn zero_pivot(self) -> PauliString {
        PauliString(self.0 & !(1 << 12 | 1))
    }

    /// Return Paulistring on 11 qubits with pivot removed.
    /// In effect, we shift Paulis 2--12 down to 1--11 and set 12 to identity.
    pub fn logical_bits(self) -> PauliString {
        let z = self.0 >> 13;
        let x = (self.0 & ((1 << 12) - 1)) >> 1;
        let bits = z << 11 | x;
        PauliString(bits)
    }
}

pub const ID: PauliString = PauliString(0);
pub const X1: PauliString = PauliString(1);
pub const Z1: PauliString = PauliString(1 << 12);
pub const Y1: PauliString = PauliString(1 | (1 << 12));

#[allow(clippy::suspicious_arithmetic_impl)]
impl Mul for PauliString {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        PauliString(self.0 ^ rhs.0)
    }
}

impl Index<usize> for PauliString {
    type Output = bool;

    fn index(&self, i: usize) -> &Self::Output {
        if self.get_bit(i) {
            &true
        } else {
            &false
        }
    }
}

impl From<&[u32; 24]> for PauliString {
    fn from(value: &[u32; 24]) -> Self {
        let mut sum = 0;
        // ensure that bit 0 indicates X_0 and bit 12 indicates Z_0.
        for bit in value.iter().rev() {
            sum <<= 1;
            sum += bit;
        }
        PauliString(sum)
    }
}

impl From<&[Pauli; 12]> for PauliString {
    /// Given Paulis in the order [qubit 0, qubit 1, ...], produce a corresponding PauliString.
    fn from(value: &[Pauli; 12]) -> Self {
        let mut sum = 0;
        // Do Z first
        for pauli in value.iter().rev() {
            match pauli {
                Pauli::Z | Pauli::Y => sum = (sum << 1) | 1,
                _ => sum <<= 1,
            }
        }
        // Then X
        for pauli in value.iter().rev() {
            match pauli {
                Pauli::X | Pauli::Y => sum = (sum << 1) | 1,
                _ => sum <<= 1,
            }
        }

        PauliString(sum)
    }
}

impl TryFrom<&[Pauli]> for PauliString {
    type Error = TryFromSliceError;
    fn try_from(value: &[Pauli]) -> Result<Self, Self::Error> {
        let ps: [Pauli; 12] = value.try_into()?;
        Ok(Self::from(&ps))
    }
}

impl From<&PauliString> for u32 {
    fn from(value: &PauliString) -> Self {
        value.0
    }
}

impl From<PauliString> for [Pauli; 12] {
    fn from(value: PauliString) -> Self {
        let mut paulis = vec![];
        for i in 0..12 {
            let x = value[i];
            let z = value[i + 12];
            let pauli = match (x, z) {
                (true, true) => Pauli::Y,
                (true, false) => Pauli::X,
                (false, true) => Pauli::Z,
                (false, false) => Pauli::I,
            };
            paulis.push(pauli);
        }

        paulis.try_into().unwrap()
    }
}

// impl Ord for PauliString {
// fn cmp(&self, other: &Self) -> std::cmp::Ordering {
//     // Create integer representing z_12x_12...z_1x_1 in binary
//     fn cmp_int(rot: &Pauli) -> u32 {
//         // Get the first 12 bits
//         let x_bits = rot.0 & ((1 << 12) - 1);
//         let z_bits = rot.0 >> 12;

//         let mut sum = 0;
//         // Interleave X and Z bits
//         for i in (0..12).rev() {
//             sum <<= 1;
//             sum += (z_bits >> i) & 1;
//             sum <<= 1;
//             sum += (x_bits >> i) & 1;
//         }

//         sum
//     }

//     let s1 = cmp_int(self);
//     let s2 = cmp_int(other);

//     s1.cmp(&s2)
// }
// }

// impl PartialOrd for PauliString {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         Some(self.cmp(other))
//     }
// }

impl fmt::Debug for PauliString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PauliString({:024b})", self.0)
    }
}

impl fmt::Display for PauliString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let paulis: [Pauli; 12] = (*self).into();
        for pauli in paulis.iter().rev() {
            write!(f, "{}", pauli)?;
        }
        Ok(())
    }
}

impl Distribution<PauliString> for StandardUniform {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> PauliString {
        PauliString(rng.random_range(0..4_u32.pow(12)))
    }
}

#[cfg(test)]
mod tests {
    use bicycle_common::Pauli;

    use super::*;

    use Pauli::{I, X, Y, Z};

    const X2: PauliString = PauliString(1 << 1);
    const X5: PauliString = PauliString(1 << 4);
    const Z2: PauliString = PauliString(1 << 13);

    #[test]
    fn rotation_construction() {
        assert_eq!(X2, PauliString::rotation(1));
        assert_eq!(Z2, PauliString::rotation(1 << 11));
        assert_eq!(X2 * Z2, PauliString::rotation(1 | 1 << 11))
    }

    #[test]
    fn check_commutes() {
        assert!(X1.commutes_with(X1));
        assert!(!X1.commutes_with(Z1));
        assert!(X1.commutes_with(Z2));
        assert!(!(X1 * X2).commutes_with(Z1 * X1 * X2));
        assert!((X1 * X2).commutes_with(Z1 * Z2));
    }

    #[test]
    fn check_conjugate() {
        let y1 = X1 * Z1;
        let y2 = X2 * Z2;

        // Commuting terms
        assert_eq!(X1, X1.conjugate_with(X1));
        assert_eq!(X2, X2.conjugate_with(X1));
        assert_eq!(X1, X1.conjugate_with(X2));
        assert_eq!(X1, X1.conjugate_with(X1 * X2));

        assert_eq!(y1, X1.conjugate_with(Z1));
        assert_eq!(y1, Z1.conjugate_with(X1));

        // Conjugate paulis to identity
        assert_eq!(y1, (X1 * y2).conjugate_with(Z1 * y2));
    }

    #[test]
    fn check_logical_bits() {
        assert_eq!(ID, X1.logical_bits());
        assert_eq!(X1, X2.logical_bits());
        assert_eq!(PauliString(1 << 11), Z2.logical_bits());
    }

    #[test]
    fn check_display() {
        let y1 = X1 * Z1;

        assert_eq!("IIIIIIIIIIIX", format!("{}", X1));
        assert_eq!("IIIIIIIIIIIZ", format!("{}", Z1));
        assert_eq!("IIIIIIIIIIIY", format!("{}", y1));
        assert_eq!("IIIIIIIXIIIZ", format!("{}", Z1 * X5));
    }

    #[test]
    fn from_paulis() {
        let paulis_arr = [X, I, X, I, I, I, I, I, I, I, I, I];
        let pauli_str: PauliString = (&paulis_arr).into();
        assert_eq!(PauliString(0b000000000000000000000101), pauli_str);
        let paulis_arr = [I, X, Z, Y, Y, Z, X, I, I, X, Z, Y];
        let pauli_str: PauliString = (&paulis_arr).into();
        assert_eq!(PauliString(0b110000111100101001011010), pauli_str);
    }
}

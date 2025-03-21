use bicycle_isa::Pauli;
use pbc_gross::language::PbcOperation;

use rand::{distr::Uniform, prelude::*};

/// Generate an infinite iterator of random measurements
pub fn random_measurements(qubits: usize) -> impl Iterator<Item = PbcOperation> {
    let mut rng = rand::rng();
    let range = Uniform::new_inclusive(0, 3).unwrap();

    // I'm not sure how I can compose random_paulis() with mapping to a Measurement;
    // I run into borrow issues.
    std::iter::repeat_with(move || {
        let paulis: Vec<_> = (0..qubits)
            .map(|_| match range.sample(&mut rng) {
                0 => Pauli::I,
                1 => Pauli::X,
                2 => Pauli::Z,
                3 => Pauli::Y,
                _ => unreachable!("RNG number out of range"),
            })
            .collect();

        PbcOperation::Measurement {
            basis: paulis,
            flip_result: false,
        }
    })
    // Remove measurements that are all identity
    .filter(|measurement| !measurement.basis().iter().all(|p| *p == Pauli::I))
}

pub fn random_paulis() -> impl Iterator<Item = Pauli> + 'static {
    let mut rng = rand::rng();
    let range = Uniform::new_inclusive(0, 3).unwrap();

    std::iter::repeat_with(move || match range.sample(&mut rng) {
        0 => Pauli::I,
        1 => Pauli::X,
        2 => Pauli::Z,
        3 => Pauli::Y,
        _ => unreachable!("RNG number out of range"),
    })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_rand_paulis() {}
}

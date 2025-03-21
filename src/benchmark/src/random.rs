use bicycle_isa::Pauli;
use pbc_gross::language::PbcOperation;

use rand::{distr::Uniform, prelude::*};

pub fn random_measurements(qubits: usize) -> impl Iterator<Item = PbcOperation> {
    std::iter::repeat(0).map(move |_| PbcOperation::Measurement {
        basis: random_paulis(qubits),
        flip_result: false,
    })
}

pub fn random_paulis(length: usize) -> Vec<Pauli> {
    let mut rng = rand::rng();
    let range = Uniform::new_inclusive(0, 3).unwrap();

    (0..length)
        .map(|_| match range.sample(&mut rng) {
            0 => Pauli::I,
            1 => Pauli::X,
            2 => Pauli::Z,
            3 => Pauli::Y,
            _ => unreachable!("RNG number out of range"),
        })
        .collect()
}

use bicycle_isa::Pauli;
use pbc_gross::language::{AnglePrecision, PbcOperation};

use rand::distr::{Distribution, StandardUniform};

/// Generate random circuit with non-trivial rotations, equivalent to a Clifford+T circuit
pub fn random_rotations(
    qubits: usize,
    angle: AnglePrecision,
) -> impl Iterator<Item = PbcOperation> {
    random_pauli_strings(qubits)
        .map(move |ps| PbcOperation::Rotation { basis: ps, angle })
        .filter(|rotation| !rotation.basis().iter().all(|p| *p == Pauli::I))
}

/// Generate an infinite iterator of random measurements
pub fn random_measurements(qubits: usize) -> impl Iterator<Item = PbcOperation> {
    random_pauli_strings(qubits)
        .map(|ps| PbcOperation::Measurement {
            basis: ps,
            flip_result: false,
        })
        // Remove measurements that are all identity
        .filter(|measurement| !measurement.basis().iter().all(|p| *p == Pauli::I))
}

pub fn random_pauli_strings(qubits: usize) -> impl Iterator<Item = Vec<Pauli>> {
    random_paulis()
        .scan(vec![], move |buf, p| {
            buf.push(p);
            if buf.len() == qubits {
                let out = std::mem::take(buf);
                *buf = vec![];
                Some(Some(out))
            } else {
                Some(None)
            }
        })
        .flatten()
}

fn random_paulis() -> impl Iterator<Item = Pauli> {
    let rng = rand::rng();
    StandardUniform.sample_iter(rng)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_rand_paulis() {
        let _ps: Vec<_> = random_paulis().take(100).collect();
    }

    #[test]
    fn test_random_rotations() {
        for qubits in 1..100 {
            let angle = AnglePrecision::lit("0.1") * AnglePrecision::from_num(qubits);
            let rotations = random_rotations(qubits, angle).take(100);
            for instruction in rotations {
                if let PbcOperation::Rotation {
                    basis,
                    angle: rot_angle,
                } = instruction
                {
                    assert!(!basis.iter().all(|p| *p == Pauli::I));
                    assert_eq!(angle, rot_angle);
                } else {
                    unreachable!()
                }
            }
        }
    }

    #[test]
    fn test_random_measurements() {
        for qubits in 1..100 {
            let measurements = random_measurements(qubits).take(100);
            for measurement in measurements {
                if let PbcOperation::Measurement { basis, .. } = measurement {
                    assert!(!basis.iter().all(|p| *p == Pauli::I));
                    assert_eq!(qubits, basis.len());
                } else {
                    unreachable!();
                }
            }
        }
    }
}

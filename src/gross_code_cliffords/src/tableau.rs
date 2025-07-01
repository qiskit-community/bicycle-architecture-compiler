// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use core::fmt;
use std::{array, error::Error, hash::Hash};

use crate::pauli_rotation::PauliString;

#[derive(Debug, Clone)]
pub struct LogicalMeasurementErr;

impl Error for LogicalMeasurementErr {
    fn description(&self) -> &str {
        "Measured data qubits"
    }

    fn cause(&self) -> Option<&dyn Error> {
        None
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl fmt::Display for LogicalMeasurementErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LogicalMeasurementError()")
    }
}

// A tableau for the special case isometry with 1 ancilla and 11 data qubits
// 11 logical qubits * 2 X/Z * 24 + 1 * 24 = 552 bits
// (n-1) * 2 * 2n + 1 * 2n
// X_1 -> XYIZZZXXZZXX
//
// 2^552 tableaus. 2^64 buckets. 2^552/2^64
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tableau {
    logicals: [PauliString; 22],
    stabilizer: PauliString,
}

impl Tableau {
    pub fn new() -> Self {
        let x_logicals: [PauliString; 11] = array::from_fn(|i| PauliString(1 << (i + 1)));
        let z_logicals: [PauliString; 11] = array::from_fn(|i| PauliString(1 << (i + 13)));

        let mut logicals = [PauliString(0); 22];
        let (x, z) = logicals.split_at_mut(11);
        x.copy_from_slice(&x_logicals);
        z.copy_from_slice(&z_logicals);

        Tableau {
            logicals,
            stabilizer: PauliString(1),
        }
    }

    pub fn measure(&self, basis: &PauliString) -> Result<Tableau, LogicalMeasurementErr> {
        if self.stabilizer.commutes_with(*basis) {
            return Err(LogicalMeasurementErr {});
        }

        // Was Stabilized by P, and anticommutes with new stabilizer Q.
        // Need new logicals R to commute with Q. So save P*R.
        let logicals = self.logicals.map(|logical| {
            if logical.commutes_with(*basis) {
                logical
            } else {
                logical * self.stabilizer
            }
        });

        let mut res = Tableau {
            logicals,
            stabilizer: *basis,
        };
        res.normalize();
        Ok(res)
    }

    fn normalize(&mut self) {
        self.logicals = self.logicals.map(|logical| {
            let reduced = logical * self.stabilizer;
            if reduced < logical {
                reduced
            } else {
                logical
            }
        });
    }
}

impl Hash for Tableau {
    // Hash only depends on two values for speed. Can't fit more that 64 bits into hash anyway.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.logicals[0].hash(state);
        self.logicals[15].hash(state);
        self.stabilizer.hash(state);
    }
}

// pub fn measure_generator() -> HashSet<Tableau> {
//     let base_meas = native_measurements(); // ~255?
//     debug!("Found {} base measurements", base_meas.len());

//     let base_tableau = Tableau::new();
//     let mut found_tableaus: HashSet<Tableau> = HashSet::new();
//     let mut prev_tableaus = HashSet::new();
//     prev_tableaus.insert(base_tableau);

//     let mut cur = 0;
//     while cur < 5 {
//         debug!("Trying {} layers of measurements", cur);
//         debug!(
//             "There are {} previous tableaus to check",
//             prev_tableaus.len()
//         );
//         cur += 1;
//         let mut new_tableaus = HashSet::new();

//         for prev in prev_tableaus.iter() {
//             for base in &base_meas {
//                 let res = prev.measure(base);

//                 if let Ok(new_tableau) = res {
//                     if !found_tableaus.contains(&new_tableau) {
//                         new_tableaus.insert(new_tableau);
//                     }
//                 }
//             }
//         }

//         found_tableaus.extend(prev_tableaus);
//         prev_tableaus = new_tableaus;
//     }

//     // At end copy all remaining new tableaus into found
//     found_tableaus.extend(prev_tableaus);

//     found_tableaus
// }

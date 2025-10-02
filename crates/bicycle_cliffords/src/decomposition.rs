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

use std::collections::HashMap;

use crate::measurement::CodeMeasurement;
use crate::pauli_string::PauliString;
use crate::{native_measurement::NativeMeasurement, pauli_string};

use bicycle_common::{AutomorphismData, BicycleISA, TwoBases};
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

// Defines a rotation that is implemented by a rotation conjugated with a base rotation.
// Need appropriate measurements conjugating the rotation on the pivot.
// Assume that conjugated_with anti-commutes with rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct MeasurementTableEntry {
    measurement: PauliString,
    conjugated_with: Option<PauliString>,
    cost: u32,
}

impl MeasurementTableEntry {
    pub fn cost(&self) -> u32 {
        self.cost
    }

    pub fn implements(&self) -> PauliString {
        if let Some(conj) = self.conjugated_with {
            // Whenever we conjugate by a rotation, the pivot gets reset.
            self.measurement.conjugate_with(conj.zero_pivot())
        } else {
            self.measurement
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MeasurementImpl {
    base: NativeMeasurementImpl,
    rotations: Vec<NativeMeasurementImpl>,
    measures: PauliString,
}

impl MeasurementImpl {
    pub fn base_measurement(&self) -> &NativeMeasurementImpl {
        &self.base
    }

    pub fn rotations(&self) -> &Vec<NativeMeasurementImpl> {
        &self.rotations
    }

    pub fn measures(&self) -> PauliString {
        self.measures
    }
}

/// A wrapper for &NativeMeasurement that caches what it measures
/// Basically a nice wrapper for (PauliString, &NativeMeasurement)
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct NativeMeasurementImpl {
    native: NativeMeasurement,
    measures: PauliString,
}

impl NativeMeasurementImpl {
    pub fn new(native: NativeMeasurement, measures: PauliString) -> Self {
        Self { native, measures }
    }

    pub fn logical(&self) -> TwoBases {
        self.native.logical
    }

    pub fn automorphism(&self) -> AutomorphismData {
        self.native.automorphism
    }

    pub fn implementation(&self) -> [BicycleISA; 3] {
        self.native.implementation()
    }

    pub fn measures(&self) -> PauliString {
        self.measures
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteMeasurementTable {
    measurements: Vec<MeasurementTableEntry>,
    native_measurements: HashMap<PauliString, NativeMeasurement>,
}

impl CompleteMeasurementTable {
    /// Look up the implementation for measuring a PauliString
    fn get(&self, p: PauliString) -> Option<&MeasurementTableEntry> {
        self.measurements.get(MeasurementTableBuilder::index(p))
    }

    /// Returns the a native measurement and its conjugating native measurements that implement rotations
    /// The ordering of rotations is such that the first element conjugates the measurement first.
    /// The given PauliString must be a valid measurement defined on 12 qubits.
    pub fn implementation(&self, p: PauliString) -> MeasurementImpl {
        assert!(p.0 <= 4_u32.pow(12), "{}", p);
        assert!(p.0 != 0); // Cannot measure identity

        let mut implementation = self.get(p).unwrap();

        let mut rots = vec![];
        while let Some(conjugate) = implementation.conjugated_with {
            rots.push(conjugate);
            implementation = self.get(implementation.measurement).unwrap();
        }

        let base_meas = self
            .native_measurements
            .get(&implementation.measurement)
            .unwrap();
        let base_impl = NativeMeasurementImpl::new(*base_meas, implementation.measurement);

        let native_rots = rots
            .into_iter()
            .map(|p| {
                self.native_measurements
                    .get(&p)
                    .map(|native| NativeMeasurementImpl::new(*native, p))
                    .unwrap()
            })
            .rev()
            .collect();
        MeasurementImpl {
            measures: p,
            base: base_impl,
            rotations: native_rots,
        }
    }

    /// Minimize over the Pauli on the pivot to measure 11 qubits in the basis p.
    /// This can be useful if you do not care about the basis of the pivot.
    /// TODO: If this becomes the only method needed, then we can shrink table by factor 4.
    pub fn min_data(&self, p: PauliString) -> MeasurementImpl {
        assert!(p.0 <= 4_u32.pow(12), "{}", p);
        assert!(
            p.pivot_bits() == pauli_string::ID,
            "Expected identity on pivot for {p}"
        );

        // Find minimum-length implementation out of three options for the pivot.

        [pauli_string::X1, pauli_string::Z1, pauli_string::Y1]
            .into_iter()
            .map(|pivot_pauli| p * pivot_pauli) // insert pivot basis
            .map(|q| self.implementation(q)) // look up implementation
            .min_by_key(|meas_impl| meas_impl.rotations().len())
            .unwrap()
    }
}

impl TryFrom<MeasurementTableBuilder> for CompleteMeasurementTable {
    type Error = String;

    fn try_from(value: MeasurementTableBuilder) -> Result<Self, Self::Error> {
        let measurements: Option<Vec<_>> = value.measurements.into_iter().collect();
        Ok(CompleteMeasurementTable {
            measurements: measurements.ok_or("All measurements should have an implementation")?,
            native_measurements: value.native_measurements,
        })
    }
}

#[derive(Debug)]
pub struct MeasurementTableBuilder {
    measurements: Vec<Option<MeasurementTableEntry>>,
    native_measurements: HashMap<PauliString, NativeMeasurement>,
    len: usize, // Count how many Some entries there are in measurements
    code: CodeMeasurement,
}

impl MeasurementTableBuilder {
    pub fn new(native_measurements: Vec<NativeMeasurement>, code: CodeMeasurement) -> Self {
        let len = 0;
        let measurements = vec![None; 4usize.pow(12)];

        let native_lookup: HashMap<PauliString, NativeMeasurement> = native_measurements
            .into_iter()
            .map(|meas| (code.measures(&meas), meas))
            .collect();

        let mut table = MeasurementTableBuilder {
            measurements,
            native_measurements: HashMap::new(), // Placeholder; set later.
            len,
            code,
        };

        for p in native_lookup.keys() {
            table.insert(MeasurementTableEntry {
                measurement: *p,
                conjugated_with: None,
                cost: 1, // TODO: Adjust me depending on noise simulations!
            });
        }
        table.native_measurements = native_lookup;

        // Insert identity
        let identity = MeasurementTableEntry {
            measurement: PauliString(0),
            conjugated_with: None,
            cost: 0,
        };
        table.insert(identity);

        table
    }

    pub fn build(&mut self) {
        info!("Synthesizing all measurements from base measurements");
        let base_measurements = NativeMeasurement::all();

        // 4^12 possible Pauli measurements on 12 qubits
        let nr_paulis: usize = 4_usize.pow(12);

        let mut next_paulis = base_measurements
            .iter()
            .map(|m| self.code.measures(m))
            .collect();

        // Create a set of base rotations
        // We pick the cheapest rotation for each paulistring, if there is duplication
        let mut base_rots: HashMap<PauliString, MeasurementTableEntry> = HashMap::new();
        for native_impl in self.native_impls() {
            let p = native_impl.implements();
            // Must have pivot support so we can prepare an ancilla there
            if !p.has_pivot_support() {
                continue;
            }

            // Insert cheapest measurement implementation
            base_rots
                .entry(p)
                .and_modify(|cur| {
                    if cur.cost() > native_impl.cost() {
                        *cur = *native_impl;
                    }
                })
                .or_insert(*native_impl);
        }

        debug!(
            "Starting search with {} base measurements and {} base rotations",
            self.len(),
            base_rots.len()
        );
        for meas in self.native_impls() {
            trace!("Native measurement: {:?}", meas.implements());
        }

        let mut cur = 1; // Count loop iterations by the cost of the current rotation
        while self.len() < nr_paulis {
            let prev_paulis = next_paulis;
            next_paulis = Vec::new();

            cur += 1;
            debug!("Iteration {cur}");

            // Conjugate all rotations of the cur cost by all base measurements to find new rotations
            for prev_pauli in prev_paulis {
                // Tight inner loop of fixed size, maybe optimize somehow by giving compiler hint?
                for (rot_pauli, rot_impl) in base_rots.iter() {
                    let prev_meas = self.get(prev_pauli)
                        .expect("MeasurementTable should contain a previously found Pauli measurement implementation.");
                    let new_rotation_impl = MeasurementTableEntry {
                        measurement: prev_pauli,
                        conjugated_with: Some(*rot_pauli),
                        cost: prev_meas.cost() + 2 * rot_impl.cost(),
                    };

                    let new_pauli = new_rotation_impl.implements();
                    let existing = self.get(new_pauli);
                    match existing {
                        None => {
                            self.insert(new_rotation_impl);
                            next_paulis.push(new_pauli);
                        }
                        Some(existing_impl) => {
                            if existing_impl.cost() > new_rotation_impl.cost() {
                                self.insert(new_rotation_impl);
                                next_paulis.push(new_pauli);
                            }
                        }
                    }
                }
            }

            debug!("Found {} new operations of {} cost", next_paulis.len(), cur);
            debug!("Total operations found: {} / {}", self.len(), nr_paulis);

            if next_paulis.is_empty() {
                error!(
                    "Did not find new operations, aborting. Found {} / {} operations",
                    self.len(),
                    nr_paulis
                );
                for (index, meas_impl) in self.measurements.iter().enumerate() {
                    if meas_impl.is_none() {
                        warn!("Did not find {}", PauliString(index as u32));
                    }
                }
                break;
            }
        }
    }

    /// Try to convert to a complete measurement table
    pub fn complete(self) -> Result<CompleteMeasurementTable, String> {
        self.try_into()
    }

    fn index(p: PauliString) -> usize {
        let i = p.0 as usize;

        assert!(
            i <= 4_usize.pow(12),
            "PauliString {p:?} has index too large"
        );
        i
    }

    /// Look up the implementation for measuring a PauliString
    fn get(&self, p: PauliString) -> Option<&MeasurementTableEntry> {
        self.measurements[MeasurementTableBuilder::index(p)].as_ref()
    }

    /// Insert a MeasurementImpl into the table
    fn insert(&mut self, meas_impl: MeasurementTableEntry) {
        let i = MeasurementTableBuilder::index(meas_impl.implements());
        if self.measurements[i].is_none() {
            self.len += 1;
        }
        self.measurements[i] = Some(meas_impl);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len > 0
    }

    fn native_impls(&self) -> impl Iterator<Item = &MeasurementTableEntry> {
        self.native_measurements
            .keys()
            .map(|k| self.get(*k).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use bicycle_common::Pauli::{I, X, Y, Z};
    use bicycle_common::{AutomorphismData, TwoBases};

    use crate::{GROSS_MEASUREMENT, TWOGROSS_MEASUREMENT};

    use super::*;

    #[test]
    fn table_constructor() {
        let native = vec![NativeMeasurement {
            automorphism: AutomorphismData::new(0, 0),
            logical: TwoBases::new(X, Y).unwrap(),
        }];

        let mut table = MeasurementTableBuilder::new(native, GROSS_MEASUREMENT);
        assert_eq!(2, table.len());

        let p: PauliString = (&[Y, Y, I, I, I, Y, I, I, I, I, I, Z]).into();
        table.insert(MeasurementTableEntry {
            measurement: p,
            conjugated_with: None,
            cost: 0,
        });

        assert_eq!(3, table.len());
    }

    #[test]
    fn table_insert() {
        let mut table = MeasurementTableBuilder::new(vec![], GROSS_MEASUREMENT);

        let nrs = [
            0b111111111111111111111111,
            0b111111111111111111111110,
            0b000000000000000000000001,
        ];
        for nr in nrs {
            let p = PauliString(nr);
            let p_impl = MeasurementTableEntry {
                measurement: p,
                conjugated_with: None,
                cost: 0,
            };
            table.insert(p_impl);
        }

        assert_eq!(4, table.len());
    }

    #[test]
    fn table_get() {
        let mut table = MeasurementTableBuilder::new(vec![], GROSS_MEASUREMENT);
        let p: PauliString = (&[Y, Y, I, I, I, Y, I, I, I, I, I, Z]).into();
        let p_impl = MeasurementTableEntry {
            measurement: p,
            conjugated_with: None,
            cost: 1,
        };

        table.insert(p_impl);
        assert_eq!(Some(&p_impl), table.get(p));
    }

    #[test]
    fn test_gross_table() -> Result<(), String> {
        table_tests(GROSS_MEASUREMENT)
    }

    #[test]
    fn test_twogross_table() -> Result<(), String> {
        table_tests(TWOGROSS_MEASUREMENT)
    }

    fn table_tests(m: CodeMeasurement) -> Result<(), String> {
        let table: CompleteMeasurementTable = build_complete_table(m)?;
        check_correct_implementation(&table);
        check_native_measurements(&table, m);
        Ok(())
    }

    fn build_complete_table(m: CodeMeasurement) -> Result<CompleteMeasurementTable, String> {
        let mut table = MeasurementTableBuilder::new(NativeMeasurement::all(), m);
        table.build();

        let measurements = table.measurements.clone();
        let res: Vec<_> = measurements.into_iter().flatten().collect();
        assert_eq!(res.len(), table.len);

        table.complete()
    }

    fn check_correct_implementation(complete: &CompleteMeasurementTable) {
        // Check that the completed table gives correct implementations for each pauli string
        for i in 1..4_u32.pow(12) {
            let p = PauliString(i);
            let meas_impl = complete.implementation(p);
            let mut q = meas_impl.base_measurement().measures();

            for rot in meas_impl.rotations() {
                q = q.conjugate_with(rot.measures().zero_pivot());
            }

            assert_eq!(p, q);
        }
    }

    fn check_native_measurements(table: &CompleteMeasurementTable, code: CodeMeasurement) {
        let native_ps: Vec<_> = NativeMeasurement::all()
            .iter()
            .map(|native| code.measures(native))
            .collect();

        for native_p in native_ps {
            let implementation = table.implementation(native_p);
            assert_eq!(native_p, implementation.measures());
            assert_eq!(0, implementation.rotations().len());
        }
    }
}

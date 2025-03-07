use std::collections::HashMap;

use crate::native_measurement::NativeMeasurement;
use crate::pauli_rotation::PauliString;

use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};

// Defines a rotation that is implemented by a rotation conjugated with a base rotation.
// Need appropriate measurements conjugating the rotation on the pivot.
// Assume that conjugated_with anti-commutes with rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct MeasurementImpl {
    measurement: PauliString,
    conjugated_with: Option<PauliString>,
    cost: u32,
}

impl MeasurementImpl {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteMeasurementTable {
    measurements: Vec<MeasurementImpl>,
    native_measurements: HashMap<PauliString, NativeMeasurement>,
}

impl CompleteMeasurementTable {
    /// Look up the implementation for measuring a PauliString
    fn get(&self, p: PauliString) -> Option<&MeasurementImpl> {
        self.measurements.get(MeasurementTableBuilder::index(p))
    }

    /// Returns the a native measurement and its conjugating native measurements that implement rotations
    /// The ordering of rotations is such that the first element conjugates the measurement first.
    /// The given PauliString must be a valid measurement defined on 12 qubits.
    pub fn implementation(&self, p: PauliString) -> (&NativeMeasurement, Vec<&NativeMeasurement>) {
        assert!(p.0 <= 4_u32.pow(12));
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

        let native_rots = rots
            .iter()
            .map(|p| self.native_measurements.get(p).unwrap())
            .rev()
            .collect();
        (base_meas, native_rots)
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

#[derive(Debug, Clone)]
pub struct MeasurementTableBuilder {
    measurements: Vec<Option<MeasurementImpl>>,
    native_measurements: HashMap<PauliString, NativeMeasurement>,
    len: usize, // Count how many Some entries there are in measurements
}

impl MeasurementTableBuilder {
    pub fn new(native_measurements: Vec<NativeMeasurement>) -> Self {
        let len = 0;
        let measurements = vec![None; 4usize.pow(12)];

        let native_lookup: HashMap<PauliString, NativeMeasurement> = native_measurements
            .into_iter()
            .map(|meas| (meas.measures(), meas))
            .collect();

        let mut table = MeasurementTableBuilder {
            measurements,
            native_measurements: HashMap::new(), // Placeholder; set later.
            len,
        };

        for p in native_lookup.keys() {
            table.insert(MeasurementImpl {
                measurement: *p,
                conjugated_with: None,
                cost: 1, // TODO: Adjust me depending on noise simulations!
            });
        }
        table.native_measurements = native_lookup;

        // Insert identity
        let identity = MeasurementImpl {
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

        let mut next_paulis = base_measurements.iter().map(|m| m.measures()).collect();

        // Create a set of base rotations
        // We pick the cheapest rotation for each paulistring, if there is duplication
        let mut base_rots: HashMap<PauliString, MeasurementImpl> = HashMap::new();
        for native_impl in self.native_impls() {
            let p = native_impl.implements();

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
            debug!("Iteration {}", cur);

            // Conjugate all rotations of the cur cost by all base measurements to find new rotations
            for prev_pauli in prev_paulis {
                // Tight inner loop of fixed size, maybe optimize somehow by giving compiler hint?
                for (rot_pauli, rot_impl) in base_rots.iter() {
                    let prev_meas = self.get(prev_pauli)
                        .expect("MeasurementTable should contain a previously found Pauli measurement implementation.");
                    let new_rotation_impl = MeasurementImpl {
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
        if i > 4_usize.pow(12) {
            error!("PauliString {:?} has index too large", p);
        }
        i
    }

    /// Look up the implementation for measuring a PauliString
    fn get(&self, p: PauliString) -> Option<&MeasurementImpl> {
        self.measurements[MeasurementTableBuilder::index(p)].as_ref()
    }

    /// Insert a MeasurementImpl into the table
    fn insert(&mut self, meas_impl: MeasurementImpl) {
        let i = MeasurementTableBuilder::index(meas_impl.implements());
        if self.measurements[i].is_none() {
            self.len += 1;
        }
        self.measurements[i] = Some(meas_impl);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    fn native_impls(&self) -> impl Iterator<Item = &MeasurementImpl> {
        self.native_measurements
            .keys()
            .map(|k| self.get(*k).unwrap())
    }
}

#[cfg(test)]
mod tests {

    use bicycle_isa::Pauli::{I, X, Y, Z};
    use bicycle_isa::{AutomorphismData, TwoBases};

    use super::*;

    #[test]
    fn table_constructor() {
        let native = vec![NativeMeasurement {
            automorphism: AutomorphismData::new(0, 0),
            logical: TwoBases::new(X, Y).unwrap(),
        }];

        let mut table = MeasurementTableBuilder::new(native);
        assert_eq!(2, table.len());

        let p: PauliString = (&[Y, Y, I, I, I, Y, I, I, I, I, I, Z]).into();
        table.insert(MeasurementImpl {
            measurement: p,
            conjugated_with: None,
            cost: 0,
        });

        assert_eq!(3, table.len());
    }

    #[test]
    fn table_insert() {
        let mut table = MeasurementTableBuilder::new(vec![]);

        let nrs = [
            0b111111111111111111111111,
            0b111111111111111111111110,
            0b000000000000000000000001,
        ];
        for nr in nrs {
            let p = PauliString(nr);
            let p_impl = MeasurementImpl {
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
        let mut table = MeasurementTableBuilder::new(vec![]);
        let p: PauliString = (&[Y, Y, I, I, I, Y, I, I, I, I, I, Z]).into();
        let p_impl = MeasurementImpl {
            measurement: p,
            conjugated_with: None,
            cost: 1,
        };

        table.insert(p_impl);
        assert_eq!(Some(&p_impl), table.get(p));
    }

    #[test]
    fn test_measurement_builder() -> Result<(), String> {
        let mut table = MeasurementTableBuilder::new(NativeMeasurement::all());
        table.build();

        let measurements = table.measurements.clone();
        let res: Vec<_> = measurements.into_iter().flatten().collect();
        assert_eq!(res.len(), table.len);

        let complete = table.complete()?;

        // Check that the completed table gives correct implementations for each pauli string
        for i in 1..4_u32.pow(12) {
            let p = PauliString(i);
            let (base_meas, rots) = complete.implementation(p);
            let mut q = base_meas.measures();

            for rot in rots {
                q = q.conjugate_with(rot.measures().zero_pivot());
            }

            assert_eq!(p, q);
        }

        Ok(())
    }
}

// Copyright 2025 IBM
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

pub mod measurement;
pub use measurement::{
    CodeMeasurement, MeasurementChoices, GROSS_MEASUREMENT, TWOGROSS_MEASUREMENT,
};

pub mod native_measurement;
mod pauli_string;

pub use pauli_string::PauliString;

pub mod decomposition;
pub use decomposition::{CompleteMeasurementTable, MeasurementTableBuilder};

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use native_measurement::NativeMeasurement;

    static MEASUREMENT_IMPLS: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), TWOGROSS_MEASUREMENT);
        builder.build();
        builder
            .complete()
            .expect("Generating a complete measurement table should succeed")
    });

    #[test]
    fn qubit_measurements_are_native() {
        for i in 0..24 {
            let p: PauliString = PauliString(1 << i);
            let meas_impl = MEASUREMENT_IMPLS.implementation(p);
            assert_eq!(0, meas_impl.rotations().len());
        }
    }
}

// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

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

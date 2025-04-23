pub mod native_measurement;
mod pauli_rotation;

pub use pauli_rotation::PauliString;

pub mod decomposition;
pub use decomposition::{CompleteMeasurementTable, MeasurementTableBuilder};

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;
    use native_measurement::NativeMeasurement;

    static MEASUREMENT_IMPLS: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
        let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all());
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

// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

mod architecture;
mod basis_changer;
mod compile;
pub mod language;
pub mod operation;
pub mod optimize;
mod small_angle;

pub use architecture::PathArchitecture;

#[cfg(test)]
mod test {

    use std::error::Error;

    use crate::language::{AnglePrecision, PbcOperation};

    use super::*;
    use gross_code_cliffords::{
        native_measurement::NativeMeasurement, MeasurementTableBuilder, TWOGROSS_MEASUREMENT,
    };
    use operation::Operations;

    #[test]
    fn integration_test_rotation() -> Result<(), Box<dyn Error>> {
        let program = r#"[
                                    {
                                        "Rotation": {
                                        "basis": [
                                            "X",
                                            "X",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "I",
                                            "Y"
                                        ],
                                        "angle": "0.125"
                                        }
                                    }
                                ]"#;
        let parsed: Vec<PbcOperation> = serde_json::from_str(program)?;
        dbg!(&parsed);
        assert_eq!(1, parsed.len());

        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), TWOGROSS_MEASUREMENT);
        builder.build();
        let measurement_table = builder.complete()?;

        let architecture = PathArchitecture { data_blocks: 2 };
        let compiled: Vec<_> = parsed
            .into_iter()
            .flat_map(|op| {
                op.compile(
                    &architecture,
                    &measurement_table,
                    AnglePrecision::lit("1e-16"),
                )
            })
            .collect();
        let ops = Operations(compiled);

        println!("{}", ops);

        Ok(())
    }
}

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
    use bicycle_cliffords::{
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

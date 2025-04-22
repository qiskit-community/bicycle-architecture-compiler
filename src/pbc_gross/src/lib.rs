mod architecture;
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
                                        "angle": 0.125
                                        }
                                    }
                                ]"#;
        let parsed: Vec<PbcOperation> = serde_json::from_str(program)?;
        dbg!(&parsed);
        assert_eq!(1, parsed.len());

        let architecture = PathArchitecture { data_blocks: 2 };
        let compiled: Vec<_> = parsed
            .into_iter()
            .flat_map(|op| op.compile(&architecture, AnglePrecision::lit("1e-16")))
            .collect();
        let ops = Operations(compiled);

        println!("{}", ops);

        Ok(())
    }
}

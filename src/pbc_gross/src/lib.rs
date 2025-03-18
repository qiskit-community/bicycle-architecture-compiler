mod architecture;
mod compile;
pub mod language;
pub mod operation;
pub mod parser;
mod small_angle;

pub use architecture::PathArchitecture;
pub use compile::compile;

#[cfg(test)]
mod test {

    use std::error::Error;

    use super::*;
    use operation::Operations;

    #[test]
    fn integration_test_rotation() -> Result<(), Box<dyn Error>> {
        let program = "r,xxiiiiiiiii,-0.125";
        let mut parser = parser::PbcParser::new(program.as_bytes());
        let parsed = parser.stream().collect::<Result<Vec<_>, _>>()?;
        dbg!(&parsed);
        assert_eq!(1, parsed.len());

        let architecture = PathArchitecture { data_blocks: 2 };
        let compiled: Vec<_> = compile(architecture, parsed.into_iter()).collect();
        let ops = Operations(compiled);

        println!("{}", ops);

        Ok(())
    }
}

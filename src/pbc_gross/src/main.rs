use std::{error, io};

use io::Write;

use log::debug;
use pbc_gross::{parser::PbcParser, PathArchitecture};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let read = io::stdin();
    let mut parser = PbcParser::new(read);
    let ops = parser.stream();

    let architecture = PathArchitecture { data_blocks: 2 };
    let mut handle = io::stdout();
    for res in ops {
        let pbc_op = res?;
        let ops = pbc_op.compile(&architecture);
        for op in ops {
            debug!("{op:?}");
            serde_json::to_writer(&mut handle, &op)?;
            writeln!(&mut handle)?;
        }
    }

    Ok(())
}

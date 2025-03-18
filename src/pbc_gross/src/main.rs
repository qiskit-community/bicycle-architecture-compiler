use std::{error, io};

use io::Write;

use log::debug;
use pbc_gross::{parser::PbcParser, PathArchitecture};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let read = io::stdin();
    let mut parser = PbcParser::new(read);
    let ops = parser.stream();

    let mut handle = io::stdout();
    let mut architecture = None;
    for res in ops {
        let pbc_op = res?;
        architecture.get_or_insert(PathArchitecture {
            data_blocks: (pbc_op.basis().len() + 1).div_ceil(11),
        });
        let ops = pbc_op.compile(&architecture.unwrap());
        for op in ops {
            debug!("{op:?}");
            serde_json::to_writer(&mut handle, &op)?;
            writeln!(&mut handle)?;
        }
    }

    Ok(())
}

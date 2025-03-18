use std::{error, fs::File};

use log::{debug, info};
use pbc_gross::{parser, PathArchitecture};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let f = File::open("example/simple.csv")?;
    let ops = parser::parse_buf(f)?;
    info!("Read input");
    debug!(
        "[{}]",
        ops.iter()
            .map(|op| op.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let architecture = PathArchitecture { data_blocks: 2 };
    let compiled = pbc_gross::compile(architecture, ops.into_iter());

    for op in compiled {
        print!("[");
        let formatted = op
            .into_iter()
            .map(|(block_i, instr)| format!("({},{})", block_i, instr))
            .collect::<Vec<_>>()
            .join(",");
        println!("{}]", formatted);
    }

    Ok(())
}

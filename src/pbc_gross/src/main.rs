use std::{error, fs::File};

use architecture::PathArchitecture;

mod architecture;
mod compile;
mod language;
mod operation;
mod parser;
mod small_angle;

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let f = File::open("data/simple.csv")?;
    let ops = parser::parse_buf(f)?;

    println!("{:?}", ops);

    let test = ops.iter();

    let architecture = PathArchitecture { data_blocks: 2 };
    compile::compile(architecture, ops.into_iter());

    Ok(())
}

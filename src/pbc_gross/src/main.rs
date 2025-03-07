use std::{error, fs::File};

use pbc_gross::{parser, PathArchitecture};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let f = File::open("data/simple.csv")?;
    let ops = parser::parse_buf(f)?;

    println!("{:?}", ops);

    let test = ops.iter();

    let architecture = PathArchitecture { data_blocks: 2 };
    pbc_gross::compile(architecture, ops.into_iter());

    Ok(())
}

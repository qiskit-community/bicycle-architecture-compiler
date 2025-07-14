// Copyright contributors to the Bicycle Architecture Compiler project

use std::{
    error::Error,
    io::{self, Write},
};

use log::debug;

use bicycle_compiler::language::AnglePrecision;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    qubits: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();
    assert!(cli.qubits > 0);
    let cliff_angle = AnglePrecision::PI / AnglePrecision::lit("4.0");
    let mut measurements = bicycle_benchmark::random::random_rotations(cli.qubits, cliff_angle);

    let mut stdout = io::stdout();
    // Stop on first error
    let err = measurements.try_for_each(|measurement| {
        let mut out = serde_json::to_string(&measurement)?;
        out.push('\n');
        stdout.write_all(out.as_bytes())
    });
    debug!("Encountered error while writing to stdout: {:?}", err);

    Ok(())
}

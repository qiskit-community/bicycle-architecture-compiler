// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use std::{
    error::Error,
    io::{self, Write},
};

use log::debug;

use clap::Parser;
use pbc_gross::language::AnglePrecision;

#[derive(Parser)]
struct Cli {
    qubits: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();
    assert!(cli.qubits > 0);
    let cliff_angle = AnglePrecision::PI / AnglePrecision::lit("4.0");
    let mut measurements = benchmark::random::random_rotations(cli.qubits, cliff_angle);

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

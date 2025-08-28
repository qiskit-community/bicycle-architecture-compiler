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

use std::{
    error::Error,
    io::{self, Write},
};

use log::debug;

use bicycle_compiler::language::AnglePrecision;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    /// Number of logical qubits
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
    debug!("Encountered error while writing to stdout: {err:?}");

    Ok(())
}

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
    io::{BufWriter, Write},
};

use log::{debug, info};

use bicycle_cliffords::{
    MeasurementChoices, MeasurementTableBuilder, PauliString, native_measurement::NativeMeasurement,
};

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    code: MeasurementChoices,
    /// Do not optimize over choice of pivot basis. Result will be 12-qubit strings.
    #[arg(long)]
    no_optimize: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();

    let mut table = MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
    table.build();
    let complete = table.complete()?;
    debug!("Done with finding costs");

    println!("Rotation,Base Meas,Rots len");

    let stdout = std::io::stdout();
    let mut buf_out = BufWriter::new(stdout);
    if !cli.no_optimize {
        info!("Optimizing over pivot measurement basis");
        for i in 1..4_u32.pow(11) {
            // Find cheapest implementation for rotation
            let x_bits = i & ((1 << 11) - 1);
            let z_bits = i >> 11;
            let p = PauliString((z_bits << 13) | (x_bits << 1));
            let meas_impl = complete.min_data(p);
            writeln!(
                buf_out,
                "{},{},{}",
                p,
                meas_impl.base_measurement().measures(),
                meas_impl.rotations().len(),
            )?;
        }
    } else {
        info!("Not optimizing over pivot qubit");
        for i in 1..4_u32.pow(12) {
            let p = PauliString(i);
            let meas_impl = complete.implementation(p);
            writeln!(
                buf_out,
                "{},{},{}",
                p,
                meas_impl.base_measurement().measures(),
                meas_impl.rotations().len()
            )?;
        }
    }

    buf_out.flush()?;
    Ok(())
}

// (C) Copyright IBM 2025
//
// This code is licensed under the Apache License, Version 2.0. You may
// obtain a copy of this license in the LICENSE.txt file in the root directory
// of this source tree or at http://www.apache.org/licenses/LICENSE-2.0.
//
// Any modifications or derivative works of this code must retain this
// copyright notice, and modified files need to carry a notice indicating
// that they have been altered from the originals.

use std::error::Error;

use log::debug;

use gross_code_cliffords::{
    native_measurement::NativeMeasurement, MeasurementChoices, MeasurementTableBuilder, PauliString,
};

use clap::Parser;

#[derive(Parser, Debug)]
struct Cli {
    code: MeasurementChoices,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();

    let mut table = MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
    table.build();
    let complete = table.complete()?;
    debug!("Done with finding costs");

    println!("Rotation,Base Meas,Rots len");

    // Map into one string before sending to stdout for speed
    let output_lines = (1..4_u32.pow(11)).map(|i| {
        // Find cheapest implementation for rotation
        let x_bits = i & ((1 << 11) - 1);
        let z_bits = i >> 11;
        let p = PauliString((z_bits << 13) | (x_bits << 1));
        let meas_impl = complete.min_data(p);
        format!(
            "{},{},{}",
            p,
            meas_impl.base_measurement().measures(),
            meas_impl.rotations().len(),
        )
    });

    let mut output = String::new();
    for output_line in output_lines {
        output.push_str(&output_line);
        output.push('\n');
    }

    print!("{}", output);

    Ok(())
}

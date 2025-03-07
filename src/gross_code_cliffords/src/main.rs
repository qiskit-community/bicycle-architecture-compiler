use std::error::Error;

use log::debug;

use gross_code_cliffords::{
    native_measurement::NativeMeasurement, MeasurementTableBuilder, PauliString,
};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut table = MeasurementTableBuilder::new(NativeMeasurement::all());
    table.build();
    let complete = table.complete()?;
    debug!("Done with finding costs");

    println!("Rotation,Base Meas,Rots len");

    // Map into one string before sending to stdout for speed
    let output_lines = (1..4_u32.pow(12)).map(|i| {
        let p = PauliString(i);
        let (base_meas, rots) = complete.implementation(p);
        format!("{},{},{}", p, base_meas.measures(), rots.len(),)
    });

    let mut output = String::new();
    for output_line in output_lines {
        output.push_str(&output_line);
        output.push('\n');
    }

    print!("{}", output);

    Ok(())
}

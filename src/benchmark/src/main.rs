use core::str;
use std::{error::Error, io};

use csv::ErrorKind;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    let qubits = str::parse::<usize>(&args[1])?;
    assert!(qubits > 0);
    let measurements = benchmark::random::random_measurements(qubits);

    let mut builder = csv::WriterBuilder::new();
    builder.has_headers(false);
    let mut writer = builder.from_writer(io::stdout());
    for measurement in measurements {
        let mut out = vec![String::from("m")];
        out.push(
            measurement
                .basis()
                .iter()
                .map(|ps| ps.to_string())
                .collect::<Vec<_>>()
                .join(""),
        );
        out.push(String::from("+"));

        // If failed to write, just quit.
        // Could happen when pipe was closed (e.g. head -n 10)
        if let Err(e) = writer.write_record(out) {
            if e.is_io_error() {
                break;
            } else {
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}

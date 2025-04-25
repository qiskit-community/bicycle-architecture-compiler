use std::{env, error, io};

use gross_code_cliffords::{
    native_measurement::NativeMeasurement, MeasurementChoices, MeasurementTableBuilder,
};
use pbc_gross::language::{AnglePrecision, PbcOperation};

use io::Write;

use clap::Parser;
use log::debug;
use pbc_gross::{optimize, PathArchitecture};
use serde_json::Deserializer;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Cli {
    #[arg(short, long, default_value_t = AnglePrecision::lit("1e-9"))]
    accuracy: AnglePrecision,
    #[arg(short, long)]
    code: MeasurementChoices,
}

fn main() -> Result<(), Box<dyn error::Error>> {
    // By default log INFO.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    let mut builder =
        MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
    builder.build();
    let measurement_table = builder.complete()?;

    let reader = io::stdin().lock();

    // Support some streaming input from Stdin
    // The following works for (a weird version of) JSON:
    let de = Deserializer::from_reader(reader);
    let ops = de.into_iter::<PbcOperation>().map(|op| op.unwrap());
    let mut ops = ops.peekable();

    // Set the architecture based on the first operation
    let first_op = ops.peek();
    let architecture = if let Some(op) = first_op {
        PathArchitecture::for_qubits(op.basis().len())
    } else {
        // No ops, may as well terminate now.
        return Ok(());
    };

    let compiled = ops.map(|op| op.compile(&architecture, &measurement_table, cli.accuracy));

    let mut optimized_chunked_ops = optimize::remove_duplicate_measurements_chunked(compiled);
    let mut stdout = io::stdout();
    // Stop on first error
    let err: Result<(), io::Error> = optimized_chunked_ops.try_for_each(|chunk| {
        let out = serde_json::to_string(&chunk)?;
        writeln!(stdout, "{}", out)
    });
    debug!("Encountered error while writing to stdout: {:?}", err);

    Ok(())
}

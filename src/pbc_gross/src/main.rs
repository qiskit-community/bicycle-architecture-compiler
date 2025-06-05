use std::{env, error, fs::File, io, path::Path};

use gross_code_cliffords::{
    native_measurement::NativeMeasurement, CompleteMeasurementTable, MeasurementChoices,
    MeasurementTableBuilder,
};
use pbc_gross::language::{AnglePrecision, PbcOperation};

use io::Write;

use clap::{Parser, Subcommand};
use log::{debug, info};
use pbc_gross::{optimize, PathArchitecture};
use serde_json::Deserializer;

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Cli {
    code: MeasurementChoices,
    #[command(subcommand)]
    commands: Option<Commands>,
    #[arg(long)]
    measurement_table: Option<String>,
    #[arg(short, long, default_value_t = AnglePrecision::lit("1e-9"))]
    accuracy: AnglePrecision,
}

/// Caching commands
#[derive(Subcommand, Clone, PartialEq, Eq)]
enum Commands {
    Generate { measurement_table: String },
}

fn main() -> Result<(), Box<dyn error::Error>> {
    // By default log INFO.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    if let Some(Commands::Generate {
        measurement_table: cache_str,
    }) = cli.commands
    {
        info!("Generating measurement table, then exiting.");
        let cache_path = Path::new(&cache_str);
        let mut f =
            File::create(cache_path).expect("Should be able to open the measurement_table file");

        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
        builder.build();
        let measurement_table = builder.complete()?;
        let serialized =
            bitcode::serialize(&measurement_table).expect("The table should be serializable");
        f.write_all(&serialized)
            .expect("The serialized table should be writable to the cache");
        std::process::exit(1);
    }

    // Generate measurement table, from cache if given or otherwise from scratch
    let measurement_table = if let Some(cache_str) = cli.measurement_table {
        let cache_path = Path::new(&cache_str);
        let read =
            std::fs::read(cache_path).expect("The measurement table file should be readable");
        bitcode::deserialize::<CompleteMeasurementTable>(&read)?
    } else {
        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
        builder.build();
        builder.complete()?
    };

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

    let optimized_auts = compiled.map(optimize::remove_trivial_automorphisms);
    let mut optimized_chunked_ops = optimize::remove_duplicate_measurements_chunked(optimized_auts);
    let mut stdout = io::stdout();
    // Stop on first error
    let err: Result<(), io::Error> = optimized_chunked_ops.try_for_each(|chunk| {
        let out = serde_json::to_string(&chunk)?;
        writeln!(stdout, "{}", out)
    });
    debug!("Encountered error while writing to stdout: {:?}", err);

    Ok(())
}

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

// Could use the following to implement some kind of caching of measurement table.

// Statically store a database to look up measurement implementations on the gross code
// by sequences of native measurements.
// Access is read-only and thread safe
// static ABC: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
//     caching_logic()
//         .expect("(De)serializing and/or generating a new measurement table should succeed")
// });

// fn caching_logic() -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     let path = Path::new("tmp/measurement_table");
//     try_deserialize(path).or_else(|_| try_create_cache(path))
// }

// fn try_deserialize(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     debug!("Attempting to deserialize measurement table");
//     let read = std::fs::read(path)?;
//     let table = bitcode::deserialize::<CompleteMeasurementTable>(&read)?;
//     Ok(table)
// }

// fn try_create_cache(path: &Path) -> Result<CompleteMeasurementTable, Box<dyn Error>> {
//     // Generate new cache file
//     info!("Could not deserialize measurement table. Generating new table. This may take a while");
//     let parent_path = path.parent().ok_or("Parent path does not exist")?;
//     std::fs::create_dir_all(parent_path)?;
//     let mut f = File::create(path).expect("Should be able to open the measurement_table file");
//     let native_measurements = NativeMeasurement::all();
//     let mut table = MeasurementTableBuilder::new(native_measurements, GROSS_MEASUREMENT);
//     table.build();
//     let table = table
//         .complete()
//         .expect("The measurement table should be complete");
//     let serialized = bitcode::serialize(&table).expect("The table should be serializable");
//     f.write_all(&serialized)
//         .expect("The serialized table should be writable to the cache");
//     Ok(table)
// }

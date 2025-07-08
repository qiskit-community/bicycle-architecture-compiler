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
    env, error,
    fs::File,
    io,
    path::{Path, PathBuf},
};

use bicycle_cliffords::{
    native_measurement::NativeMeasurement, CompleteMeasurementTable, MeasurementChoices,
    MeasurementTableBuilder,
};
use bicycle_compiler::language::{AnglePrecision, PbcOperation};

use io::Write;

use bicycle_compiler::{optimize, PathArchitecture};
use clap::{Parser, Subcommand};
use log::{debug, info};
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
        info!("Generating measurement table.");
        let cache_path = Path::new(&cache_str);

        // Ensure that we can write a file in the desired output directory.  To do this we
        // write and delte an empty file in the parent directory of the full path of the
        // (output) cache file.  We do this in order to fail early rather than computing the
        // measurement table, only to find at the end that we cannot write the result.
        match cache_path.parent() {
            Some(cache_dir) => {
                let temp_filename = "dummy_file_check";
                let mut temp_file_path = PathBuf::from(cache_dir);
                temp_file_path.push(temp_filename);
                match File::create(&temp_file_path) {
                    Ok(_) => {
                        // Successfully created dummy file. Remove file.
                        std::fs::remove_file(temp_file_path)?;
                    }
                    Err(e) => {
                        eprintln!(
                            "Cannot create measurement_table output file in the target directory: {}",
                            e
                        );
                        std::process::exit(1);
                    }
                }
            }
            None => {
                eprintln!("No parent directory found for {}", cache_str);
                std::process::exit(1);
            }
        }

        // Create a builder and build the measurement table.
        let mut builder =
            MeasurementTableBuilder::new(NativeMeasurement::all(), cli.code.measurement());
        builder.build();
        let measurement_table = builder.complete()?;

        // Serialize the measurement table and write to the cache file.
        let serialized =
            bitcode::serialize(&measurement_table).expect("The table should be serializable");
        info!("Done generating measurement table, writing.");
        let f = File::create(cache_path);
        match f {
            Ok(mut f) => {
                f.write_all(&serialized)
                    .expect("The serialized table should be writable to the cache");
            }
            Err(e) => {
                eprintln!(
                    "Cannot create  measurement_table output file in the target directory: {}",
                    e
                );
                std::process::exit(1);
            }
        }
        info!("Done writing measurement table, exiting.");
        std::process::exit(0);
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

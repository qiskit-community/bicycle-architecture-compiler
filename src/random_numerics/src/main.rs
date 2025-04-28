use std::{env, error::Error, io};

use bicycle_isa::{BicycleISA, Pauli, TwoBases};
use fixed::traits::LosslessTryInto;
use gross_code_cliffords::{
    native_measurement::NativeMeasurement, MeasurementChoices, MeasurementTableBuilder,
};
use log::{debug, trace};
use numerics::{
    model::{ErrorPrecision, GROSS_1E3, GROSS_1E4, TWO_GROSS_1E3, TWO_GROSS_1E4},
    OutputData,
};

use clap::{Parser, ValueEnum};
use pbc_gross::language::AnglePrecision;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Output {
    code: String,
    p: f64,
    i: usize,
    qubits: usize,
    t_injs: u64,
    automorphisms: u64,
    measurements: u64,
    joint_measurements: u64,
    measurement_depth: u64,
    end_time: u64,
    total_error: f64,
}

impl Output {
    pub fn new(model: MeasurementChoices, error: ErrorRate, data: OutputData) -> Self {
        let code = format!("{}", model);
        let p: f64 = error.into();

        Self {
            code,
            p,
            i: data.i,
            qubits: data.qubits,
            t_injs: data.t_injs,
            automorphisms: data.automorphisms,
            measurements: data.measurements,
            joint_measurements: data.joint_measurements,
            measurement_depth: data.measurement_depth,
            end_time: data.end_time,
            total_error: data.total_error,
        }
    }
}

#[derive(Debug, ValueEnum, Clone, Copy, Eq, PartialEq)]
enum ErrorRate {
    #[clap(name = "1e-3")]
    E3,
    #[clap(name = "1e-4")]
    E4,
}

impl From<ErrorRate> for f64 {
    fn from(value: ErrorRate) -> Self {
        match value {
            ErrorRate::E3 => 1e-3,
            ErrorRate::E4 => 1e-4,
        }
    }
}

#[derive(Parser, Debug)]
struct Cli {
    #[arg(short, long)]
    qubits: usize,
    #[arg(short, long)]
    model: MeasurementChoices,
    #[arg(short, long)]
    noise: ErrorRate,
    #[arg(short = 'e', long, default_value_t = 1.0/3.0)]
    max_error: f64,
    #[arg(short = 'i', long, default_value_t = 10_usize.pow(6))]
    max_iter: usize,
    #[arg(short, long)]
    accuracy: Option<AnglePrecision>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // By default log INFO.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    trace!("Cli arguments: {:?}", cli);
    let model = match (cli.model, cli.noise) {
        (MeasurementChoices::Gross, ErrorRate::E3) => GROSS_1E3,
        (MeasurementChoices::Gross, ErrorRate::E4) => GROSS_1E4,
        (MeasurementChoices::TwoGross, ErrorRate::E3) => TWO_GROSS_1E3,
        (MeasurementChoices::TwoGross, ErrorRate::E4) => TWO_GROSS_1E4,
    };

    // Set the small-angle synthesis accuracy to same order of magnitude as in-module measurement.
    let measurement_error: ErrorPrecision = model.instruction_error(&BicycleISA::Measure(
        TwoBases::new(Pauli::X, Pauli::Z).unwrap(),
    ));
    let unsigned_measurement_error: AnglePrecision = measurement_error.lossless_try_into().unwrap();
    let angle_precision: AnglePrecision = cli.accuracy.unwrap_or(unsigned_measurement_error);
    debug!("Set angle precision: {angle_precision:?}");

    let random_ops = benchmark::random::random_measurements(cli.qubits);

    let mut builder =
        MeasurementTableBuilder::new(NativeMeasurement::all(), cli.model.measurement());
    builder.build();
    let measurement_table = builder.complete()?;

    let architecture = pbc_gross::PathArchitecture::for_qubits(cli.qubits);
    let compiled =
        random_ops.map(|op| op.compile(&architecture, &measurement_table, angle_precision));
    let optimized_chunked_ops =
        pbc_gross::optimize::remove_duplicate_measurements_chunked(compiled);

    let output_data = numerics::run_numerics(optimized_chunked_ops, architecture, model);

    // Stop when error exceeds 1/3 or iterations gets too large
    let short_data = output_data
        // Output at least one line.
        .take_while(|data| {
            data.i == 1 || (data.total_error <= cli.max_error && data.i <= cli.max_iter)
        });

    let mut outputs = short_data.map(|data| Output::new(cli.model, cli.noise, data));
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let err = outputs.try_for_each(|output| wtr.serialize(output));
    debug!("Exited with {:?}", err);

    Ok(())
}

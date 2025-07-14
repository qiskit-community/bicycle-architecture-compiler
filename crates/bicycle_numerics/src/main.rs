// Copyright contributors to the Bicycle Architecture Compiler project

use std::{env, error::Error, io};

use bicycle_numerics::{
    model::{Model, FAKE_SLOW, GROSS_1E3, GROSS_1E4, TWO_GROSS_1E3, TWO_GROSS_1E4},
    OutputData,
};
use log::{debug, trace};

use bicycle_compiler::operation::Operation;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
enum ModelChoices {
    #[clap(name = "gross_1e-3")]
    Gross1e3,
    #[clap(name = "gross_1e-4")]
    Gross1e4,
    #[clap(name = "two-gross_1e-3")]
    TwoGross1e3,
    #[clap(name = "two-gross_1e-4")]
    TwoGross1e4,
    #[clap(name = "fake_slow")]
    FakeSlow,
}

impl ModelChoices {
    fn model(self) -> Model {
        match self {
            Self::Gross1e3 => GROSS_1E3,
            Self::Gross1e4 => GROSS_1E4,
            Self::TwoGross1e3 => TWO_GROSS_1E3,
            Self::TwoGross1e4 => TWO_GROSS_1E4,
            Self::FakeSlow => FAKE_SLOW,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct Output {
    code: &'static str,
    p: f64,
    i: usize,
    qubits: usize,
    idles: u64,
    t_injs: u64,
    automorphisms: u64,
    measurements: u64,
    joint_measurements: u64,
    measurement_depth: u64,
    end_time: u64,
    total_error: f64,
}

impl Output {
    pub fn new(model: ModelChoices, data: OutputData) -> Self {
        let (code, p) = match model {
            ModelChoices::Gross1e3 => ("gross", 1e-3),
            ModelChoices::Gross1e4 => ("gross", 1e-4),
            ModelChoices::TwoGross1e3 => ("two-gross", 1e-3),
            ModelChoices::TwoGross1e4 => ("two-gross", 1e-4),
            ModelChoices::FakeSlow => ("fake", 0.0),
        };

        Self {
            code,
            p,
            i: data.i,
            qubits: data.qubits,
            idles: data.idles,
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

#[derive(Parser, Debug)]
struct Cli {
    qubits: usize,
    model: ModelChoices,
    #[arg(short = 'e', long, default_value_t = 1.0/3.0)]
    max_error: f64,
    #[arg(short = 'i', long, default_value_t = 10_usize.pow(6))]
    max_iter: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    // By default log INFO.
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    trace!("Number of qubits: {}", cli.qubits);
    let model = cli.model.model();

    let reader = io::stdin().lock();

    // Support some streaming input from Stdin
    // The following works for (a weird version of) JSON:
    let de = Deserializer::from_reader(reader);
    let ops = de.into_iter::<Vec<Operation>>().map(|op| op.unwrap());

    let architecture = bicycle_compiler::PathArchitecture::for_qubits(cli.qubits);

    let output_data = bicycle_numerics::run_numerics(ops, architecture, model);

    // Stop when error exceeds 1/3 or iterations gets too large
    let short_data = output_data
        // Output at least one line.
        .take_while(|data| {
            data.i == 1 || (data.total_error <= cli.max_error && data.i <= cli.max_iter)
        });

    let mut outputs = short_data.map(|data| Output::new(cli.model, data));
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let err = outputs.try_for_each(|output| wtr.serialize(output));
    debug!("Exited with {:?}", err);

    Ok(())
}

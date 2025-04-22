use std::{env, error::Error, io};

use bicycle_isa::BicycleISA;
use clap::Parser;
use log::{debug, trace};
use model::{Model, ModelChoices};
use pbc_gross::{operation::Operation, PathArchitecture};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

pub mod model;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IsaCounter {
    t_injs: u64,
    automorphisms: u64,
    measurements: u64,
    joint_measurements: u64,
}

impl IsaCounter {
    fn add(&mut self, instr: &BicycleISA) {
        match instr {
            BicycleISA::TGate(_) => {
                self.t_injs += 1;
                self.measurements += 1;
            }
            BicycleISA::Automorphism(_) => self.automorphisms += 1,
            BicycleISA::Measure(_) => self.measurements += 1,
            BicycleISA::JointMeasure(_) => self.joint_measurements += 1,
            _ => unreachable!("There should not be any other instructions, {}", instr),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct OutputData {
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

fn numerics(
    chunked_ops: impl Iterator<Item = Vec<Operation>>,
    architecture: PathArchitecture,
    model: Model,
) -> impl Iterator<Item = OutputData> {
    let data_blocks = architecture.data_blocks();
    let qubits = architecture.qubits();

    let mut depths: Vec<u64> = vec![0; data_blocks];
    let mut times: Vec<u64> = vec![0; data_blocks];
    let mut total_error = model::ErrorPrecision::ZERO;
    chunked_ops.enumerate().map(move |(i, ops)| {
        let mut counter: IsaCounter = Default::default();
        // Accumulate counts. Or use a fold.
        ops.iter().for_each(|instr| counter.add(&instr[0].1));

        // Compute the new depths and timing for each block
        for op in ops {
            // Find the max depth/time between blocks
            let mut max_depth = 0;
            let mut max_time = 0;
            for (block_i, _) in op.iter() {
                max_depth = max_depth.max(depths[*block_i]);
                max_time = max_time.max(times[*block_i]);
            }

            for (block_i, instr) in op.iter() {
                depths[*block_i] = max_depth;
                match instr {
                    BicycleISA::Measure(_) | BicycleISA::JointMeasure(_) => {
                        depths[*block_i] = max_depth + 1
                    }
                    _ => depths[*block_i] = max_depth,
                }

                // Insert idling noise
                let time_diff = max_time - times[*block_i];
                total_error += model.idling_error(time_diff);

                times[*block_i] = max_time + model.timing(instr);
            }

            // Update error rate once per op
            let (_, instr) = &op[0];
            total_error += model.instruction_error(instr);
        }

        // Calculate the max depth currently
        let measurement_depth = depths.iter().max().unwrap();
        let end_time = times.iter().max().unwrap();

        OutputData {
            i: i + 1,
            qubits,
            t_injs: counter.t_injs,
            automorphisms: counter.automorphisms,
            measurements: counter.measurements,
            joint_measurements: counter.joint_measurements,
            measurement_depth: *measurement_depth,
            end_time: *end_time,
            total_error: total_error.to_num(),
        }
    })
}

#[derive(Parser, Debug)]
struct Cli {
    qubits: usize,
    model: ModelChoices,
    #[arg(short = 'e',long,default_value_t = 1.0/3.0)]
    max_error: f64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct Output {
    code: &'static str,
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
    pub fn new(model: ModelChoices, data: OutputData) -> Self {
        let (code, p) = match model {
            ModelChoices::Gross1e3 => ("gross", 1e-3),
            ModelChoices::Gross1e4 => ("gross", 1e-4),
            ModelChoices::TwoGross1e3 => ("two-gross", 1e-3),
            ModelChoices::TwoGross1e4 => ("two-gross", 1e-4),
        };

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

    let architecture = pbc_gross::PathArchitecture::for_qubits(cli.qubits);

    let output_data = numerics(ops, architecture, model);

    // Stop when error exceeds 1/3 or iterations gets too large
    let max_error = 1. / 3.;
    let max_iter = 10_usize.pow(6);
    let short_data =
        output_data.take_while(|data| data.total_error <= max_error && data.i <= max_iter);

    let mut outputs = short_data.map(|data| Output::new(cli.model, data));
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let err = outputs.try_for_each(|output| wtr.serialize(output));
    debug!("Exited with {:?}", err);

    Ok(())
}

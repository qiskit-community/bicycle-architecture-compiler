use std::{env, error::Error, f64::consts::PI, io};

use bicycle_isa::BicycleISA;
use clap::Parser;
use log::{info, trace};
use model::{Model, ModelChoices};
use pbc_gross::{operation::Operation, PathArchitecture};
use serde_json::Deserializer;

pub mod model;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IsaCounter {
    t_injs: usize,
    automorphisms: usize,
    measurements: usize,
    joint_measurements: usize,
}

impl IsaCounter {
    fn add(&mut self, instr: &BicycleISA) {
        match instr {
            BicycleISA::TGate(_) => self.t_injs += 1,
            BicycleISA::Automorphism(_) => self.automorphisms += 1,
            BicycleISA::Measure(_) => self.measurements += 1,
            BicycleISA::JointMeasure(_) => self.joint_measurements += 1,
            _ => unreachable!("There should not be any other instructions, {}", instr),
        }
    }

    fn max(self, other: IsaCounter) -> Self {
        Self {
            t_injs: self.t_injs.max(other.t_injs),
            automorphisms: self.automorphisms.max(other.automorphisms),
            measurements: self.measurements.max(other.measurements),
            joint_measurements: self.joint_measurements.max(other.joint_measurements),
        }
    }
}

fn numerics(
    mut chunked_ops: impl Iterator<Item = Vec<Operation>>,
    architecture: PathArchitecture,
    model: Model,
) {
    println!(
        "i,qubits,blocks,rotations,automorphisms,measurements,joint measurements,cumulative measurement depth,syndrome time,error rate"
    );
    let data_blocks = architecture.data_blocks();
    let qubits = architecture.qubits();

    let mut depths: Vec<u64> = vec![0; data_blocks];
    let mut times: Vec<u64> = vec![0; data_blocks];
    let mut total_error = model::ErrorPrecision::ZERO;
    let mut i = 0;
    let max_loops = 10_i64.pow(7);
    while total_error <= 1.0 / 3.0 && i <= max_loops {
        let ops = chunked_ops.next().unwrap();
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

        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            i + 1,
            qubits,
            data_blocks,
            counter.t_injs,
            counter.automorphisms,
            counter.measurements,
            counter.joint_measurements,
            measurement_depth,
            end_time,
            total_error,
        );

        trace!("{total_error}");

        i += 1;
    }
    if i >= max_loops {
        info!("Max iterations reached. Params: {:?}, {}", model, qubits);
    }
}

#[derive(Parser, Debug)]
struct Cli {
    qubits: usize,
    model: ModelChoices,
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

    numerics(ops, architecture, model);
    Ok(())
}

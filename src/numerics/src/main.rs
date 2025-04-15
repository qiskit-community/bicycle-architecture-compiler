use std::error::Error;

use clap::Parser;
use log::trace;
use model::{Model, ModelChoices};
use pbc_gross::{
    operation::{Instruction, Operation},
    PathArchitecture,
};

pub mod model;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct InstructionCounter {
    rotations: usize,
    automorphisms: usize,
    measurements: usize,
    joint_measurements: usize,
}

impl InstructionCounter {
    fn add(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Rotation(_) => self.rotations += 1,
            Instruction::Automorphism(_) => self.automorphisms += 1,
            Instruction::Measure(_) => self.measurements += 1,
            Instruction::JointMeasure(_) => self.joint_measurements += 1,
        }
    }

    fn max(self, other: InstructionCounter) -> Self {
        Self {
            rotations: self.rotations.max(other.rotations),
            automorphisms: self.automorphisms.max(other.automorphisms),
            measurements: self.measurements.max(other.measurements),
            joint_measurements: self.joint_measurements.max(other.joint_measurements),
        }
    }
}

fn numerics(
    mut operations: impl Iterator<Item = Vec<Operation>>,
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
    while total_error <= 0.5 {
        let meas_impl = operations.next().unwrap();
        let mut counter: InstructionCounter = Default::default();
        // Accumulate counts. Or use a fold.
        meas_impl.iter().for_each(|instr| counter.add(&instr[0].1));

        // Compute the new depths and timing for each block
        for op in meas_impl {
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
                    Instruction::Measure(_) | Instruction::JointMeasure(_) => {
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
        let measurement_depth = depths.iter().reduce(|a, b| a.max(b)).unwrap();
        let end_time = times.iter().reduce(|maxt, t| maxt.max(t)).unwrap();

        println!(
            "{},{},{},{},{},{},{},{},{},{}",
            i + 1,
            qubits,
            data_blocks,
            counter.rotations,
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
}

#[derive(Parser)]
struct Cli {
    qubits: usize,
    model: ModelChoices,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();
    trace!("Number of qubits: {}", cli.qubits);
    let model = cli.model.model();
    let random_circuit = benchmark::random::random_rotations(cli.qubits, 0.123);

    let architecture = pbc_gross::PathArchitecture::for_qubits(cli.qubits);

    let compiled_measurements = random_circuit.map(|meas| meas.compile(&architecture));
    let optimized_measurements =
        pbc_gross::optimize::remove_duplicate_measurements_chunked(compiled_measurements);

    numerics(optimized_measurements, architecture, model);
    Ok(())
}

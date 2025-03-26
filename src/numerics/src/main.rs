use std::error::Error;

use log::trace;
use pbc_gross::operation::Instruction;

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

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let args: Vec<_> = std::env::args().collect();
    let qubits = str::parse::<usize>(&args[1])?;
    trace!("Number of qubits: {qubits}");
    let random_circuit = benchmark::random::random_measurements(qubits);

    let architecture = pbc_gross::PathArchitecture::for_qubits(qubits);

    let compiled_measurements = random_circuit.map(|meas| meas.compile(&architecture));
    let optimized_measurements =
        pbc_gross::optimize::remove_duplicate_measurements_chunked(compiled_measurements);

    let data_blocks = architecture.data_blocks();
    println!(
        "i,qubits,blocks,rotations,automorphisms,measurements,joint measurements,cumulative measurement depth"
    );

    let mut depths: Vec<u64> = vec![Default::default(); data_blocks];
    for (i, meas_impl) in optimized_measurements.enumerate() {
        let mut counter: InstructionCounter = Default::default();
        // Accumulate counts. Or use a fold.
        meas_impl.iter().for_each(|instr| counter.add(&instr[0].1));

        // Compute the new depths for each block
        for op in meas_impl {
            // Update the InstructionCounter of the involved blocks to be the max
            if op.len() > 1 {
                let mut max_counter: u64 = Default::default();
                for (block_i, _) in op.iter() {
                    max_counter = max_counter.max(depths[*block_i]);
                }
                for (block_i, _) in op.iter() {
                    depths[*block_i] = max_counter;
                }
            }

            // Add the instruction to each block
            for (block_i, instr) in op {
                match instr {
                    Instruction::Measure(_) | Instruction::JointMeasure(_) => depths[block_i] += 1,
                    _ => {}
                }
            }
        }

        // Calculate the max depth currently
        let measurement_depth = depths.iter().reduce(|a, b| a.max(b)).unwrap();

        println!(
            "{},{},{},{},{},{},{},{}",
            i + 1,
            qubits,
            data_blocks,
            counter.rotations,
            counter.automorphisms,
            counter.measurements,
            counter.joint_measurements,
            measurement_depth,
        );
    }

    Ok(())
}

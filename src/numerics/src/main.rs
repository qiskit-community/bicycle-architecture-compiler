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
    println!("i,qubits,blocks,rotations,automorphisms,measurements,joint measurements");

    for (i, meas_impl) in optimized_measurements.enumerate() {
        let mut counter: InstructionCounter = Default::default();
        // Accumulate counts. Or use a fold.
        meas_impl.iter().for_each(|instr| counter.add(&instr[0].1));
        println!(
            "{},{},{},{},{},{},{}",
            i + 1,
            qubits,
            data_blocks,
            counter.rotations,
            counter.automorphisms,
            counter.measurements,
            counter.joint_measurements,
        );
    }

    Ok(())
}

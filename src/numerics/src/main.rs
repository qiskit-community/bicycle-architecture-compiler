use std::error::Error;

use fixed::types::U32F96;
use log::trace;
use pbc_gross::operation::Instruction;

// Because we need to support precision up to 10^-20,
// which is >2^-65
type ErrorPrecision = U32F96;

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
        "i,qubits,blocks,rotations,automorphisms,measurements,joint measurements,cumulative measurement depth,syndrome time,error rate"
    );

    let mut depths: Vec<u64> = vec![0; data_blocks];
    let mut times: Vec<u64> = vec![0; data_blocks];
    let mut total_error = ErrorPrecision::ZERO;
    for (i, meas_impl) in optimized_measurements.enumerate() {
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
                // Only add if diff > 0 due to float rounding
                // Not sure if necessary
                if time_diff != 0 {
                    total_error += GROSS_10E3.idling_error(time_diff);
                }

                times[*block_i] = max_time + timing(instr);
            }

            // Update error rate once per op
            let (_, instr) = &op[0];
            total_error += GROSS_10E3.instruction_error(instr);
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
    }

    Ok(())
}

/// Time it takes to perform an instruction
pub fn timing(instruction: &Instruction) -> u64 {
    match instruction {
        Instruction::Rotation(_) => 30,
        Instruction::Automorphism(_) => 3,
        Instruction::Measure(_) => 10,
        Instruction::JointMeasure(_) => 10,
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ErrorModel {
    idle: ErrorPrecision,
    shift: ErrorPrecision,
    inmodule: ErrorPrecision,
    intermodule: ErrorPrecision,
    t_inj: ErrorPrecision,
}

impl ErrorModel {
    pub fn instruction_error(&self, instruction: &Instruction) -> ErrorPrecision {
        match instruction {
            Instruction::Rotation(_) => self.t_inj,
            Instruction::Measure(_) => self.inmodule,
            Instruction::JointMeasure(_) => self.intermodule,
            Instruction::Automorphism(_) => self.shift,
        }
    }

    pub fn idling_error(&self, cycles: u64) -> ErrorPrecision {
        (cycles.div_ceil(8) as u128) * self.idle
    }
}

const GROSS_10E3: ErrorModel = ErrorModel {
    idle: ErrorPrecision::lit("1e-6"),
    shift: ErrorPrecision::lit("1e-5"),
    inmodule: ErrorPrecision::lit("1e-4"),
    intermodule: ErrorPrecision::lit("1e-4"),
    t_inj: ErrorPrecision::lit("1e-4"),
};

const GROSS_10E4: ErrorModel = ErrorModel {
    idle: ErrorPrecision::lit("1e-11"),
    shift: ErrorPrecision::lit("1e-10"),
    inmodule: ErrorPrecision::lit("1e-9"),
    intermodule: ErrorPrecision::lit("1e-9"),
    t_inj: ErrorPrecision::lit("1e-9"),
};

const TWO_GROSS_10E3: ErrorModel = ErrorModel {
    idle: ErrorPrecision::lit("1e-11"),
    shift: ErrorPrecision::lit("1e-10"),
    inmodule: ErrorPrecision::lit("1e-9"),
    intermodule: ErrorPrecision::lit("1e-9"),
    t_inj: ErrorPrecision::lit("1e-10"),
};

const TWO_GROSS_10E4: ErrorModel = ErrorModel {
    idle: ErrorPrecision::lit("1e-20"),
    shift: ErrorPrecision::lit("1e-19"),
    inmodule: ErrorPrecision::lit("1e-18"),
    intermodule: ErrorPrecision::lit("1e-18"),
    t_inj: ErrorPrecision::lit("1e-18"),
};

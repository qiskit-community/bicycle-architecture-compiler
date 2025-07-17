// Copyright contributors to the Bicycle Architecture Compiler project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bicycle_common::BicycleISA;

use bicycle_compiler::{operation::Operation, PathArchitecture};
use log::trace;
use model::Model;
use serde::{Deserialize, Serialize};

pub mod model;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IsaCounter {
    pub idles: u64,
    pub t_injs: u64,
    pub automorphisms: u64,
    pub measurements: u64,
    pub joint_measurements: u64,
}

impl IsaCounter {
    fn add(&mut self, instr: &BicycleISA) {
        trace!("Adding: {}", instr);
        match instr {
            BicycleISA::TGate(_) => self.t_injs += 1,
            BicycleISA::Automorphism(autdata) => self.automorphisms += autdata.nr_generators(),
            BicycleISA::Measure(_) => self.measurements += 1,
            BicycleISA::JointMeasure(_) => self.joint_measurements += 1,
            _ => unreachable!("There should not be any other instructions, {}", instr),
        }
        trace!("Now at: {:?}", &self);
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct OutputData {
    pub i: usize,
    pub qubits: usize,
    pub idles: u64,
    pub t_injs: u64,
    pub automorphisms: u64,
    pub measurements: u64,
    pub joint_measurements: u64,
    pub measurement_depth: u64,
    pub end_time: u64,
    pub total_error: f64,
}

pub fn run_numerics(
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
        trace!("Ops: {:?}", ops);
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
                let (idle_cycles, idle_error) = model.idling_error(time_diff);
                counter.idles += idle_cycles;
                total_error += idle_error;

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
            idles: counter.idles,
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

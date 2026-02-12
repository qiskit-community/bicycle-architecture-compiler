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

use std::hint::black_box;
use std::time::Instant;

use bicycle_common::parity_check::{
    ErrorSource, gross_toric_parity_checks, simulate_syndrome_once,
};

fn main() {
    let checks = gross_toric_parity_checks();
    let shots: u64 = 10_000;
    let p = 1e-3;

    println!("=== Syndrome Throughput Benchmark ===");
    println!("code: gross toric");
    println!("shots: {shots}");
    println!("bernoulli p: {p}");

    let start = Instant::now();
    let mut checksum: u64 = 0;

    for i in 0..shots {
        let shot = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Bernoulli {
                p,
                seed: i ^ 0xA5A5_5A5A,
            },
            ErrorSource::Bernoulli {
                p,
                seed: i ^ 0x5A5A_A5A5,
            },
        )
        .expect("benchmark inputs should be valid");

        checksum ^= shot.syndrome_x.iter().map(|&b| b as u64).sum::<u64>();
        checksum ^= shot.syndrome_z.iter().map(|&b| b as u64).sum::<u64>();
    }

    let elapsed = start.elapsed();
    let per_shot = elapsed / shots as u32;

    println!("total: {:.2?}", elapsed);
    println!("per shot: {per_shot:.2?}");
    println!("checksum: {}", black_box(checksum));
}

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

//! Benchmark for the Clifford measurement-table build.
//!
//! The table build is the dominant cost of compiler startup (~87 s in
//! release mode on a 2024 laptop).  This benchmark breaks the build
//! into its three phases so that optimisations can be measured:
//!
//! 1. **Init** – seed the table with 540 native measurements.
//! 2. **Build** – BFS search over Pauli conjugations (the hot path).
//! 3. **Complete** – convert the builder into the final lookup table.
//!
//! Run with:
//!
//! ```sh
//! cargo bench --package bicycle_cliffords --bench bench_table_build
//! ```
//!
//! To benchmark only one code (faster iteration while optimising):
//!
//! ```sh
//! cargo bench --package bicycle_cliffords --bench bench_table_build -- --code gross
//! ```

use std::time::Instant;

use bicycle_cliffords::{
    CodeMeasurement, GROSS_MEASUREMENT, MeasurementTableBuilder, TWOGROSS_MEASUREMENT,
    native_measurement::NativeMeasurement,
};

fn profile_table_build(name: &str, code: CodeMeasurement) {
    println!("--- {name} ---");

    // Phase 1: Init
    let t0 = Instant::now();
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), code);
    let init = t0.elapsed();
    println!(
        "  init:     {:>10.3?}  ({} entries seeded)",
        init,
        builder.len()
    );

    // Phase 2: Build (BFS search – the bottleneck)
    let t1 = Instant::now();
    builder.build();
    let build = t1.elapsed();
    println!(
        "  build:    {:>10.3?}  ({} total entries)",
        build,
        builder.len()
    );

    // Phase 3: Convert to CompleteMeasurementTable
    let t2 = Instant::now();
    let _table = builder.complete().expect("Table building should succeed");
    let complete = t2.elapsed();
    println!("  complete: {:>10.3?}", complete);

    let total = t0.elapsed();
    println!("  TOTAL:    {:>10.3?}", total);
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code_filter = args
        .windows(2)
        .find(|w| w[0] == "--code")
        .map(|w| w[1].as_str());

    println!("=== Clifford Measurement Table Build Benchmark ===");
    println!("  table size: 4^12 = 16,777,216 entries");
    println!();

    match code_filter {
        Some("gross") => {
            profile_table_build("gross", GROSS_MEASUREMENT);
        }
        Some("two-gross") => {
            profile_table_build("two-gross", TWOGROSS_MEASUREMENT);
        }
        _ => {
            profile_table_build("gross", GROSS_MEASUREMENT);
            profile_table_build("two-gross", TWOGROSS_MEASUREMENT);
        }
    }
}

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

//! Benchmarks for the compilation pipeline.
//!
//! Measures the end-to-end cost of compiling PBC operations (measurements
//! and Clifford rotations) to bicycle ISA instructions.  Only
//! measurement compilation is benchmarked here because rotation
//! compilation delegates to the external `gridsynth` binary which is not
//! available in all environments.
//!
//! Run with:
//!
//! ```sh
//! cargo bench --package bicycle_compiler --bench bench_compile
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use bicycle_cliffords::{
    CompleteMeasurementTable, GROSS_MEASUREMENT, MeasurementTableBuilder,
    native_measurement::NativeMeasurement,
};
use bicycle_common::Pauli;
use bicycle_compiler::PathArchitecture;
use bicycle_compiler::language::PbcOperation;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_gross_table() -> CompleteMeasurementTable {
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    builder.build();
    builder.complete().expect("Table should build successfully")
}

/// Run `f` for at least `min_duration` and report per-iteration average.
fn bench<F: FnMut()>(label: &str, iters_per_batch: u64, min_duration: Duration, mut f: F) {
    // Warm-up
    for _ in 0..iters_per_batch.min(5) {
        f();
    }

    let mut total_iters: u64 = 0;
    let start = Instant::now();
    while start.elapsed() < min_duration {
        for _ in 0..iters_per_batch {
            f();
        }
        total_iters += iters_per_batch;
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / total_iters as u32;
    println!("  {label:<50} {per_iter:>10.2?}/iter  ({total_iters} iters in {elapsed:.2?})");
}

// ---------------------------------------------------------------------------
// Test circuits
// ---------------------------------------------------------------------------

/// 1-block measurement: single X on qubit 1, rest identity.
fn single_block_measurement() -> PbcOperation {
    let mut basis = vec![Pauli::I; 11];
    basis[0] = Pauli::X;
    basis[1] = Pauli::Z;
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

/// 2-block measurement: non-trivial Paulis on both 11-qubit modules.
fn two_block_measurement() -> PbcOperation {
    let mut basis = vec![Pauli::I; 22];
    basis[0] = Pauli::X;
    basis[1] = Pauli::Z;
    // Second block
    basis[11] = Pauli::Z;
    basis[12] = Pauli::X;
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

/// 3-block measurement: non-trivial Paulis across all three modules.
fn three_block_measurement() -> PbcOperation {
    let mut basis = vec![Pauli::I; 33];
    basis[0] = Pauli::X;
    basis[1] = Pauli::Y;
    basis[11] = Pauli::Z;
    basis[12] = Pauli::X;
    basis[22] = Pauli::Z;
    basis[23] = Pauli::Y;
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

/// Dense measurement: every qubit has a non-identity Pauli (11 qubits).
fn dense_single_block_measurement() -> PbcOperation {
    let basis = vec![
        Pauli::X,
        Pauli::Z,
        Pauli::Y,
        Pauli::X,
        Pauli::Z,
        Pauli::Y,
        Pauli::X,
        Pauli::Z,
        Pauli::Y,
        Pauli::X,
        Pauli::Z,
    ];
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Compilation Pipeline Benchmark ===\n");

    // Build table first (this is the expensive step)
    println!("[Building gross measurement table...]");
    let t = Instant::now();
    let table = build_gross_table();
    println!("  Table built in {:.2?}\n", t.elapsed());

    let accuracy = bicycle_compiler::language::AnglePrecision::lit("1e-16");

    // --- 1-block measurements ---
    println!("[Single-block measurement compilation]");
    let arch1 = PathArchitecture { data_blocks: 1 };
    let op = single_block_measurement();

    bench(
        "sparse measurement (1 block, 2 non-I qubits)",
        100,
        Duration::from_secs(3),
        || {
            black_box(op.compile(&arch1, &table, accuracy));
        },
    );

    let op_dense = dense_single_block_measurement();
    bench(
        "dense measurement  (1 block, 11 non-I qubits)",
        100,
        Duration::from_secs(3),
        || {
            black_box(op_dense.compile(&arch1, &table, accuracy));
        },
    );

    // --- 2-block measurements ---
    println!();
    println!("[Two-block measurement compilation]");
    let arch2 = PathArchitecture { data_blocks: 2 };
    let op2 = two_block_measurement();

    bench(
        "sparse measurement (2 blocks, 4 non-I qubits)",
        100,
        Duration::from_secs(3),
        || {
            black_box(op2.compile(&arch2, &table, accuracy));
        },
    );

    // --- 3-block measurements ---
    println!();
    println!("[Three-block measurement compilation]");
    let arch3 = PathArchitecture { data_blocks: 3 };
    let op3 = three_block_measurement();

    bench(
        "sparse measurement (3 blocks, 6 non-I qubits)",
        100,
        Duration::from_secs(3),
        || {
            black_box(op3.compile(&arch3, &table, accuracy));
        },
    );

    // --- JSON parse + compile (end-to-end) ---
    println!();
    println!("[End-to-end: JSON parse -> compile (1 block)]");
    let json_str = r#"{"Measurement":{"basis":["X","Z","I","I","I","I","I","I","I","I","I"],"flip_result":false}}"#;

    bench(
        "parse + compile measurement",
        100,
        Duration::from_secs(3),
        || {
            let parsed: PbcOperation = serde_json::from_str(json_str).unwrap();
            black_box(parsed.compile(&arch1, &table, accuracy));
        },
    );

    println!();
    println!("Done.");
}

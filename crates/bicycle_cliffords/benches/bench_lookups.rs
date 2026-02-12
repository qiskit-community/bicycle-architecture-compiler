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

//! Micro-benchmarks for the fast-path operations that run *after* the
//! measurement table has been built.
//!
//! These operations are exercised millions of times during compilation
//! and during the BFS table-build inner loop, so even small improvements
//! compound significantly.
//!
//! Benchmarked operations:
//!
//! * **PauliString arithmetic** – `commutes_with`, `conjugate_with`,
//!   `zero_pivot`, multiplication (XOR).
//! * **Table lookups** – `implementation()` and `min_data()` for random
//!   Pauli strings.
//!
//! Run with:
//!
//! ```sh
//! cargo bench --package bicycle_cliffords --bench bench_lookups
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use bicycle_cliffords::{
    CompleteMeasurementTable, GROSS_MEASUREMENT, MeasurementTableBuilder, PauliString,
    native_measurement::NativeMeasurement,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build the gross-code measurement table once (shared across benchmarks).
fn build_gross_table() -> CompleteMeasurementTable {
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    builder.build();
    builder.complete().expect("Table should build successfully")
}

/// Generate a deterministic set of non-trivial Pauli strings for
/// reproducible benchmarks.  We avoid `rand` so that the benchmark
/// binary has no extra dependencies.
fn sample_paulis(n: usize) -> Vec<PauliString> {
    // Simple LCG for deterministic pseudo-random u32 values.
    let mut state: u64 = 0xDEAD_BEEF;
    (0..n)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let raw = ((state >> 16) as u32) % (4_u32.pow(12) - 1) + 1; // 1..4^12
            PauliString(raw)
        })
        .collect()
}

/// 11-qubit Pauli strings (identity on pivot) for `min_data` benchmarks.
fn sample_11qubit_paulis(n: usize) -> Vec<PauliString> {
    sample_paulis(n)
        .into_iter()
        .map(|p| PauliString(p.0 & !((1 << 12) | 1))) // zero out pivot bits
        .filter(|p| p.0 != 0) // skip identity
        .collect()
}

/// Run `f` repeatedly for at least `min_duration` and report the
/// per-iteration average.
fn bench<F: FnMut()>(label: &str, iters: u64, min_duration: Duration, mut f: F) {
    // Warm-up
    for _ in 0..iters.min(10) {
        f();
    }

    let mut total_iters: u64 = 0;
    let start = Instant::now();
    while start.elapsed() < min_duration {
        for _ in 0..iters {
            f();
        }
        total_iters += iters;
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / total_iters as u32;

    println!("  {label:<40} {per_iter:>10.2?}/iter  ({total_iters} iters in {elapsed:.2?})");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    println!("=== Clifford Lookup & PauliString Micro-Benchmarks ===\n");

    // -- PauliString operations (no table needed) ---------------------------
    println!("[PauliString operations]");
    let paulis = sample_paulis(4096);
    let n = paulis.len();

    bench(
        "commutes_with (4096 pairs)",
        1,
        Duration::from_secs(2),
        || {
            for i in 0..n {
                let j = (i + 1) % n;
                black_box(paulis[i].commutes_with(paulis[j]));
            }
        },
    );

    bench(
        "conjugate_with (4096 pairs)",
        1,
        Duration::from_secs(2),
        || {
            for i in 0..n {
                let j = (i + 1) % n;
                black_box(paulis[i].conjugate_with(paulis[j]));
            }
        },
    );

    bench("zero_pivot (4096)", 1, Duration::from_secs(2), || {
        for p in &paulis {
            black_box(p.zero_pivot());
        }
    });

    bench(
        "multiply / XOR (4096 pairs)",
        1,
        Duration::from_secs(2),
        || {
            for i in 0..n {
                let j = (i + 1) % n;
                black_box(paulis[i] * paulis[j]);
            }
        },
    );

    println!();

    // -- Table lookups (requires building the table first) ------------------
    println!("[Building gross table for lookup benchmarks...]");
    let t = Instant::now();
    let table = build_gross_table();
    println!("  Table built in {:.2?}\n", t.elapsed());

    println!("[Table lookup operations]");
    let lookup_paulis = sample_paulis(1024);

    bench(
        "implementation() (1024 lookups)",
        1,
        Duration::from_secs(3),
        || {
            for p in &lookup_paulis {
                black_box(table.implementation(*p));
            }
        },
    );

    let min_data_paulis = sample_11qubit_paulis(1024);
    let k = min_data_paulis.len();

    bench(
        &format!("min_data() ({k} lookups)"),
        1,
        Duration::from_secs(3),
        || {
            for p in &min_data_paulis {
                black_box(table.min_data(*p));
            }
        },
    );

    // -- Throughput estimate for BFS inner loop -----------------------------
    println!();
    println!("[BFS inner-loop throughput estimate]");
    println!("  Simulates the hot path: lookup prev + conjugate + check + insert decision");

    let frontier = sample_paulis(4096);
    let base_rots = sample_paulis(256);

    bench(
        "conjugate_with loop (4096 x 256)",
        1,
        Duration::from_secs(3),
        || {
            for p in &frontier {
                for r in &base_rots {
                    let new = p.conjugate_with(r.zero_pivot());
                    black_box(new);
                }
            }
        },
    );

    println!();
    println!("Done.");
}

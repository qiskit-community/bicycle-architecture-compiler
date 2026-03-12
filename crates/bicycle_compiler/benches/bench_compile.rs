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

use bicycle_cliffords::{
    CompleteMeasurementTable, GROSS_MEASUREMENT, MeasurementTableBuilder,
    native_measurement::NativeMeasurement,
};
use bicycle_common::Pauli;
use bicycle_compiler::PathArchitecture;
use bicycle_compiler::language::{AnglePrecision, PbcOperation};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_gross_table() -> CompleteMeasurementTable {
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    builder.build();
    builder.complete().expect("Table should build successfully")
}

// ---------------------------------------------------------------------------
// Test circuits
// ---------------------------------------------------------------------------

/// Create a sparse measurement on m blocks
fn sparse_m_block_basis(m: usize) -> Vec<Pauli> {
    let mut basis = [Pauli::I; 11];
    basis[2] = Pauli::Z;
    basis.repeat(m)
}

/// Create a dense measurement on m blocks
fn dense_m_block_basis(m: usize) -> Vec<Pauli> {
    let basis = vec![
        Pauli::X,
        Pauli::Z,
        Pauli::Y,
        Pauli::X,
        Pauli::Z,
        Pauli::Y,
        Pauli::Y,
        Pauli::Y,
        Pauli::Y,
        Pauli::X,
        Pauli::Z,
    ];
    basis.repeat(m)
}

/// Benchmark suite for measurement
fn bench_compile(c: &mut Criterion) {
    let table = build_gross_table();
    let accuracy = bicycle_compiler::language::AnglePrecision::lit("1e-16");

    // Dense rotations
    let mut group = c.benchmark_group("rotation (dense)");
    for m in 1..20 {
        let arch = PathArchitecture { data_blocks: m };
        let basis = dense_m_block_basis(m);
        let op = PbcOperation::Rotation {
            basis,
            angle: AnglePrecision::lit("0.1"),
        };
        group.throughput(criterion::Throughput::Elements(m as u64));
        group.bench_with_input(BenchmarkId::from_parameter(m), &op, |b, s| {
            b.iter(|| s.compile(&arch, &table, accuracy));
        });
    }
    group.finish();

    // Dense measurements
    let mut group = c.benchmark_group("measurement (dense)");
    for m in 1..20 {
        let arch = PathArchitecture { data_blocks: m };
        let basis = dense_m_block_basis(m);
        let op = PbcOperation::Measurement {
            basis,
            flip_result: false,
        };
        group.throughput(criterion::Throughput::Elements(m as u64));
        group.bench_with_input(BenchmarkId::from_parameter(m), &op, |b, s| {
            b.iter(|| s.compile(&arch, &table, accuracy));
        });
    }
    group.finish();

    // Native measurements
    let mut group = c.benchmark_group("measurement (native)");
    for m in 1..20 {
        let arch = PathArchitecture { data_blocks: m };
        let basis = sparse_m_block_basis(m);
        let op = PbcOperation::Measurement {
            basis,
            flip_result: false,
        };
        group.throughput(criterion::Throughput::Elements(m as u64));
        group.bench_with_input(BenchmarkId::from_parameter(m), &op, |b, s| {
            b.iter(|| s.compile(&arch, &table, accuracy));
        });
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = bench_compile
}
criterion_main!(benches);

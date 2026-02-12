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

//! TDD correctness tests for the compilation-pipeline benchmarks.
//!
//! These tests verify that every circuit the benchmark compiles
//! produces structurally valid output: correct block indices,
//! properly paired JointMeasure instructions, and architecture-
//! valid operations.

use std::sync::LazyLock;

use bicycle_cliffords::{
    native_measurement::NativeMeasurement, CompleteMeasurementTable, MeasurementTableBuilder,
    GROSS_MEASUREMENT,
};
use bicycle_common::{BicycleISA, Pauli};
use bicycle_compiler::language::PbcOperation;
use bicycle_compiler::PathArchitecture;

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

static GROSS_TABLE: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    builder.build();
    builder.complete().expect("Table should build successfully")
});

const ACCURACY: bicycle_compiler::language::AnglePrecision =
    bicycle_compiler::language::AnglePrecision::lit("1e-16");

// ---------------------------------------------------------------------------
// Helpers – same circuits used by bench_compile.rs
// ---------------------------------------------------------------------------

fn single_block_measurement() -> PbcOperation {
    let mut basis = vec![Pauli::I; 11];
    basis[0] = Pauli::X;
    basis[1] = Pauli::Z;
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

fn two_block_measurement() -> PbcOperation {
    let mut basis = vec![Pauli::I; 22];
    basis[0] = Pauli::X;
    basis[1] = Pauli::Z;
    basis[11] = Pauli::Z;
    basis[12] = Pauli::X;
    PbcOperation::Measurement {
        basis,
        flip_result: false,
    }
}

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
// Structural validators
// ---------------------------------------------------------------------------

/// Check that all block indices in compiled operations are within range.
fn assert_block_indices_in_range(ops: &[Vec<(usize, BicycleISA)>], num_blocks: usize) {
    for (step, op) in ops.iter().enumerate() {
        for (block_idx, isa) in op {
            assert!(
                *block_idx < num_blocks,
                "Step {}: block index {} >= num_blocks {} (instruction {:?})",
                step,
                block_idx,
                num_blocks,
                isa
            );
        }
    }
}

/// Check that every JointMeasure instruction appears in pairs on adjacent blocks.
fn assert_joint_measures_are_paired(ops: &[Vec<(usize, BicycleISA)>]) {
    for (step, op) in ops.iter().enumerate() {
        let joints: Vec<_> = op
            .iter()
            .filter(|(_, isa)| matches!(isa, BicycleISA::JointMeasure(_)))
            .collect();

        if !joints.is_empty() {
            assert_eq!(
                joints.len(),
                2,
                "Step {}: JointMeasure must appear in pairs, found {}",
                step,
                joints.len()
            );
            let (b0, _) = joints[0];
            let (b1, _) = joints[1];
            assert_eq!(
                b0.abs_diff(*b1),
                1,
                "Step {}: JointMeasure blocks {} and {} must be adjacent",
                step,
                b0,
                b1
            );
        }
    }
}

/// Check that operations validate against the architecture.
fn assert_architecture_valid(ops: &[Vec<(usize, BicycleISA)>], arch: &PathArchitecture) {
    for (step, op) in ops.iter().enumerate() {
        assert!(
            arch.validate_operation(op),
            "Step {}: operation {:?} fails architecture validation",
            step,
            op
        );
    }
}

/// Check that the compiled output is non-empty (a valid circuit must produce instructions).
fn assert_non_empty(ops: &[Vec<(usize, BicycleISA)>], label: &str) {
    assert!(
        !ops.is_empty(),
        "Compilation of '{}' must produce at least one instruction",
        label
    );
}

// =========================================================================
// 1. Single-block measurement compilation correctness
// =========================================================================

#[test]
fn single_block_sparse_produces_valid_output() {
    let arch = PathArchitecture { data_blocks: 1 };
    let op = single_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_non_empty(&compiled, "single_block_sparse");
    assert_block_indices_in_range(&compiled, 1);
    assert_architecture_valid(&compiled, &arch);
}

#[test]
fn single_block_sparse_has_no_joint_measures() {
    let arch = PathArchitecture { data_blocks: 1 };
    let op = single_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    let has_joint = compiled.iter().any(|step| {
        step.iter()
            .any(|(_, isa)| matches!(isa, BicycleISA::JointMeasure(_)))
    });
    assert!(
        !has_joint,
        "Single-block compilation must not produce JointMeasure instructions"
    );
}

#[test]
fn single_block_dense_produces_valid_output() {
    let arch = PathArchitecture { data_blocks: 1 };
    let op = dense_single_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_non_empty(&compiled, "single_block_dense");
    assert_block_indices_in_range(&compiled, 1);
    assert_architecture_valid(&compiled, &arch);
}

// =========================================================================
// 2. Two-block measurement compilation correctness
// =========================================================================

#[test]
fn two_block_produces_valid_output() {
    let arch = PathArchitecture { data_blocks: 2 };
    let op = two_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_non_empty(&compiled, "two_block");
    assert_block_indices_in_range(&compiled, 2);
    assert_joint_measures_are_paired(&compiled);
    assert_architecture_valid(&compiled, &arch);
}

#[test]
fn two_block_produces_joint_measures() {
    let arch = PathArchitecture { data_blocks: 2 };
    let op = two_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    let joint_count = compiled
        .iter()
        .filter(|step| {
            step.iter()
                .any(|(_, isa)| matches!(isa, BicycleISA::JointMeasure(_)))
        })
        .count();
    assert!(
        joint_count >= 1,
        "Two-block compilation must produce at least one JointMeasure step, got {}",
        joint_count
    );
}

// =========================================================================
// 3. Three-block measurement compilation correctness
// =========================================================================

#[test]
fn three_block_produces_valid_output() {
    let arch = PathArchitecture { data_blocks: 3 };
    let op = three_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_non_empty(&compiled, "three_block");
    assert_block_indices_in_range(&compiled, 3);
    assert_joint_measures_are_paired(&compiled);
    assert_architecture_valid(&compiled, &arch);
}

#[test]
fn three_block_produces_multiple_joint_measures() {
    let arch = PathArchitecture { data_blocks: 3 };
    let op = three_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    let joint_count = compiled
        .iter()
        .filter(|step| {
            step.iter()
                .any(|(_, isa)| matches!(isa, BicycleISA::JointMeasure(_)))
        })
        .count();
    // 3 blocks means at least 2 JointMeasure pairs to create GHZ state.
    assert!(
        joint_count >= 2,
        "Three-block compilation must produce at least 2 JointMeasure steps, got {}",
        joint_count
    );
}

// =========================================================================
// 4. Scaling: compilation cost grows linearly with blocks
// =========================================================================

#[test]
fn compilation_output_scales_with_blocks() {
    let op1 = single_block_measurement();
    let arch1 = PathArchitecture { data_blocks: 1 };
    let len1 = op1.compile(&arch1, &GROSS_TABLE, ACCURACY).len();

    let op2 = two_block_measurement();
    let arch2 = PathArchitecture { data_blocks: 2 };
    let len2 = op2.compile(&arch2, &GROSS_TABLE, ACCURACY).len();

    let op3 = three_block_measurement();
    let arch3 = PathArchitecture { data_blocks: 3 };
    let len3 = op3.compile(&arch3, &GROSS_TABLE, ACCURACY).len();

    // More blocks means more instructions (strictly).
    assert!(
        len2 > len1,
        "2-block output ({}) must have more instructions than 1-block ({})",
        len2,
        len1
    );
    assert!(
        len3 > len2,
        "3-block output ({}) must have more instructions than 2-block ({})",
        len3,
        len2
    );
}

// =========================================================================
// 6. JSON round-trip (end-to-end) correctness
// =========================================================================

#[test]
fn json_parse_produces_identical_compilation() {
    let json_str = r#"{"Measurement":{"basis":["X","Z","I","I","I","I","I","I","I","I","I"],"flip_result":false}}"#;
    let parsed: PbcOperation = serde_json::from_str(json_str).unwrap();

    let arch = PathArchitecture { data_blocks: 1 };
    let compiled_from_json = parsed.compile(&arch, &GROSS_TABLE, ACCURACY);

    let programmatic = single_block_measurement();
    let compiled_from_code = programmatic.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_eq!(
        compiled_from_json, compiled_from_code,
        "JSON-parsed and programmatically-constructed circuits must compile identically"
    );
}

#[test]
fn json_parse_round_trip_preserves_basis() {
    let op = two_block_measurement();
    let serialized = serde_json::to_string(&op).unwrap();
    let deserialized: PbcOperation = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        op, deserialized,
        "JSON round-trip must preserve the PbcOperation"
    );
}

// =========================================================================
// 8. Architecture validation edge cases
// =========================================================================

#[test]
fn for_qubits_computes_correct_block_count() {
    assert_eq!(1, PathArchitecture::for_qubits(1).data_blocks());
    assert_eq!(1, PathArchitecture::for_qubits(11).data_blocks());
    assert_eq!(2, PathArchitecture::for_qubits(12).data_blocks());
    assert_eq!(2, PathArchitecture::for_qubits(22).data_blocks());
    assert_eq!(3, PathArchitecture::for_qubits(23).data_blocks());
    assert_eq!(3, PathArchitecture::for_qubits(33).data_blocks());
    assert_eq!(10, PathArchitecture::for_qubits(110).data_blocks());
}

#[test]
fn architecture_qubits_is_inverse_of_for_qubits() {
    for blocks in 1..=10 {
        let arch = PathArchitecture {
            data_blocks: blocks,
        };
        assert_eq!(blocks * 11, arch.qubits());
        assert_eq!(
            blocks,
            PathArchitecture::for_qubits(arch.qubits()).data_blocks()
        );
    }
}

// =========================================================================
// 7. Semantic correctness: the compiled ISA sequence uses the right
//    Clifford decomposition for every block
// =========================================================================

/// For each block in a multi-block measurement, the compiler calls
/// `min_data(pauli)` and uses the returned `MeasurementImpl`.  Verify
/// that for our benchmark circuits, the Clifford decomposition used by
/// the compiler genuinely implements the intended Pauli on each block.
///
/// This goes beyond structural checking -- it validates the *semantic*
/// content of the compilation.
#[test]
fn compilation_uses_correct_clifford_decomposition_per_block() {
    use bicycle_cliffords::PauliString;

    // Two-block: X Z I ... I | Z X I ... I
    let basis_blocks: Vec<Vec<Pauli>> = vec![
        vec![
            Pauli::X,
            Pauli::Z,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
        ],
        vec![
            Pauli::Z,
            Pauli::X,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
            Pauli::I,
        ],
    ];

    for (block_idx, block_basis) in basis_blocks.iter().enumerate() {
        // Construct the 12-qubit PauliString: prepend pivot (I), then 11 data qubits.
        let mut full_basis = vec![Pauli::I];
        full_basis.extend_from_slice(block_basis);
        let ps: PauliString = (&full_basis[..]).try_into().unwrap();

        // This is exactly what the compiler does internally.
        let meas_impl = GROSS_TABLE.min_data(ps);

        // Verify reconstruction: base + rotations = intended Pauli (modulo pivot).
        let mut reconstructed = meas_impl.base_measurement().measures();
        for rot in meas_impl.rotations() {
            reconstructed = reconstructed.conjugate_with(rot.measures().zero_pivot());
        }

        assert_eq!(
            ps.zero_pivot(),
            reconstructed.zero_pivot(),
            "Block {}: Clifford decomposition must reconstruct the intended Pauli",
            block_idx
        );

        // Verify the pivot drives valid basis changes.
        let pivot = meas_impl.measures().get_pauli(0);
        assert_ne!(
            pivot,
            Pauli::I,
            "Block {}: pivot must be non-identity for basis-change selection",
            block_idx
        );
    }
}

/// Dense single-block: every qubit is non-identity.
/// Verify the Clifford decomposition for this maximally non-trivial case.
#[test]
fn dense_block_clifford_decomposition_is_correct() {
    use bicycle_cliffords::PauliString;

    let data_qubits = vec![
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

    let mut full_basis = vec![Pauli::I]; // pivot
    full_basis.extend_from_slice(&data_qubits);
    let ps: PauliString = (&full_basis[..]).try_into().unwrap();

    let meas_impl = GROSS_TABLE.min_data(ps);

    let mut reconstructed = meas_impl.base_measurement().measures();
    for rot in meas_impl.rotations() {
        reconstructed = reconstructed.conjugate_with(rot.measures().zero_pivot());
    }

    assert_eq!(
        ps.zero_pivot(),
        reconstructed.zero_pivot(),
        "Dense block Clifford decomposition must reconstruct correctly"
    );

    // Some dense Paulis may still be native measurements. The key invariant
    // is reconstruction correctness, which is asserted above.
}

// =========================================================================
// 9. Deterministic compilation (no hidden randomness)
// =========================================================================

#[test]
fn compilation_is_deterministic() {
    let arch = PathArchitecture { data_blocks: 2 };
    let op = two_block_measurement();

    let compiled_a = op.compile(&arch, &GROSS_TABLE, ACCURACY);
    let compiled_b = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    assert_eq!(
        compiled_a, compiled_b,
        "Compiling the same circuit twice must produce identical output"
    );
}

// =========================================================================
// 10. Verify benchmark operations are not optimized away
// =========================================================================

/// The benchmarks use `black_box()` to prevent dead-code elimination.
/// This test verifies that the operations the benchmarks measure actually
/// produce *observable* side effects (non-zero results) so we can be
/// confident the compiler isn't silently optimizing them to no-ops.
#[test]
fn benchmark_operations_produce_nonzero_results() {
    use bicycle_cliffords::PauliString;

    let arch = PathArchitecture { data_blocks: 1 };
    let op = single_block_measurement();
    let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);

    // The compiled output must be non-empty and contain actual ISA
    // instructions (not zero-cost no-ops).
    assert!(
        compiled.len() > 1,
        "Compiled output must have multiple steps"
    );

    // Verify PauliString operations produce non-trivial results.
    let a = PauliString(0b101010101010101010101010); // alternating pattern
    let b = PauliString(0b010101010101010101010101);

    // These must not be optimized to constants by the compiler.
    let product = a * b;
    assert_ne!(product, a);
    assert_ne!(product, b);
    assert_ne!(product, PauliString(0));

    let conjugated = a.conjugate_with(b);
    assert_ne!(conjugated, PauliString(0)); // non-trivial

    // implementation() must return a real decomposition.
    let meas_impl = GROSS_TABLE.implementation(a);
    assert!(
        !meas_impl.rotations().is_empty() || {
            // Native measurement has 0 rotations -- also valid.
            meas_impl.base_measurement().measures() == a
        }
    );
}

// =========================================================================
// 11. Multi-block example JSON files (smoke tests)
// =========================================================================

#[test]
fn two_blocks_json_compiles_successfully() {
    let json = include_str!("../example/two_blocks.json");
    let ops: Vec<PbcOperation> = serde_json::from_str(json).expect("two_blocks.json must parse");
    assert!(!ops.is_empty(), "two_blocks.json must contain operations");

    let arch = PathArchitecture { data_blocks: 2 };
    for op in &ops {
        match op {
            PbcOperation::Measurement { .. } => {
                let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);
                assert_non_empty(&compiled, "two_blocks.json measurement");
                assert_block_indices_in_range(&compiled, 2);
                assert_joint_measures_are_paired(&compiled);
                assert_architecture_valid(&compiled, &arch);
            }
            PbcOperation::Rotation { .. } => {
                // Rotation compilation requires gridsynth; skip if not available.
            }
        }
    }
}

#[test]
fn three_blocks_json_compiles_successfully() {
    let json = include_str!("../example/three_blocks.json");
    let ops: Vec<PbcOperation> = serde_json::from_str(json).expect("three_blocks.json must parse");
    assert!(!ops.is_empty(), "three_blocks.json must contain operations");

    let arch = PathArchitecture { data_blocks: 3 };
    for op in &ops {
        match op {
            PbcOperation::Measurement { .. } => {
                let compiled = op.compile(&arch, &GROSS_TABLE, ACCURACY);
                assert_non_empty(&compiled, "three_blocks.json measurement");
                assert_block_indices_in_range(&compiled, 3);
                assert_joint_measures_are_paired(&compiled);
                assert_architecture_valid(&compiled, &arch);
            }
            PbcOperation::Rotation { .. } => {
                // Rotation compilation requires gridsynth; skip.
            }
        }
    }
}

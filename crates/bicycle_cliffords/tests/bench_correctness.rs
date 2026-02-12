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

//! TDD correctness tests for the operations exercised by the benchmarks.
//!
//! These tests verify that the PauliString algebra, measurement table,
//! and lookup operations produce *mathematically correct* results, not
//! just that they run fast.  Every property tested here corresponds
//! directly to an operation timed by the benchmark suite.

use std::sync::LazyLock;

use bicycle_cliffords::{
    CompleteMeasurementTable, GROSS_MEASUREMENT, MeasurementTableBuilder, PauliString,
    native_measurement::NativeMeasurement,
};

// ---------------------------------------------------------------------------
// Shared fixture – build the table once for all tests in this file.
// ---------------------------------------------------------------------------

static GROSS_TABLE: LazyLock<CompleteMeasurementTable> = LazyLock::new(|| {
    let mut builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    builder.build();
    builder.complete().expect("Table should build successfully")
});

/// Same deterministic sample generator used by the benchmarks.
/// Duplicated here so tests stay in sync with what the benchmarks actually
/// exercise.
fn sample_paulis(n: usize) -> Vec<PauliString> {
    let mut state: u64 = 0xDEAD_BEEF;
    (0..n)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let raw = ((state >> 16) as u32) % (4_u32.pow(12) - 1) + 1;
            PauliString(raw)
        })
        .collect()
}

fn sample_11qubit_paulis(n: usize) -> Vec<PauliString> {
    sample_paulis(n)
        .into_iter()
        .map(|p| PauliString(p.0 & !((1 << 12) | 1)))
        .filter(|p| p.0 != 0)
        .collect()
}

// =========================================================================
// 1. Benchmark sample-generator correctness
// =========================================================================

#[test]
fn sample_paulis_are_in_valid_range() {
    let paulis = sample_paulis(4096);
    let max = 4_u32.pow(12); // 16,777,216
    for p in &paulis {
        assert!(p.0 >= 1, "Sample should never be identity (0): got {}", p.0);
        assert!(
            p.0 < max,
            "Sample exceeds 12-qubit range: {} >= {}",
            p.0,
            max
        );
    }
}

#[test]
fn sample_paulis_are_deterministic() {
    let a = sample_paulis(256);
    let b = sample_paulis(256);
    assert_eq!(a, b, "Same seed must produce identical sequences");
}

#[test]
fn sample_11qubit_paulis_have_zero_pivot() {
    let paulis = sample_11qubit_paulis(1024);
    for p in &paulis {
        assert_eq!(
            p.pivot_bits(),
            PauliString(0),
            "11-qubit sample must have zero pivot bits: {:?}",
            p
        );
        assert_ne!(p.0, 0, "11-qubit sample must not be identity");
    }
}

#[test]
fn sample_paulis_have_sufficient_variety() {
    let paulis = sample_paulis(4096);
    let unique: std::collections::HashSet<u32> = paulis.iter().map(|p| p.0).collect();
    // With 4096 draws from ~16M space, collisions should be extremely rare.
    assert!(
        unique.len() >= 4000,
        "Expected >= 4000 unique samples, got {}",
        unique.len()
    );
}

// =========================================================================
// 2. PauliString algebraic properties (property-based on benchmark inputs)
// =========================================================================

/// commutes_with is symmetric: a commutes with b iff b commutes with a.
#[test]
fn commutes_with_is_symmetric() {
    let paulis = sample_paulis(1024);
    for i in 0..paulis.len() {
        let j = (i + 1) % paulis.len();
        assert_eq!(
            paulis[i].commutes_with(paulis[j]),
            paulis[j].commutes_with(paulis[i]),
            "commutes_with must be symmetric for {:?} and {:?}",
            paulis[i],
            paulis[j]
        );
    }
}

/// Every Pauli string commutes with itself.
#[test]
fn commutes_with_self() {
    let paulis = sample_paulis(2048);
    for p in &paulis {
        assert!(
            p.commutes_with(*p),
            "PauliString must commute with itself: {:?}",
            p
        );
    }
}

/// Every Pauli string commutes with identity.
#[test]
fn commutes_with_identity() {
    let identity = PauliString(0);
    let paulis = sample_paulis(1024);
    for p in &paulis {
        assert!(
            p.commutes_with(identity),
            "Must commute with identity: {:?}",
            p
        );
    }
}

/// conjugate_with is idempotent when the Pauli commutes.
/// If a commutes with b, then a.conjugate_with(b) == a.
#[test]
fn conjugate_with_commuting_is_identity() {
    let paulis = sample_paulis(2048);
    for i in 0..paulis.len() {
        let j = (i + 7) % paulis.len(); // pick a different index
        if paulis[i].commutes_with(paulis[j]) {
            assert_eq!(
                paulis[i],
                paulis[i].conjugate_with(paulis[j]),
                "Conjugating with a commuting Pauli should be identity"
            );
        }
    }
}

/// conjugate_with anti-commuting case: a.conjugate_with(b) == a * b (XOR).
#[test]
fn conjugate_with_anticommuting_is_product() {
    let paulis = sample_paulis(2048);
    for i in 0..paulis.len() {
        let j = (i + 3) % paulis.len();
        if !paulis[i].commutes_with(paulis[j]) {
            let result = paulis[i].conjugate_with(paulis[j]);
            let expected = paulis[i] * paulis[j]; // XOR
            assert_eq!(
                result, expected,
                "Conjugating with anticommuting Pauli should give product"
            );
        }
    }
}

/// zero_pivot zeroes exactly the pivot bits and preserves everything else.
#[test]
fn zero_pivot_preserves_logical_bits() {
    let paulis = sample_paulis(2048);
    for p in &paulis {
        let zp = p.zero_pivot();
        // Pivot bits (bit 0 for X, bit 12 for Z) must be zero.
        assert_eq!(zp.0 & 1, 0, "Pivot X bit must be zero after zero_pivot");
        assert_eq!(
            zp.0 & (1 << 12),
            0,
            "Pivot Z bit must be zero after zero_pivot"
        );
        // All other bits must match the original.
        let mask = !((1_u32 << 12) | 1);
        assert_eq!(zp.0 & mask, p.0 & mask, "Non-pivot bits must be preserved");
    }
}

/// Multiplication (XOR) is commutative.
#[test]
fn multiply_is_commutative() {
    let paulis = sample_paulis(1024);
    for i in 0..paulis.len() {
        let j = (i + 1) % paulis.len();
        assert_eq!(
            paulis[i] * paulis[j],
            paulis[j] * paulis[i],
            "Pauli multiplication (XOR) must be commutative"
        );
    }
}

/// Multiplication is associative.
#[test]
fn multiply_is_associative() {
    let paulis = sample_paulis(1024);
    for i in 0..paulis.len() - 2 {
        let a = paulis[i];
        let b = paulis[i + 1];
        let c = paulis[i + 2];
        assert_eq!(
            (a * b) * c,
            a * (b * c),
            "Pauli multiplication must be associative"
        );
    }
}

/// Every element is its own inverse under XOR.
#[test]
fn multiply_self_is_identity() {
    let paulis = sample_paulis(1024);
    let identity = PauliString(0);
    for p in &paulis {
        assert_eq!(*p * *p, identity, "p * p must equal identity for {:?}", p);
    }
}

// =========================================================================
// 3. Table structure correctness
// =========================================================================

/// The table must contain exactly 4^12 = 16,777,216 entries (complete).
/// This validates the build benchmark's "total entries" output.
#[test]
fn gross_table_is_complete() {
    // Force table build
    let _ = &*GROSS_TABLE;
    // The table exists (complete() didn't fail), which means all 4^12
    // entries are filled.  This is checked by the CompleteMeasurementTable
    // constructor, but let's also verify lookups don't panic.
    let max = 4_u32.pow(12);
    // Spot-check a spread of values across the entire range.
    for i in (1..max).step_by(16384) {
        let p = PauliString(i);
        let _impl = GROSS_TABLE.implementation(p);
    }
}

/// implementation() must reconstruct the queried Pauli string.
/// This verifies the core correctness property for the benchmark's
/// lookup operations.
#[test]
fn implementation_reconstructs_pauli() {
    let paulis = sample_paulis(512);
    for p in &paulis {
        let meas_impl = GROSS_TABLE.implementation(*p);

        // Reconstruct: start from base, conjugate with each rotation.
        let mut q = meas_impl.base_measurement().measures();
        for rot in meas_impl.rotations() {
            q = q.conjugate_with(rot.measures().zero_pivot());
        }

        assert_eq!(
            *p, q,
            "implementation() must reconstruct the original Pauli: expected {:?}, got {:?}",
            p, q
        );
    }
}

/// min_data() takes a 11-qubit Pauli (zero pivot) and returns an
/// implementation for the best pivot choice.  The reconstructed Pauli
/// must match the original on the 11 logical qubits (pivot may differ).
#[test]
fn min_data_reconstructs_logical_bits() {
    let paulis = sample_11qubit_paulis(512);
    for p in &paulis {
        let meas_impl = GROSS_TABLE.min_data(*p);

        // Reconstruct full 12-qubit Pauli from the implementation.
        let mut q = meas_impl.base_measurement().measures();
        for rot in meas_impl.rotations() {
            q = q.conjugate_with(rot.measures().zero_pivot());
        }

        // The logical bits (qubits 1-11) must match; pivot (qubit 0) is
        // chosen by min_data and may be X, Z, or Y.
        assert_eq!(
            p.zero_pivot(),
            q.zero_pivot(),
            "min_data() must reconstruct the logical bits: expected {:?}, got {:?}",
            p.zero_pivot(),
            q.zero_pivot()
        );

        // The pivot must be non-identity (one of X, Z, Y).
        assert_ne!(
            q.pivot_bits(),
            PauliString(0),
            "min_data() result must have non-identity pivot: {:?}",
            q
        );
    }
}

/// min_data() minimizes over 3 pivot choices (X, Z, Y).  Its rotation
/// count must be <= the minimum of the 3 individual implementation()
/// calls for those pivot choices.
#[test]
fn min_data_picks_cheapest_pivot() {
    let x1 = PauliString(1); // X on pivot
    let z1 = PauliString(1 << 12); // Z on pivot
    let y1 = PauliString(1 | (1 << 12)); // Y on pivot

    let paulis = sample_11qubit_paulis(512);
    for p in &paulis {
        let min = GROSS_TABLE.min_data(*p);
        let min_cost = min.rotations().len();

        // Check all 3 pivot options manually.
        let cost_x = GROSS_TABLE.implementation(*p * x1).rotations().len();
        let cost_z = GROSS_TABLE.implementation(*p * z1).rotations().len();
        let cost_y = GROSS_TABLE.implementation(*p * y1).rotations().len();
        let expected_min = cost_x.min(cost_z).min(cost_y);

        assert_eq!(
            min_cost, expected_min,
            "min_data cost ({}) should equal min(X={}, Z={}, Y={}) = {} for {:?}",
            min_cost, cost_x, cost_z, cost_y, expected_min, p
        );
    }
}

/// Native measurements (540 of them) must have zero rotations in their
/// implementation.
#[test]
fn native_measurements_have_zero_rotations() {
    let natives = NativeMeasurement::all();
    for native in &natives {
        let p = GROSS_MEASUREMENT.measures(native);
        let meas_impl = GROSS_TABLE.implementation(p);
        assert_eq!(
            0,
            meas_impl.rotations().len(),
            "Native measurement for {:?} should have 0 rotations, got {}",
            p,
            meas_impl.rotations().len()
        );
    }
}

/// The builder's seeded entry count must match the number of distinct
/// native Pauli strings (540 natives + 1 identity = 541).
#[test]
fn builder_init_seeds_correct_count() {
    let builder = MeasurementTableBuilder::new(NativeMeasurement::all(), GROSS_MEASUREMENT);
    // 15 logical bases * 36 automorphisms = 540, plus identity = 541
    assert_eq!(
        541,
        builder.len(),
        "Initial seed count should be 540 native + 1 identity"
    );
}

/// ---- CRITICAL EXHAUSTIVE TEST ----
///
/// `min_data()` is the function the compiler actually calls (not
/// `implementation()`), yet the upstream tests only exhaustively verify
/// `implementation()`.  This test closes that gap by checking ALL
/// 4^11 - 1 = 4,194,303 non-identity 11-qubit Pauli strings.
///
/// For each one we verify:
///   1. The returned implementation reconstructs the correct *logical*
///      bits (qubits 1-11).
///   2. The chosen pivot is non-identity (X, Z, or Y).
///   3. The rotation count equals the minimum across the three pivot
///      options (X, Z, Y), i.e. min_data genuinely picked the cheapest.
#[test]
fn min_data_exhaustive_all_11qubit_paulis() {
    let x1 = PauliString(1);
    let z1 = PauliString(1 << 12);
    let y1 = PauliString(1 | (1 << 12));
    let pivot_mask: u32 = (1 << 12) | 1;

    // Iterate over all non-identity 11-qubit Pauli strings.
    // An 11-qubit Pauli uses bits 1..11 (X) and 13..23 (Z), with bits
    // 0 and 12 (pivot) always zero.
    //
    // We generate them by iterating 1..4^11 and mapping to the correct
    // bit positions.
    let mut checked: u64 = 0;
    for raw in 1..4_u32.pow(11) {
        // Map raw → 11-qubit PauliString with zero pivot.
        // raw bits 0..10 → X bits 1..11, raw bits 11..21 → Z bits 13..23.
        let x_bits = raw & ((1 << 11) - 1);
        let z_bits = raw >> 11;
        let ps = PauliString((x_bits << 1) | (z_bits << 13));

        // Sanity: pivot must be zero.
        debug_assert_eq!(ps.0 & pivot_mask, 0);

        let meas_impl = GROSS_TABLE.min_data(ps);

        // 1. Reconstruct from base + rotations.
        let mut q = meas_impl.base_measurement().measures();
        for rot in meas_impl.rotations() {
            q = q.conjugate_with(rot.measures().zero_pivot());
        }
        // Logical bits must match.
        assert_eq!(
            ps.zero_pivot(),
            q.zero_pivot(),
            "min_data reconstruction failed for raw={raw}, ps={ps:?}"
        );

        // 2. Pivot must be non-identity.
        assert_ne!(
            q.pivot_bits(),
            PauliString(0),
            "min_data must choose a non-identity pivot for {ps:?}"
        );

        // 3. Rotation count must be the true minimum.
        let min_cost = meas_impl.rotations().len();
        let cost_x = GROSS_TABLE.implementation(ps * x1).rotations().len();
        let cost_z = GROSS_TABLE.implementation(ps * z1).rotations().len();
        let cost_y = GROSS_TABLE.implementation(ps * y1).rotations().len();
        let expected_min = cost_x.min(cost_z).min(cost_y);
        assert_eq!(
            min_cost, expected_min,
            "min_data did not pick cheapest pivot for {ps:?}: got {min_cost}, expected min({cost_x},{cost_z},{cost_y})={expected_min}"
        );

        checked += 1;
    }

    // Verify we checked exactly 4^11 - 1 entries.
    assert_eq!(
        checked,
        4_u64.pow(11) - 1,
        "Must have checked all non-identity 11-qubit Paulis"
    );
}

/// Verify that `min_data().measures()` returns a Pauli whose pivot
/// is consistent with the chosen decomposition.  The compile pipeline
/// calls `meas_impl.measures().get_pauli(0)` to determine basis changes,
/// so this pivot value MUST be the actual pivot of the reconstructed
/// Pauli -- not just any value.
#[test]
fn min_data_measures_field_matches_reconstruction() {
    let paulis = sample_11qubit_paulis(1024);
    for p in &paulis {
        let meas_impl = GROSS_TABLE.min_data(*p);

        // measures() is set by implementation(), which stores `p` directly.
        let stored = meas_impl.measures();

        // Reconstruct independently.
        let mut reconstructed = meas_impl.base_measurement().measures();
        for rot in meas_impl.rotations() {
            reconstructed = reconstructed.conjugate_with(rot.measures().zero_pivot());
        }

        // The stored measures() field must exactly equal the reconstructed value.
        assert_eq!(
            stored, reconstructed,
            "measures() field must match reconstruction for input {:?}",
            p
        );

        // The pivot extracted by the compiler must be non-identity.
        let pivot_pauli = stored.get_pauli(0);
        assert_ne!(
            pivot_pauli,
            bicycle_common::Pauli::I,
            "Pivot Pauli from measures() must not be identity for {:?}",
            p
        );
    }
}

// =========================================================================
// 4. BFS inner-loop operation correctness
// =========================================================================

/// The BFS inner loop computes: new = prev.conjugate_with(rot.zero_pivot()).
/// Verify that for any prev in the table, conjugating with a base rotation
/// produces a valid PauliString that is also in the table.
#[test]
fn bfs_conjugation_produces_valid_entries() {
    let natives = NativeMeasurement::all();
    let native_paulis: Vec<PauliString> = natives
        .iter()
        .map(|n| GROSS_MEASUREMENT.measures(n))
        .collect();

    // Pick some "frontier" entries (native Paulis as seed)
    let base_rots: Vec<PauliString> = native_paulis
        .iter()
        .filter(|p| p.has_pivot_support())
        .map(|p| p.zero_pivot())
        .collect();

    let max = 4_u32.pow(12);
    for prev in native_paulis.iter().take(50) {
        for rot in base_rots.iter().take(50) {
            let new = prev.conjugate_with(*rot);
            assert!(
                new.0 < max,
                "Conjugation result must be in valid range: {} >= {}",
                new.0,
                max
            );
            // The new Pauli must be in the table (since the table is complete).
            let _impl = GROSS_TABLE.implementation(new);
        }
    }
}

/// Conjugation with identity must return the original.
#[test]
fn conjugation_with_zero_pivot_identity_is_noop() {
    let identity_rot = PauliString(0); // zero_pivot of identity
    let paulis = sample_paulis(512);
    for p in &paulis {
        // Everything commutes with identity, so conjugation is a no-op.
        assert_eq!(
            *p,
            p.conjugate_with(identity_rot),
            "Conjugating with zero Pauli should be identity"
        );
    }
}

/// Conjugation preserves commutation class.
///
/// **Mathematical proof (binary symplectic picture):**
///
/// `conjugate_with(r)` maps `a → a` if `[a,r]=0`, or `a → a⊕r` if
/// `[a,r]=1` (anti-commute).  Denote `a' = conjugate_with(r)(a)`.
///
/// The symplectic inner product is linear:
///   `[a⊕r, b⊕r] = [a,b] + [a,r] + [r,b] + [r,r]`
///
/// Since `[r,r] = 0` always (every Pauli commutes with itself), we have
/// four cases depending on `([a,r], [b,r])`:
///
/// | `[a,r]` | `[b,r]` | `a'`  | `b'`  | `[a',b']`               |
/// |---------|---------|-------|-------|--------------------------|
/// |    0    |    0    |  `a`  |  `b`  | `[a,b]`                 |
/// |    1    |    0    | `a⊕r` |  `b`  | `[a,b] + [r,b]` = `[a,b]+0` |
/// |    0    |    1    |  `a`  | `b⊕r` | `[a,b] + [a,r]` = `[a,b]+0` |
/// |    1    |    1    | `a⊕r` | `b⊕r` | `[a,b] + 1 + 1 + 0` = `[a,b]` |
///
/// In every case `[a',b'] = [a,b]`.  QED.
///
/// This test empirically verifies the proof by covering all 4 cases
/// explicitly and then checking a large random sample.
#[test]
fn conjugation_preserves_commutation() {
    let paulis = sample_paulis(512);
    let rots: Vec<PauliString> = sample_paulis(64)
        .into_iter()
        .map(|p| p.zero_pivot())
        .filter(|p| p.0 != 0) // skip identity rot for case coverage
        .collect();

    // Track which of the 4 cases we've actually exercised.
    let mut case_00 = false; // both commute with r
    let mut case_10 = false; // a anti-commutes, b commutes
    let mut case_01 = false; // a commutes, b anti-commutes
    let mut case_11 = false; // both anti-commute with r

    for i in 0..paulis.len() - 1 {
        let a = paulis[i];
        let b = paulis[i + 1];
        let commute_before = a.commutes_with(b);

        for r in &rots {
            let a_comm = a.commutes_with(*r);
            let b_comm = b.commutes_with(*r);

            match (a_comm, b_comm) {
                (true, true) => case_00 = true,
                (false, true) => case_10 = true,
                (true, false) => case_01 = true,
                (false, false) => case_11 = true,
            }

            let a2 = a.conjugate_with(*r);
            let b2 = b.conjugate_with(*r);
            assert_eq!(
                commute_before,
                a2.commutes_with(b2),
                "Conjugation must preserve commutation: case ({},{}), a={:?}, b={:?}, r={:?}",
                !a_comm as u8,
                !b_comm as u8,
                a,
                b,
                r
            );
        }
    }

    // Verify all 4 cases were exercised -- otherwise the test has a
    // coverage hole.
    assert!(case_00, "Case (commute,commute) was never exercised");
    assert!(case_10, "Case (anti,commute) was never exercised");
    assert!(case_01, "Case (commute,anti) was never exercised");
    assert!(case_11, "Case (anti,anti) was never exercised");
}

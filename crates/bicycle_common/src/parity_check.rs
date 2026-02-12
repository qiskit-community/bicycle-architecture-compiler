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

use std::fmt::{Display, Formatter};

use rand::{Rng, SeedableRng, rngs::StdRng};
use sprs::{CsMat, TriMat};

/// Dense GF(2) matrix in row-major form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryMatrix {
    rows: usize,
    cols: usize,
    data: Vec<u8>,
}

impl BinaryMatrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![0; rows * cols],
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn get(&self, row: usize, col: usize) -> u8 {
        self.data[self.index(row, col)]
    }

    pub fn row_weight(&self, row: usize) -> usize {
        assert!(row < self.rows);
        let start = row * self.cols;
        let end = start + self.cols;
        self.data[start..end].iter().map(|v| *v as usize).sum()
    }

    pub fn col_weight(&self, col: usize) -> usize {
        assert!(col < self.cols);
        (0..self.rows).map(|row| self.get(row, col) as usize).sum()
    }

    pub fn transpose(&self) -> Self {
        let mut out = Self::zeros(self.cols, self.rows);
        for row in 0..self.rows {
            for col in 0..self.cols {
                let out_idx = out.index(col, row);
                out.data[out_idx] = self.get(row, col);
            }
        }
        out
    }

    pub fn hstack(&self, rhs: &Self) -> Self {
        assert_eq!(
            self.rows, rhs.rows,
            "cannot hstack matrices with different rows"
        );
        let mut out = Self::zeros(self.rows, self.cols + rhs.cols);
        for row in 0..self.rows {
            for col in 0..self.cols {
                let out_idx = out.index(row, col);
                out.data[out_idx] = self.get(row, col);
            }
            for col in 0..rhs.cols {
                let out_idx = out.index(row, self.cols + col);
                out.data[out_idx] = rhs.get(row, col);
            }
        }
        out
    }

    pub fn row_major_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Convert to CSR format for decoder interoperability.
    pub fn to_csr(&self) -> CsMat<u8> {
        let mut tri = TriMat::new((self.rows, self.cols));
        for row in 0..self.rows {
            for col in 0..self.cols {
                if self.get(row, col) == 1 {
                    tri.add_triplet(row, col, 1u8);
                }
            }
        }
        tri.to_csr()
    }

    fn index(&self, row: usize, col: usize) -> usize {
        assert!(row < self.rows);
        assert!(col < self.cols);
        row * self.cols + col
    }

    fn toggle(&mut self, row: usize, col: usize) {
        let idx = self.index(row, col);
        self.data[idx] ^= 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToricParityChecks {
    pub order: (usize, usize),
    pub hx: BinaryMatrix,
    pub hz: BinaryMatrix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyndromeError {
    DimensionMismatch { expected: usize, found: usize },
    NonBinaryInput { index: usize, value: u8 },
}

impl Display for SyndromeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DimensionMismatch { expected, found } => write!(
                f,
                "error vector length mismatch: expected {expected}, found {found}"
            ),
            Self::NonBinaryInput { index, value } => {
                write!(
                    f,
                    "error vector contains non-binary entry at {index}: {value}"
                )
            }
        }
    }
}

impl std::error::Error for SyndromeError {}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorSource {
    Explicit(Vec<u8>),
    Bernoulli { p: f64, seed: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedSyndrome {
    pub syndrome_x: Vec<u8>,
    pub syndrome_z: Vec<u8>,
    pub x_error: Vec<u8>,
    pub z_error: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimulationError {
    CheckWidthMismatch { hx_cols: usize, hz_cols: usize },
    InvalidProbability { label: &'static str, value: f64 },
    XSyndrome(SyndromeError),
    ZSyndrome(SyndromeError),
}

impl Display for SimulationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckWidthMismatch { hx_cols, hz_cols } => {
                write!(
                    f,
                    "check matrix width mismatch: hx has {hx_cols}, hz has {hz_cols}"
                )
            }
            Self::InvalidProbability { label, value } => {
                write!(f, "invalid Bernoulli probability for {label}: {value}")
            }
            Self::XSyndrome(err) => write!(f, "failed to compute x syndrome: {err}"),
            Self::ZSyndrome(err) => write!(f, "failed to compute z syndrome: {err}"),
        }
    }
}

impl std::error::Error for SimulationError {}

const GROSS_A_TERMS: &[(i32, i32)] = &[(0, 0), (0, 1), (3, -1)];
const GROSS_B_TERMS: &[(i32, i32)] = &[(0, 0), (1, 0), (-1, -3)];

/// Build toric parity-check matrices from two bivariate polynomials A, B.
///
/// The generated matrices follow the notebook construction:
/// Hx = [A | B], Hz = [B^T | A^T].
pub fn toric_parity_checks(
    order: (usize, usize),
    a_terms: &[(i32, i32)],
    b_terms: &[(i32, i32)],
) -> ToricParityChecks {
    let a = polynomial_matrix(order, a_terms);
    let b = polynomial_matrix(order, b_terms);
    let hx = a.hstack(&b);
    let hz = b.transpose().hstack(&a.transpose());
    ToricParityChecks { order, hx, hz }
}

/// Gross toric parity-check matrices.
pub fn gross_toric_parity_checks() -> ToricParityChecks {
    toric_parity_checks((12, 6), GROSS_A_TERMS, GROSS_B_TERMS)
}

/// Two-gross toric parity-check matrices.
pub fn two_gross_toric_parity_checks() -> ToricParityChecks {
    toric_parity_checks((12, 12), GROSS_A_TERMS, GROSS_B_TERMS)
}

/// Compute syndrome s = H * e^T over GF(2).
pub fn syndrome(h: &BinaryMatrix, error: &[u8]) -> Result<Vec<u8>, SyndromeError> {
    if error.len() != h.cols() {
        return Err(SyndromeError::DimensionMismatch {
            expected: h.cols(),
            found: error.len(),
        });
    }
    for (index, value) in error.iter().copied().enumerate() {
        if value > 1 {
            return Err(SyndromeError::NonBinaryInput { index, value });
        }
    }

    let mut out = vec![0u8; h.rows()];
    for (row, out_value) in out.iter_mut().enumerate() {
        let mut parity = 0u8;
        for (col, error_value) in error.iter().copied().enumerate() {
            parity ^= h.get(row, col) & error_value;
        }
        *out_value = parity;
    }
    Ok(out)
}

/// Compute one CSS syndrome sample from either explicit or seeded Bernoulli errors.
///
/// Convention:
/// * `syndrome_x = Hx * z_error^T`
/// * `syndrome_z = Hz * x_error^T`
pub fn simulate_syndrome_once(
    hx: &BinaryMatrix,
    hz: &BinaryMatrix,
    x_error_source: ErrorSource,
    z_error_source: ErrorSource,
) -> Result<SimulatedSyndrome, SimulationError> {
    if hx.cols() != hz.cols() {
        return Err(SimulationError::CheckWidthMismatch {
            hx_cols: hx.cols(),
            hz_cols: hz.cols(),
        });
    }
    let n = hx.cols();
    let x_error = materialize_error_source(n, x_error_source, "x_error")?;
    let z_error = materialize_error_source(n, z_error_source, "z_error")?;

    let syndrome_x = syndrome(hx, &z_error).map_err(SimulationError::XSyndrome)?;
    let syndrome_z = syndrome(hz, &x_error).map_err(SimulationError::ZSyndrome)?;

    Ok(SimulatedSyndrome {
        syndrome_x,
        syndrome_z,
        x_error,
        z_error,
    })
}

fn materialize_error_source(
    n: usize,
    source: ErrorSource,
    label: &'static str,
) -> Result<Vec<u8>, SimulationError> {
    match source {
        ErrorSource::Explicit(error) => Ok(error),
        ErrorSource::Bernoulli { p, seed } => {
            if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                return Err(SimulationError::InvalidProbability { label, value: p });
            }
            let mut rng = StdRng::seed_from_u64(seed);
            let mut error = vec![0u8; n];
            for bit in &mut error {
                *bit = u8::from(rng.random::<f64>() < p);
            }
            Ok(error)
        }
    }
}

fn polynomial_matrix(order: (usize, usize), terms: &[(i32, i32)]) -> BinaryMatrix {
    let (d1, d2) = order;
    let dim = d1 * d2;
    let mut out = BinaryMatrix::zeros(dim, dim);

    for &(ax, ay) in terms {
        let sx = rem_euclid_i32(ax, d1);
        let sy = rem_euclid_i32(ay, d2);
        for x in 0..d1 {
            for y in 0..d2 {
                let row = x * d2 + y;
                let col = ((x + sx) % d1) * d2 + ((y + sy) % d2);
                out.toggle(row, col);
            }
        }
    }
    out
}

fn rem_euclid_i32(value: i32, modulus: usize) -> usize {
    let modulus = modulus as i64;
    (value as i64).rem_euclid(modulus) as usize
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{
        BinaryMatrix, ErrorSource, SimulationError, SyndromeError, gross_toric_parity_checks,
        polynomial_matrix, simulate_syndrome_once, syndrome, toric_parity_checks,
        two_gross_toric_parity_checks,
    };

    #[test]
    fn polynomial_matrix_handles_negative_exponents() {
        let matrix = polynomial_matrix((3, 2), &[(0, 0), (-1, 1)]);
        assert_eq!(matrix.rows(), 6);
        assert_eq!(matrix.cols(), 6);
        assert_eq!(matrix.row_weight(0), 2);
        assert_eq!(matrix.get(0, 0), 1);
        assert_eq!(matrix.get(0, 5), 1);
    }

    #[test]
    fn gross_shapes_and_weights() {
        let checks = gross_toric_parity_checks();
        assert_eq!(checks.order, (12, 6));
        assert_eq!(checks.hx.rows(), 72);
        assert_eq!(checks.hx.cols(), 144);
        assert_eq!(checks.hz.rows(), 72);
        assert_eq!(checks.hz.cols(), 144);

        for row in 0..checks.hx.rows() {
            assert_eq!(checks.hx.row_weight(row), 6);
            assert_eq!(checks.hz.row_weight(row), 6);
        }
        for col in 0..checks.hx.cols() {
            assert_eq!(checks.hx.col_weight(col), 3);
            assert_eq!(checks.hz.col_weight(col), 3);
        }
    }

    #[test]
    fn two_gross_shapes_and_weights() {
        let checks = two_gross_toric_parity_checks();
        assert_eq!(checks.order, (12, 12));
        assert_eq!(checks.hx.rows(), 144);
        assert_eq!(checks.hx.cols(), 288);
        assert_eq!(checks.hz.rows(), 144);
        assert_eq!(checks.hz.cols(), 288);

        for row in 0..checks.hx.rows() {
            assert_eq!(checks.hx.row_weight(row), 6);
            assert_eq!(checks.hz.row_weight(row), 6);
        }
        for col in 0..checks.hx.cols() {
            assert_eq!(checks.hx.col_weight(col), 3);
            assert_eq!(checks.hz.col_weight(col), 3);
        }
    }

    #[test]
    fn css_orthogonality_for_gross_and_two_gross() {
        assert_css_orthogonality(
            &gross_toric_parity_checks().hx,
            &gross_toric_parity_checks().hz,
        );
        assert_css_orthogonality(
            &two_gross_toric_parity_checks().hx,
            &two_gross_toric_parity_checks().hz,
        );
    }

    #[test]
    fn fingerprints_match_reference_generation() {
        let gross = gross_toric_parity_checks();
        let two_gross = two_gross_toric_parity_checks();

        assert_eq!(
            sha256_hex(gross.hx.row_major_bytes()),
            "d18899e6afd52abed989ab8f2109ce81e3151af9e619b35888f47e3ef935e058"
        );
        assert_eq!(
            sha256_hex(gross.hz.row_major_bytes()),
            "0ec2c6530e9fa7d1a266450f830e0c94c7ed71e10b409e64188a4d81eabafd08"
        );
        assert_eq!(
            sha256_hex(two_gross.hx.row_major_bytes()),
            "64a709abea173ccabf4bb016ddbec0322b949daaec712102ce58124684f7d791"
        );
        assert_eq!(
            sha256_hex(two_gross.hz.row_major_bytes()),
            "431ac0504f6138c155ec67cf83a069448e337a63bcc9f1aa793f2d59e11659c3"
        );
    }

    #[test]
    fn explicit_order_generation_matches_named_constructors() {
        let a_terms = &[(0, 0), (0, 1), (3, -1)];
        let b_terms = &[(0, 0), (1, 0), (-1, -3)];
        let built = toric_parity_checks((12, 6), a_terms, b_terms);
        assert_eq!(built.hx, gross_toric_parity_checks().hx);
        assert_eq!(built.hz, gross_toric_parity_checks().hz);
    }

    #[test]
    fn sparse_interop_preserves_shape_and_entries() {
        let mut matrix = BinaryMatrix::zeros(3, 4);
        matrix.toggle(0, 1);
        matrix.toggle(1, 3);
        matrix.toggle(2, 0);
        matrix.toggle(2, 1);

        let sparse = matrix.to_csr();
        assert_eq!(sparse.rows(), 3);
        assert_eq!(sparse.cols(), 4);
        assert_eq!(sparse.nnz(), 4);
        assert_eq!(sparse.get(0, 1), Some(&1));
        assert_eq!(sparse.get(1, 3), Some(&1));
        assert_eq!(sparse.get(2, 0), Some(&1));
        assert_eq!(sparse.get(2, 1), Some(&1));
        assert_eq!(sparse.get(0, 0), None);
    }

    #[test]
    fn sparse_interop_scales_to_gross_hx() {
        let gross = gross_toric_parity_checks();
        let sparse = gross.hx.to_csr();
        assert_eq!(sparse.rows(), gross.hx.rows());
        assert_eq!(sparse.cols(), gross.hx.cols());
        assert_eq!(sparse.nnz(), gross.hx.rows() * 6);
    }

    #[test]
    fn syndrome_matches_manual_parity() {
        let mut h = BinaryMatrix::zeros(3, 4);
        h.toggle(0, 1);
        h.toggle(1, 3);
        h.toggle(2, 0);
        h.toggle(2, 1);

        let s = syndrome(&h, &[1, 0, 1, 1]).expect("valid binary vector");
        assert_eq!(s, vec![0, 1, 1]);
    }

    #[test]
    fn syndrome_rejects_non_binary_input() {
        let mut h = BinaryMatrix::zeros(1, 3);
        h.toggle(0, 0);
        let err = syndrome(&h, &[1, 2, 0]).expect_err("must reject non-binary entries");
        assert_eq!(err, SyndromeError::NonBinaryInput { index: 1, value: 2 });
    }

    #[test]
    fn syndrome_rejects_wrong_length() {
        let h = BinaryMatrix::zeros(2, 4);
        let err = syndrome(&h, &[1, 0]).expect_err("must reject wrong length");
        assert_eq!(
            err,
            SyndromeError::DimensionMismatch {
                expected: 4,
                found: 2
            }
        );
    }

    #[test]
    fn simulate_syndrome_zero_error_returns_zero_vectors() {
        let checks = gross_toric_parity_checks();
        let n = checks.hx.cols();

        let shot = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Explicit(vec![0; n]),
            ErrorSource::Explicit(vec![0; n]),
        )
        .expect("valid zero-error simulation");

        assert!(shot.syndrome_x.iter().all(|&b| b == 0));
        assert!(shot.syndrome_z.iter().all(|&b| b == 0));
        assert!(shot.x_error.iter().all(|&b| b == 0));
        assert!(shot.z_error.iter().all(|&b| b == 0));
    }

    #[test]
    fn simulate_syndrome_seeded_sampling_is_deterministic() {
        let checks = gross_toric_parity_checks();
        let s1 = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Bernoulli { p: 0.05, seed: 42 },
            ErrorSource::Bernoulli {
                p: 0.03,
                seed: 1337,
            },
        )
        .expect("first seeded run should pass");
        let s2 = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Bernoulli { p: 0.05, seed: 42 },
            ErrorSource::Bernoulli {
                p: 0.03,
                seed: 1337,
            },
        )
        .expect("second seeded run should pass");
        assert_eq!(s1, s2);
    }

    #[test]
    fn simulate_syndrome_rejects_mismatched_check_widths() {
        let hx = BinaryMatrix::zeros(2, 5);
        let hz = BinaryMatrix::zeros(2, 4);
        let err = simulate_syndrome_once(
            &hx,
            &hz,
            ErrorSource::Explicit(vec![0; 5]),
            ErrorSource::Explicit(vec![0; 5]),
        )
        .expect_err("must reject mismatched check widths");

        assert_eq!(
            err,
            SimulationError::CheckWidthMismatch {
                hx_cols: 5,
                hz_cols: 4
            }
        );
    }

    #[test]
    fn simulate_syndrome_rejects_invalid_probability() {
        let checks = gross_toric_parity_checks();
        let err = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Bernoulli { p: 1.5, seed: 1 },
            ErrorSource::Explicit(vec![0; checks.hx.cols()]),
        )
        .expect_err("must reject invalid probability");

        assert_eq!(
            err,
            SimulationError::InvalidProbability {
                label: "x_error",
                value: 1.5
            }
        );
    }

    #[test]
    fn simulate_syndrome_rejects_wrong_explicit_error_length() {
        let checks = gross_toric_parity_checks();
        let n = checks.hx.cols();
        let err = simulate_syndrome_once(
            &checks.hx,
            &checks.hz,
            ErrorSource::Explicit(vec![0; n - 1]),
            ErrorSource::Explicit(vec![0; n]),
        )
        .expect_err("must reject wrong explicit x-error length");

        assert_eq!(
            err,
            SimulationError::ZSyndrome(SyndromeError::DimensionMismatch {
                expected: n,
                found: n - 1
            })
        );
    }

    fn assert_css_orthogonality(hx: &BinaryMatrix, hz: &BinaryMatrix) {
        assert_eq!(
            hx.cols(),
            hz.cols(),
            "hx and hz must have the same number of columns"
        );
        for x_row in 0..hx.rows() {
            for z_row in 0..hz.rows() {
                let mut parity = 0u8;
                for col in 0..hx.cols() {
                    parity ^= hx.get(x_row, col) & hz.get(z_row, col);
                }
                assert_eq!(
                    parity, 0,
                    "found anticommuting checks at rows ({x_row}, {z_row})"
                );
            }
        }
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }
}

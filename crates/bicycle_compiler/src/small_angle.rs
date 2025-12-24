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

use core::str;
use std::{
    collections::HashMap,
    io::{self, ErrorKind},
    process::Command,
    sync::{LazyLock, Mutex},
};

use bicycle_common::Pauli;
use log::{debug, trace};
use regex::Regex;

use crate::language::AnglePrecision;

type CacheHashMap =
    HashMap<(AnglePrecision, AnglePrecision), (Vec<SingleRotation>, Vec<CliffordGate>)>;
static CACHE: LazyLock<Mutex<CacheHashMap>> = LazyLock::new(Default::default);

/// The angle θ such that Z(θ) := exp(-iθ/2) diag(1, exp(iθ)) = T up to the global phase exp(-iθ/2).
pub const T_ANGLE: AnglePrecision = AnglePrecision::FRAC_PI_4;

/// Synthesize a rotation e^{iθZ} in terms of T and T_X = HTH rotations, followed by Cliffords,
/// up to a global phase.
/// The required accuracy must be less than 0.1 and determines ‖e^{iθZ} - U‖ ≤ ε in operator norm.
pub fn synthesize_angle(
    theta: AnglePrecision,
    accuracy: AnglePrecision,
) -> (Vec<SingleRotation>, Vec<CliffordGate>) {
    assert!(accuracy <= 1e-1);

    // Handle T gate special case. We only check for equality, and if not pass it to gridsynth.
    if theta.abs() == T_ANGLE {
        trace!("Angle equal to T: {theta}");
        return (
            vec![SingleRotation::Z {
                dagger: theta.is_negative(),
            }],
            vec![],
        );
    }
    // Some notes for approximation guarantees and an implementation that suffers from rounding errors.
    // Since we don't care about the global phase, we can write Z(θ) = diag(1, exp(-i2θ))
    // and obtain ||Z(θ) - T|| = √(2(1-cos(2(π/4-θ))) ≤ ε
    // (More generally, we can compute ||Z(θ) - Z(θ')|| = √(2(1-cos(θ-θ')))
    // Note: ε² needs 96*2 fractional bits and therefore may underflow to 0. This is fine because
    // that means it is smaller than the precision of the left hand side.
    // let rhs = AnglePrecision::ONE - (accuracy * accuracy) / 2;
    // let lhs = (2 * (T_ANGLE - theta.abs());
    // // FIXME: The cos may round 1-δ to 1, which is not ok.
    // let lhs_float: f64 = lhs.to_num();
    // if lhs_float.cos() >= rhs {
    //     trace!("Close to T: {theta}");
    //     return (vec![SingleRotation::Z { dagger: theta.is_negative() }], vec![]);
    // }

    if let Some(result) = CACHE.try_lock().unwrap().get(&(theta, accuracy)) {
        trace!("Cached angle: {theta}");
        return result.clone();
    }
    debug!("Synthesizing angle: {theta}");

    // Do I need scientific notation here? E.g. for the accuracy.
    let gates = run_gridsynth(&theta.to_string(), &accuracy.to_string())
        .expect("gridsynth should run successfully. Is it installed? See README.");
    let res =
        compile_rots(&gates).expect("Should be able to parse MA normal form provided by gridsynth");

    CACHE
        .try_lock()
        .unwrap()
        .insert((theta, accuracy), res.clone());
    res
}

/// Synthesize a rotation e^{iθX} up to global phase.
pub fn synthesize_angle_x(
    theta: AnglePrecision,
    accuracy: AnglePrecision,
) -> (Vec<SingleRotation>, Vec<CliffordGate>) {
    let (mut rots, mut cliff) = synthesize_angle(theta, accuracy);
    for rot in rots.iter_mut() {
        rot.switch_basis();
    }
    cliff.insert(0, CliffordGate::H);
    cliff.push(CliffordGate::H);
    (rots, cliff)
}

fn run_gridsynth(angle: &str, accuracy: &str) -> Result<String, io::Error> {
    dbg!(angle);
    dbg!(accuracy);
    let cmd = Command::new("gridsynth")
        .arg("-p") // Ignore global phase
        .args(["--epsilon", accuracy])
        // Use "--" to ensure negative angles are not interpreted as arguments
        .args(["--", angle])
        .output()?;

    let mut output = cmd.stdout;
    output.truncate(output.len() - 1);

    String::from_utf8(output).map_err(|err| io::Error::new(ErrorKind::InvalidData, err.to_string()))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SingleRotation {
    Z { dagger: bool },
    X { dagger: bool },
}

impl SingleRotation {
    fn take_dagger(&mut self) {
        // Maybe factor out dagger field to super type?
        match self {
            Self::Z { dagger } => *dagger = !*dagger,
            Self::X { dagger } => *dagger = !*dagger,
        }
    }

    /// Conjugate in-place this SingleRotation by Hadamards, switching its basis
    fn switch_basis(&mut self) {
        match self {
            Self::Z { dagger } => *self = Self::X { dagger: *dagger },
            Self::X { dagger } => *self = Self::Z { dagger: *dagger },
        };
    }

    #[allow(dead_code)]
    pub fn basis(&self) -> Pauli {
        match *self {
            Self::Z { dagger: _ } => Pauli::Z,
            Self::X { dagger: _ } => Pauli::X,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliffordGate {
    S,
    H,
    X,
    W,
}

impl TryFrom<char> for CliffordGate {
    type Error = io::Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'S' => Ok(CliffordGate::S),
            'H' => Ok(CliffordGate::H),
            'X' => Ok(CliffordGate::X),
            'W' => Ok(CliffordGate::W),
            _ => Err(io::Error::new(
                ErrorKind::InvalidData,
                "Unexpected character when converting to CliffordGate",
            )),
        }
    }
}

/// Compile rotations up to global phase
/// W gates are discarded
fn compile_rots(gates: &str) -> Result<(Vec<SingleRotation>, Vec<CliffordGate>), io::Error> {
    let mut rotations = vec![];
    let mut cliffords: Vec<CliffordGate> = vec![];

    // Regular expression for Matsumoto-Amano normal form
    let re = Regex::new(r"(?<first>T)?(?<main>(HT|SHT)*)(?<clifford>[HXSW]*)").unwrap();
    let main_re = Regex::new(r"(HT|SHT)").unwrap();

    let captured = re
        .captures(gates)
        .expect("The gate sequence should be in Matsumoto-Amano normal form");

    if captured.name("first").is_some() {
        rotations.push(SingleRotation::Z { dagger: false });
    }

    let mut s_start = false;
    let mut z_basis = true;
    let main = &captured["main"];
    for m in main_re.find_iter(main) {
        // SHT case
        if m.len() == 3 {
            // Dagger previous T
            if let Some(t) = rotations.last_mut() {
                t.take_dagger();
            } else {
                // No previous T
                // An S as the beginning commutes with e^{iφZ} such that we can push it to the ending sequence of Cliffords
                // This saves us from doing Y-basis rotations
                s_start = true;
            }
        }

        // Now deal with remaining HT
        z_basis = !z_basis;
        if z_basis {
            rotations.push(SingleRotation::Z { dagger: false });
        } else {
            rotations.push(SingleRotation::X { dagger: false });
        }
    }

    let mut cliff_chars: &str = &captured["clifford"];
    if !z_basis {
        // Take a Hadamard from the Clifford circuit
        if let Some('H') = cliff_chars.chars().next() {
            cliff_chars = &cliff_chars[1..];
        } else {
            // Or insert HH if it isn't there and take the first (i.e. insert H)
            cliffords.push(CliffordGate::H);
        }
    }

    let mut cliff_circuit: Vec<CliffordGate> = cliff_chars
        .chars()
        .map(|c| c.try_into())
        .collect::<Result<Vec<CliffordGate>, io::Error>>()?;
    cliffords.append(&mut cliff_circuit);

    if s_start {
        cliffords.push(CliffordGate::S);
    }

    Ok((rotations, cliffords))
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use super::*;

    // #[test]
    // fn test_05_minus3() -> Result<(), Box<dyn Error>> {
    //     let test_str = "THTHTSHTSHTHTHTHTHTHTHTHTHTHTHTHTSHTHTHTSHTHTSHTHTHTSHTHTHTHTHTSHTHTSSS";
    //     let res = run_gridsynth("0.5", "1e-3")?;

    //     // The exact sequence is not stable, but T count is.
    //     assert_eq!(
    //         test_str.chars().filter(|c| c == &'T').count(),
    //         res.chars().filter(|c| c == &'T').count()
    //     );

    //     Ok(())
    // }

    #[test]
    fn parse_ma_form_t_start() -> Result<(), Box<dyn Error>> {
        let ma = "THTSW";

        let (rotations, cliffords) = compile_rots(ma)?;

        assert_eq!(
            rotations,
            vec![
                SingleRotation::Z { dagger: false },
                SingleRotation::X { dagger: false },
            ],
        );

        // Hadamard gets inserted for X basis
        assert_eq!(
            cliffords,
            vec![CliffordGate::H, CliffordGate::S, CliffordGate::W,]
        );

        Ok(())
    }

    #[test]
    fn parse_ma_form_s_start() -> Result<(), Box<dyn Error>> {
        let ma = "SHTSHTXW";

        let (rotations, cliffords) = compile_rots(ma)?;

        assert_eq!(
            rotations,
            vec![
                SingleRotation::X { dagger: true },
                SingleRotation::Z { dagger: false },
            ],
        );

        assert_eq!(
            cliffords,
            vec![CliffordGate::X, CliffordGate::W, CliffordGate::S,]
        );

        Ok(())
    }

    #[test]
    fn parse_t_dag() -> Result<(), Box<dyn Error>> {
        let ma = "TSSS";
        let (rotations, cliffords) = compile_rots(ma)?;
        assert_eq!(rotations, vec![SingleRotation::Z { dagger: false }]);
        assert_eq!(cliffords, vec![CliffordGate::S; 3]);
        Ok(())
    }

    #[test]
    fn synthesize_t() {
        let (rots, cliffs) = synthesize_angle(T_ANGLE, AnglePrecision::lit("1e-6"));
        assert_eq!(rots, vec![SingleRotation::Z { dagger: false }]);
        assert_eq!(cliffs, vec![]);
    }

    #[test]
    fn synthesize_tx() {
        let (rots, cliffords) = synthesize_angle_x(-T_ANGLE, AnglePrecision::lit("1e-6"));
        assert_eq!(rots, vec![SingleRotation::X { dagger: true }]);
        assert_eq!(cliffords, vec![CliffordGate::H, CliffordGate::H]);
    }

    #[test]
    fn synthesize_01() {
        let (rots, _) = synthesize_angle(AnglePrecision::lit("0.1"), AnglePrecision::lit("1e-6"));
        println!("{rots:?}");
        assert!(rots.len() > 30);
    }

    #[test]
    /// Test the highest-precision synthesis of an angle close to T.
    /// This should not give only a T gate because it is too far from a T at the given accuracy.
    fn underflow_precision() {
        let smallest_accuracy = AnglePrecision::from_bits(1);
        let (rots, _) = synthesize_angle(T_ANGLE - 2 * smallest_accuracy, smallest_accuracy);
        println!("{rots:?}");
        assert!(rots.len() > 30);
    }
}

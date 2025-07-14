// Copyright contributors to the Bicycle Architecture Compiler project

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

/// Synthesize a rotation e^{iθZ} in terms of e^{iπ/8Z} and e^{iπ/8X} rotations, followed by Cliffords.
/// The required accuracy must be less than 0.1 and determines ‖e^{iθZ} - U‖ ≤ ε.
pub fn synthesize_angle(
    theta: AnglePrecision,
    accuracy: AnglePrecision,
) -> (Vec<SingleRotation>, Vec<CliffordGate>) {
    assert!(accuracy <= 1e-1);

    // Handle T gate special case
    let sign = theta.is_negative();

    if (AnglePrecision::PI / AnglePrecision::lit("4.0") - theta.abs()).abs() <= accuracy {
        trace!("Close to T: {theta}");
        return (vec![SingleRotation::Z { dagger: sign }], vec![]);
    }

    if let Some(result) = CACHE.try_lock().unwrap().get(&(theta, accuracy)) {
        trace!("Cached angle: {theta}");
        return result.clone();
    }
    debug!("Synthesizing angle: {theta}");

    // Do I need scientific notation here? E.g. for the accuracy.
    let gates = run_pygridsynth(&theta.to_string(), &accuracy.to_string())
        .expect("Pygridsynth should run successfully. Is it installed? See README.");
    let res = compile_rots(&gates)
        .expect("Should be able to parse MA normal form provided by pygridsynth");

    CACHE
        .try_lock()
        .unwrap()
        .insert((theta, accuracy), res.clone());
    res
}

/// Synthesize a rotation e^{iθX}
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

fn run_pygridsynth(angle: &str, accuracy: &str) -> Result<String, io::Error> {
    let res = Command::new("python")
        .args(["-m", "pygridsynth"])
        .arg(angle)
        .arg(accuracy)
        .output()?;

    let mut output = res.stdout;
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

    #[test]
    fn test_05_minus3() -> Result<(), Box<dyn Error>> {
        let test_str =
            "THTHTSHTHTHTHTHTSHTHTHTHTSHTHTSHTSHTSHTSHTSHTSHTSHTHTSHTHTSHTSHTHTSHTSHTHTSHSSWWWWWWW";
        let res = run_pygridsynth("0.5", "1e-3")?;

        // The exact sequence is not stable, but T count is.
        assert_eq!(
            test_str.chars().filter(|c| c == &'T').count(),
            res.chars().filter(|c| c == &'T').count()
        );

        Ok(())
    }

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
    fn synthesize_t() {
        let (rots, cliffs) = synthesize_angle(
            AnglePrecision::PI / AnglePrecision::lit("4.0"),
            AnglePrecision::lit("1e-6"),
        );
        assert_eq!(rots, vec![SingleRotation::Z { dagger: false }]);
        assert_eq!(cliffs, vec![]);
    }

    #[test]
    fn synthesize_tx() {
        let (rots, _) = synthesize_angle_x(
            -AnglePrecision::PI / AnglePrecision::lit("4.0"),
            AnglePrecision::lit("1e-6"),
        );
        assert_eq!(rots, vec![SingleRotation::X { dagger: true }]);
    }
}

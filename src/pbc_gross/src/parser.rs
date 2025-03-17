use core::fmt;
use std::error;
use std::io;

use crate::language::PbcOperation;
use bicycle_isa::Pauli;

#[derive(Clone, Debug)]
pub struct SerializationError;

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error during serialization or deserialization")
    }
}

impl error::Error for SerializationError {}

// Parse a read buffer into a vector of operations
// Could make this an iterable and parse in streaming fashion?
// Should probably write a proper parser for the input language to get line-by-line errors.
// See: e.g. Chumsky for Rust (but what about other languages? Would a Yacc grammar be easier?)
pub fn parse_buf<R: io::Read>(readme: R) -> Result<Vec<PbcOperation>, Box<dyn error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .comment(Some(b'#'))
        .from_reader(readme);
    let mut ops = vec![];
    for result in rdr.records() {
        let record = result?;
        let operation = match &record[0] {
            "m" => {
                let mut basis = Vec::new();
                for ch in record[1].chars() {
                    basis.push(Pauli::try_from(&ch)?);
                }
                if basis.len() % 11 != 0 {
                    return Err(Box::from(SerializationError));
                }

                Ok(PbcOperation::Measurement {
                    basis,
                    flip_result: &record[2] == "-",
                })
            }
            "r" => {
                let mut basis = Vec::new();
                for ch in record[1].chars() {
                    basis.push(Pauli::try_from(&ch)?);
                }

                if basis.len() % 11 != 0 {
                    return Err(Box::from(SerializationError));
                }

                let angle: f64 = record[2].parse()?;
                Ok(PbcOperation::Rotation { basis, angle })
            }
            _ => Err(SerializationError),
        };
        ops.push(operation?);
    }

    Ok(ops)
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use super::*;

    use Pauli::{I, X, Y, Z};

    #[test]
    fn simple_parse() -> Result<(), Box<dyn Error>> {
        let input = "r,xxiiiiiiiii,-0.125
r,izziiiiiiii,0.25
r,yyyiiiiiiii,-0.25
# Now we measure
m,ziiiiiiiiii,-
m,iziiiiiiiii,+
";
        let result = parse_buf(input.as_bytes())?;

        let expected = vec![
            PbcOperation::Rotation {
                basis: vec![X, X, I, I, I, I, I, I, I, I, I],
                angle: -0.125,
            },
            PbcOperation::Rotation {
                basis: vec![I, Z, Z, I, I, I, I, I, I, I, I],
                angle: 0.25,
            },
            PbcOperation::Rotation {
                basis: vec![Y, Y, Y, I, I, I, I, I, I, I, I],
                angle: -0.25,
            },
            PbcOperation::Measurement {
                basis: vec![Z, I, I, I, I, I, I, I, I, I, I],
                flip_result: true,
            },
            PbcOperation::Measurement {
                basis: vec![I, Z, I, I, I, I, I, I, I, I, I],
                flip_result: false,
            },
        ];

        assert_eq!(expected, result);

        Ok(())
    }
}

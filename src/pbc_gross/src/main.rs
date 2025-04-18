use std::{
    error,
    io::{self, Read},
};

use pbc_gross::language::PbcOperation;

use io::Write;

use log::debug;
use pbc_gross::{optimize, PathArchitecture};
use serde_json::Deserializer;

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let reader = io::stdin().lock();

    // Support some streaming input from Stdin
    // The following works for (a weird version of) JSON:
    let de = Deserializer::from_reader(reader);
    let ops = de.into_iter::<PbcOperation>().map(|op| op.unwrap());
    let mut ops = ops.peekable();

    // Set the architecture based on the first operation
    let first_op = ops.peek();
    let architecture = if let Some(op) = first_op {
        PathArchitecture::for_qubits(op.basis().len())
    } else {
        // No ops, may as well terminate now.
        return Ok(());
    };

    let compiled = ops.flat_map(|op| op.compile(&architecture));

    let optimized_ops = optimize::remove_duplicate_measurements(compiled);
    let mut handle = io::stdout();
    for op in optimized_ops {
        for gate in op {
            debug!("{gate:?}");
            serde_json::to_writer(&mut handle, &gate)?;
            writeln!(&mut handle)?;
        }
    }

    Ok(())
}

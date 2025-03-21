use std::{error, io};

use io::Write;

use log::debug;
use pbc_gross::{optimize, parser::PbcParser, PathArchitecture};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let read = io::stdin();
    let mut parser = PbcParser::new(read);
    let ops = parser.stream();

    let mut peek_ops = ops.peekable();

    // Set the architecture based on the first operation
    let first_op = peek_ops.peek();
    let architecture = if let Some(op) = first_op {
        PathArchitecture {
            data_blocks: (op.basis().len() + 1).div_ceil(11),
        }
    } else {
        // No ops, may as well terminate now.
        return Ok(());
    };

    let compiled = peek_ops.flat_map(|op| op.compile(&architecture));

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

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

use std::{env, error::Error, io, path::Path};

use bicycle_numerics::{
    OutputData,
    model::{FAKE_SLOW, GROSS_1E3, GROSS_1E4, Model, TWO_GROSS_1E3, TWO_GROSS_1E4},
};
use log::{debug, trace};

use bicycle_compiler::operation::Operation;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;

#[derive(Debug, Clone, Eq, PartialEq, ValueEnum)]
enum ModelChoices {
    /// Gross codes with physical noise rate p=10^-3
    #[clap(name = "gross_1e-3")]
    Gross1e3,
    /// Gross codes with physical noise rate p=10^-4
    #[clap(name = "gross_1e-4")]
    Gross1e4,
    /// Two-gross codes with physical noise rate p=10^-3
    #[clap(name = "two-gross_1e-3")]
    TwoGross1e3,
    /// Two-gross codes with physical noise rate p=10^-4
    #[clap(name = "two-gross_1e-4")]
    TwoGross1e4,
    /// A model that has no physical noise, p=0, and worst-case timing information between all of the previous models
    #[clap(name = "fake_slow")]
    FakeSlow,
    /// A model read from a TOML file, identified by its file name
    #[value(skip)]
    Custom { file_name: String, model: Model },
}

impl ModelChoices {
    fn model(&self) -> Model {
        match self {
            Self::Gross1e3 => GROSS_1E3,
            Self::Gross1e4 => GROSS_1E4,
            Self::TwoGross1e3 => TWO_GROSS_1E3,
            Self::TwoGross1e4 => TWO_GROSS_1E4,
            Self::FakeSlow => FAKE_SLOW,
            Self::Custom { model, .. } => *model,
        }
    }
}

/// Load the model from the input string; either load the .toml file, or lookup the preset.
fn parse_model(model: &str) -> Result<ModelChoices, String> {
    let path = Path::new(model);
    if path.extension().is_some_and(|ext| ext == "toml") {
        let model = Model::from_file(path).map_err(|e| e.to_string())?;
        let file_name = path.file_name().unwrap().to_string_lossy().into_owned();
        return Ok(ModelChoices::Custom { file_name, model });
    }
    ModelChoices::from_str(model, false)
        .map_err(|_| "expected a .toml file or a preset (see --help)".to_owned())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Output {
    /// The code of a preset, or the file name of a model config
    code: String,
    /// Physical error rate, unknown for a model config
    p: Option<f64>,
    i: usize,
    qubits: usize,
    idles: u64,
    t_injs: u64,
    automorphisms: u64,
    measurements: u64,
    joint_measurements: u64,
    measurement_depth: u64,
    end_time: u64,
    total_error: f64,
}

impl Output {
    pub fn new(model: &ModelChoices, data: OutputData) -> Self {
        let (code, p) = match model {
            ModelChoices::Gross1e3 => ("gross", Some(1e-3)),
            ModelChoices::Gross1e4 => ("gross", Some(1e-4)),
            ModelChoices::TwoGross1e3 => ("two-gross", Some(1e-3)),
            ModelChoices::TwoGross1e4 => ("two-gross", Some(1e-4)),
            ModelChoices::FakeSlow => ("fake", Some(0.0)),
            ModelChoices::Custom { file_name, .. } => (file_name.as_str(), None),
        };

        Self {
            code: code.to_owned(),
            p,
            i: data.i,
            qubits: data.qubits,
            idles: data.idles,
            t_injs: data.t_injs,
            automorphisms: data.automorphisms,
            measurements: data.measurements,
            joint_measurements: data.joint_measurements,
            measurement_depth: data.measurement_depth,
            end_time: data.end_time,
            total_error: data.total_error,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Cli {
    /// Number of logical qubits in the input circuit (do not include pivot ancillas).
    qubits: usize,
    /// Choose which architecture the circuit is run on.
    ///
    /// Either a path to a .toml file with a custom timing and error model, or one of the presets:
    ///
    /// - gross_1e-3:     Gross codes with physical noise rate p=10^-3
    /// - gross_1e-4:     Gross codes with physical noise rate p=10^-4
    /// - two-gross_1e-3: Two-gross codes with physical noise rate p=10^-3
    /// - two-gross_1e-4: Two-gross codes with physical noise rate p=10^-4
    /// - fake_slow:      A model that has no physical noise, p=0, and worst-case timing information between all of the previous models
    ///
    /// A .toml file needs a [timing] table (in syndrome cycles) and an [error] table (logical
    /// error probabilities *as strings*), each with the keys idle, shift, inmodule, intermodule and
    /// t_inj.
    #[arg(value_parser = parse_model, verbatim_doc_comment)]
    model: ModelChoices,
    /// Set a limit to the error rate when the numerics should halt
    #[arg(short = 'e', long)]
    max_error: Option<f64>,
    /// Set a limit to the number of input lines (PBC gates) before halting.
    #[arg(short = 'i', long)]
    max_iter: Option<usize>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // By default log INFO.
    if env::var("RUST_LOG").is_err() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { env::set_var("RUST_LOG", "info") };
    }
    env_logger::init();

    let cli = Cli::parse();
    trace!("Number of qubits: {}", cli.qubits);
    let model = cli.model.model();

    let reader = io::stdin().lock();

    // Support some streaming input from Stdin
    // The following works for (a weird version of) JSON:
    let de = Deserializer::from_reader(reader);
    let ops = de.into_iter::<Vec<Operation>>().map(|op| op.unwrap());

    let architecture = bicycle_compiler::PathArchitecture::for_qubits(cli.qubits);

    let output_data = bicycle_numerics::run_numerics(ops, architecture, model);

    // Stop when error exceeds set value (if set) or iterations gets too large (if set)
    let short_data = output_data
        .take_while(|data| {
            cli.max_error
                .is_none_or(|max_err| data.total_error <= max_err)
        })
        .take_while(|data| cli.max_iter.is_none_or(|max_iter| data.i <= max_iter));

    let mut outputs = short_data.map(|data| Output::new(&cli.model, data));
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let err = outputs.try_for_each(|output| wtr.serialize(output));
    debug!("Exited with {err:?}");

    Ok(())
}

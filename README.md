# Bicycle Architecture Compiler

[Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094) presents the Bicycle Architecture - a modular architecture for fault-tolerant quantum computation based on bivariate bicycle codes. The analysis features:

 - A universal Instruction Set Archirecture (ISA) of logical gates in bivariate bicyle codes
 - Circuit-level noise benchmarks of all the ISA instructions
 - An end-to-end compilation scheme from Pauli-Based Computation (PBC) to the bicycle ISA

This compiler prototype implements and benchmarks this end-to-end compilation scheme so that it may inform future Qiskit development for fault-tolerant architectures.
Some example use cases are:
 - Random PBC circuits as a form of worst-case Clifford+T quantum circuits. See [`./notebooks/random_circuits.ipynb`](./notebooks/random_circuits.ipynb)
 - User-specified PBC circuits supporting arbitrary-angle rotations, see [`./notebooks/custom_circuits.ipynb`](./notebooks/custom_circuits.ipynb)

## Installation

### Platform support

This software is tested on some Linux and macOS platforms.

### Rust

We recommend using [rustup](https://www.rust-lang.org/tools/install) to install a rust toolchain.
Then run
```sh
shell> cargo build --release
```

### Python

Please ensure that a `python` (python3) executable is available in your path.
We recommend using a virtual environment, such as `venv` or `pyenv`. For example

```sh
shell> python -m venv ./.venv
shell> source ./.venv/bin/activate # for macos or linux running `bash` or `zsh`
```

Or if you choose `pyenv`,
```sh
shell> pyenv virtualenv pbc-gross
shell> pyenv local pbc-gross
```

Once your virtual environment is activated, you can install required and optional packages.
Required packages may be installed like this
```sh
shell> pip install -r requirements.txt
```

The compiler depends on the the package `pygridsynth~=1.1`
for synthesizing rotations by angles other than $\pm\pi/4$.
To test your installation, the following command should succeed
```sh
shell> python -m pygridsynth 0.5 1e-3
```
printing something like (the exact output may differ)
```
THTHTSHTHTHTHTHTSHTHTHTHTSHTHTSHTSHTSHTSHTSHTSHTSHTHTSHTHTSHTSHTHTSHTSHTHTSHSSWWWWWWW
```

### Optional dependencies

To run the notebooks,
we require the packages `numpy`, `matplotlib`, and `jupyter`.
These can be installed via
```sh
shell> pip install -r optional_dependencies.txt
```

Furthermore, the following commandline applications are helpful and may be required for some functionality:
* `jq` - commandline JSON processor, or parsing JSON to a newline-delimited input.
* GNU `parallel` in order to
  run [./scripts/run_random_numerics.sh](./scripts/run_random_numerics.sh) in parallel.

## Usage

For a workflow for generating benchmarks, see [scripts/README.md](scripts/).

### Folder map

```
pbc-compiler/
├── analysis/                      # Python modules for postprocessing and steering
├── scripts/                       # Scripts for generating data (has some overlap with analysis)
├── notebooks/                     # Notebook for running and plotting a random circuit experiment.
├── data/                          # Cached measurement tables, and random circuit data
└── crates/
    ├── bicycle_common/            # Definition of bicycle ISA
    ├── bicycle_benchmark/         # Random generation of PBC circuits
    ├── bicycle_cliffords/         # Clifford gate implementation via brute-force search
    ├── bicycle_compiler/          # PBC to Bicycle ISA compiler
    ├── bicycle_numerics/          # Simple noise simulation and stats collections
    └── bicycle_random_numerics/   # Benchmarking via random PBC circuits
```

### Crates

The PBC compiler packages are located under `crates/`.

1. [`bicycle_common`](./crates/bicycle_common) define the bicycle instructions, which are used as a shared language.
1. [`bicycle_cliffords`](./crates/) searches & builds a table to implement Clifford gates on Gross and Two Gross codes using the least rotations.
1. [`bicycle_compiler`](./crates/) the main compiler that takes in a PBC circuit and outputs a circuit using bicycle instructions.
1. [`bicycle_numerics`](./crates/) adds timing information and collects data about the compiled circuits.
1. [`bicycle_benchmark`](./crates/) generates random circuits of Pauli-generated rotations or measurements.
1. [`bicycle_random_numerics`](./crates/) a wrapper package for collecting data faster (basically runs `cargo run --package benchmark <args> | cargo run --package pbc_gross <args> | cargo run --package numerics <args>` without (de)serialization overhead).

Each crate has more info in their respective READMEs.


## Testing

To test in release mode try this
```sh
shell> cargo test -r
```

The script [./scripts/local_QA.sh](./scripts/local_QA.sh) runs quality assurance tests locally.
This includes test, rustfmt, and clippy.
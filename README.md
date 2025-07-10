# IBM Bicycle Architecture Compiler

[Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094) presents the Bicycle Architecture - a modular architecture for fault-tolerant quantum computation based on bivariate bicycle codes. The analysis features:

 - A universal Instruction Set Archirecture (ISA) of logical gates in bivariate bicyle codes
 - Circuit-level noise benchmarks of all the ISA instructions
 - An end-to-end compilation scheme from Pauli-Based Computation (PBC) to the bicycle ISA

This repository implements and benchmarks this end-to-end compilation scheme. We provide facilities for:
 - Random PBC circuits as a form of worst-case Clifford+T quantum circuits. See
     - [`./crates/bicycle_random_numerics`](./crates/bicycle_random_numerics)
     - [`./scripts/generate_tables_and_random_numerics.py`](./scripts/generate_tables_and_random_numerics.py)
     - [`./notebooks/Circuit_failure_probability.ipynb`](./notebooks/Circuit_failure_probability.ipynb)
 - User-specified PBC circuits supporting arbitrary-angle rotations, see `crates/bicycle_numerics`

## Installation

See information on installing rust [below](#rust)
```sh
shell> cargo build --release
```

## Platform support

This software is tested on some Linux and macOS platforms.

## Dependencies

### Rust

We recommend using [rustup](https://www.rust-lang.org/tools/install) to install a rust toolchain

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

### Python dependencies

Once your virtual environment is activated, you can install required and optional packages.

Required packages (currently only `pygridsynth~=1.1`) may be installed like this
```sh
shell> pip install -r requirements.txt
```

#### Gridsynth

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

#### Optional Python packages

These packages are `numpy`, `matplotlib`, and `jupyter`. They are needed to run the notebook.
```sh
shell> pip install -r optional_dependencies.txt
```

#### Dependencies not availble via `pip` or `cargo`.

* `jq` - commandline JSON processor

### Optional dependencies

* You must have installed either `python3` or GNU `parallel` in order to
  run [./scripts/run_random_numerics.sh](./scripts/run_random_numerics.sh).

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

### PBC Compiler

The PBC compiler packages are located under `crates/`.
There are various packages:

1. [`bicycle_common`](./crates/bicycle_common) define the bicycle instructions, which are used as a shared language.
1. [`bicycle_benchmark`](./crates/) generates random circuits of Pauli-generated rotations or measurements.
1. [`bicycle_cliffords`](./crates/) searches & builds a table to implement Clifford gates on Gross and Two Gross codes using the least rotations.
1. [`bicycle_compiler`](./crates/) the main compiler that takes in a PBC circuit and outputs a circuit using bicycle instructions.
1. [`bicycle_numerics`](./crates/) adds timing information and collects data about the compiled circuits.
1. [`bicycle_random_numerics`](./crates/) a wrapper package for collecting data faster (basically runs `cargo run --package benchmark <args> | cargo run --package pbc_gross <args> | cargo run --package numerics <args>` without (de)serialization overhead).

Each package has more info in their respective READMEs.

### Installation

```sh
shell> cargo build -r

shell> python -m venv venv
shell> source venv/bin/activate
shell> pip install -e .
```

### Crate Dependencies

This information is provide to aid in understanding the role of each crate.

```
bicycle_common v0.0.1

bicycle_cliffords v0.0.1
└── bicycle_common v0.0.1

bicycle_benchmark v0.0.1
├── bicycle_common v0.0.1
└── bicycle_compiler v0.0.1

bicycle_compiler v0.0.1
├── bicycle_common v0.0.1
└── bicycle_cliffords v0.0.1

bicycle_numerics v0.0.1
├── bicycle_benchmark v0.0.1
├── bicycle_common v0.0.1
└── bicycle_compiler v0.0.1

bicycle_random_numerics v0.0.1
├── bicycle_benchmark v0.0.1
├── bicycle_common v0.0.1
├── bicycle_cliffords v0.0.1
└── bicycle_compiler v0.0.1
```

### Testing

To test in release mode try this
```sh
shell> cargo test -r
```

The script [./scripts/local_QA.sh](./scripts/local_QA.sh) runs quality assurance tests locally.
This includes test, rustfmt, and clippy.

The tests run almost twice as fast if you use `nextest`.
You can installl nextest like this
```sh
shell> cargo install cargo-binstall
shell> cargo binstall cargo-nextest --secure
```

The script [./scripts/local_QA.sh](./scripts/local_QA.sh) will use `nextest` if it is
installed.

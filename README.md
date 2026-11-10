# Bicycle Architecture Compiler

[Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094) presents the Bicycle Architecture - a modular architecture for fault-tolerant quantum computation based on bivariate bicycle codes. The analysis features:

 - A universal Instruction Set Architecture (ISA) of logical gates in bivariate bicycle codes
 - Circuit-level noise benchmarks of all the ISA instructions
 - An end-to-end compilation scheme from Pauli-Based Computation (PBC) to the bicycle ISA

This compiler prototype implements and benchmarks this end-to-end compilation scheme so that it may inform future Qiskit development for fault-tolerant architectures.
Some example use cases are:
 - Random PBC circuits as a form of worst-case Clifford+T quantum circuits. See [`./notebooks/random_circuits.ipynb`](./notebooks/random_circuits.ipynb)
 - User-specified PBC circuits supporting arbitrary-angle rotations, see [`./notebooks/custom_circuits.ipynb`](./notebooks/custom_circuits.ipynb)
 - Qiskit circuits to Gross code ISA, see [`./scripts/qiskit_demo.sh`](./scripts/qiskit_demo.sh)

## Installation

This section, and the following sections, deal with the Rust installation. The Python interface is installed separately, see [Python interface](#python-interface).

### Rust

We recommend using [rustup](https://www.rust-lang.org/tools/install) to install a rust toolchain.
Then run
```sh
shell> cargo build --release
```
Binaries are built and placed in `./target/release/`.
In the documentation we use these binaries without including their (relative) path.
Alternatively, you can use `cargo run --release --package <packageName> -- <arguments>`.

### Gridsynth

Angles other than $\pm\pi/4$ are synthesized with the built-in [rsgridsynth](https://github.com/qiskit-community/rsgridsynth) library.
No external `gridsynth` installation is required.

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

This Rust workspace consists of many Rust crates (packages) that can be used either as library
or, in many cases, also as a binary.
The contents of the repository as summarized as follows.

```
pbc-compiler/
├── analysis/                      # Python modules for postprocessing and steering
├── scripts/                       # Scripts for generating data (has some overlap with analysis)
├── notebooks/                     # Notebook for running and plotting a random circuit experiment.
├── data/                          # Cached measurement tables, and random circuit data
└── crates/
    ├── bicycle_common/            # Common definitions. Bicycle instructions.
    ├── bicycle_benchmark/         # Random generation of PBC circuits
    ├── bicycle_cliffords/         # Clifford gate implementation via search
    ├── bicycle_compiler/          # PBC to bicycle circuit compiler
    ├── bicycle_numerics/          # Additive noise estimates and stats collection
    ├── bicycle_random_numerics/   # Benchmarking via random PBC circuits
    └── pyext/                     # Python FFI (not included in the default workspaces)
```

Each crate has more info in their respective READMEs.

Many binary crates can be used via their compiled binaries (obtained by `cargo build` or `cargo run --package <package>`).
For an example workflow that generates benchmarks see [scripts/README.md](scripts/),
and more advanced examples are illustrated by Jupyter notebooks in `./notebooks/`.


### Testing

**Rust**

To test in release mode, run

```sh
shell> cargo test --release
```

The script [./scripts/local_QA.sh](./scripts/local_QA.sh) runs quality assurance tests locally.
This includes test, rustfmt, and clippy.

**Python**

To test the Python bindings and lint the tests, run the following after having installed
the package

```sh
shell> make lint
shell> make test
```

## Python interface

Separately from the Rust crates, a minimal Python interface is provided by the
`bicycle_architecture_compiler` package, which can be installed from PyPI via
```sh
shell> pip install bicycle_architecture_compiler
```
or from a local clone via `pip install .`.
Note that installing in editable mode compiles the Rust crates in debug mode, which can be
rebuilt in release mode via
```sh
shell> python setup.py build_rust --inplace --release
```

The package exposes two functions, which compile a Qiskit `QuantumCircuit` of Pauli-based
computation (PBC) instructions to the bicycle ISA and estimate its error and resources:

 - `estimate` uses one of the preset timing and error models (`"gross"` or `"two_gross"` code at
   a physical error rate of `"1e-3"` or `"1e-4"`),
 - `estimate_from_config` reads a custom timing and error model from a TOML file,
   see e.g. [`./test/error_models/gross3.toml`](./test/error_models/gross3.toml).

For example:

```python
from qiskit.circuit import QuantumCircuit
from qiskit.transpiler import generate_preset_pbc_pass_manager
from bicycle_architecture_compiler import estimate

bell = QuantumCircuit(2)
bell.h(0)
bell.cx(0, 1)
pbc = generate_preset_pbc_pass_manager().run(bell)  # convert to PBC instructions

data = estimate(pbc, "gross", "1e-3", "1e-10")  # code, physical error, gridsynth accuracy
print(data["total_error"], data["end_time"], data["counts"])
```
or
```python
from bicycle_architecture_compiler import estimate_from_config

data = estimate_from_config(pbc, "gross", "model.toml", "1e-10")
```
The measurement table is generated in the local directory on first use, unless a path is passed
via the `measurement_table` argument.



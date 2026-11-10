## [Py 0.1.0] - 2026-11-10

This is the first release of the Python package `bicycle_architecture_compiler`.

### 🚀 Features

- Add `estimate` and `estimate_from_config` to run the compiler on a 
  ``qiskit.QuantumCircuit`` containing only PBC instructions and return the accumulated
  numerics (error, end time and operation counts) of the circuit. 

### 🧪 Testing

- A minimal Python test suite can be run via `make test` and linted via `make lint`.

### ⚙️ Miscellaneous Tasks

- Python stubs for Python API exposed from Rust can be generated via `make stubs`.

## [0.3.0] - 2026-10-29

### 🚀 Features

- *(compiler)* [**breaking**] Replace external gridsynth with rsgridsynth (#29)

### 🐛 Bug Fixes

- *(compiler)* Remove dbg statement (#21)

### 📚 Documentation

- *(common)* Remove reference to internal doc
- *(compiler)* Use the exp(i P φ/2) angle convention consistently (#27)

### 🧪 Testing

- Set optimization level to 1 to ensure tests run fast enough
- *(compiler)* Run rsgridsynth tests serially (#30)
## [0.2.1] - 2026-03-30

### 🚀 Features

- *(compiler)* Add rsgridsynth backend for small angle synthesis (#20)

### ⚙️ Miscellaneous Tasks

- *(release)* Prepare changelog
- *(release)* Ensure unified version on release
- Release
## [0.2.0] - 2026-03-26

### 🚀 Features

- Add multi-block examples and docs (#4)
- Add benchmarks and correctness tests

### 🐛 Bug Fixes

- Auto-create tmp and data directories (#3)

### 🧪 Testing

- Make benchmark correctness suite standalone
- *(cliffords)* Drop duplicated bench correctness suite
- *(compiler)* Drop duplicated bench correctness suite

### ⚙️ Miscellaneous Tasks

- Release
## [0.1.0] - 2025-07-31

### 💼 Other

- Scripts/, data/, tmp/

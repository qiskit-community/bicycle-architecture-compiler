## [0.2.1] - 2026-03-30

### 🚀 Features

- *(compiler)* Add rsgridsynth backend for small angle synthesis (#20)

## [0.2.0] - 2026-03-26

### 🚀 Features

- Add multi-block examples and docs (#4)
- Add benchmarks and correctness tests
- **breaking** Use gridsynth to ignore global phase. Remove pygridsynth.
- Fix error in automorphism action. Add test for commutation relations.
- **breaking** Remove default max-error and max-iter flags. Fixes #7

### 🐛 Bug Fixes

- Auto-create tmp and data directories (#3)
- Reverse post-rotation order. Fixes #19
- Remove incorrect T angle comparison.
- Fix timing. Address #1

### 💼 Other

- **breaking** Upgrade to Rust version 2024
- Create CODE_OF_CONDUCT.md
- Simplify automorphism construction
- Add flag to Cliffords to generate 11q/12q tables
- Add 0.1 synthesis example circuit
- Add Qiskit circuit parser tool
- Add Qiskit example to custom_circuits notebook
- Use criterion for benchmarking compiler

### 🧪 Testing

- Check native measurements have 0 rotations in table

### ◀️ Revert

- Revert compiler auto-dir creation; keep script mkdir

## [0.1.0] - 2025-07-31

First release



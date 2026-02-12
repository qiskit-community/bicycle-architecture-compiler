# `bicycle_common`

This crate implements common functionality of the Bicycle Architecture Compiler.
Its main contribution are the logical instruction of bivariate bicycle codes.
It supports instantiating instructions and some methods for manipulation, random sampling, etc.

`Pauli` is also defined in this crate.

### Summary of the logical instructions
We list 

> type `display name` description

of the _bicycle instructions_ used in [Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094):

* __SyndromeCycle__ `sc` Idle operation. Not used explicitly by the compiler, only inferred with timing information in the numerics.
* __Automorphism__ `aut(_,_)` Perform a unitary automorphism gate, see Section 9.2 of [arXiv:2308.07915](https://arxiv.org/abs/2308.07915).
* __Measure__ `meas(_,_)` Measure the first and/or seventh qubit.
* __JointMeasure__ `jMeas(_,_)` One half of a joint measurement between code modules. Measure the first and/or seventh qubit of each module.
* __TGate__ `T(_,_,_)` Apply $exp(i P\pi/8)$ for $P \in \set{X,Z,Y}$ on the first or seventh qubit.

Note that we also define other logical instructions of bivariate bicycle codes that are not used (explicitly) by the compiler:

* __CSSInitZero__ `init0` Initialize all logical qubits in a code module to `|0>`.
* __CSSInitPlus__ `init+` Initialize all logical qubits in a code module to `|+>`.
* __DestructiveZ__ `measZ` Measure all logical qubits in a code module in Z basis.
* __DestructiveX__ `measX` Measure all logical qubits in a code module in X basis.
* __ParallelMeasure__`pMeas(_)` Measure the first and seventh qubit independently.
* __JointBellInit__ `jBell` One half of an instruction acting on two code modules. Initialize 12 Bell pairs using transversal CX.
* __JointTransversalCX__ `jCnot` One half of an instruction acting on two code modules. Perform 12 CX gates via transversal CX.
* __InitT__ `initT` Initialize all logical qubits in a code module to `|T>`, at physical noise rate.

### Toric parity-check helpers

`bicycle_common::parity_check` includes a minimal parity-check toolbox:

* `gross_toric_parity_checks()` and `two_gross_toric_parity_checks()` build toric `Hx` / `Hz`.
* `BinaryMatrix::to_csr()` converts to `sprs::CsMat<u8>` for decoder interop.
* `syndrome(&h, &error)` computes `H * e^T` over GF(2) with input validation.

Example:

```rust
use bicycle_common::parity_check::{gross_toric_parity_checks, syndrome};

let checks = gross_toric_parity_checks();
let mut error = vec![0u8; checks.hx.cols()];
error[0] = 1;
let s = syndrome(&checks.hx, &error)?;
assert_eq!(s.len(), checks.hx.rows());
# Ok::<(), bicycle_common::parity_check::SyndromeError>(())
```

# `bicycle_common`

This crate implements the instruction set of the bivariate bicycle codes. It supports
instantiating instructions and some methods for manipulation, random sampling, etc.

`pub enum Pauli` is defined in this crate as well because `pub enum BicycleISA` depends on it.
However `Pauli` is also used directly in other crates, and so has public visibility.

Description of the instruction set of logical operations in bivariate bicycle codes,
see [`pub enum BicycleISA`](https://github.ibm.com/ibm-q-research/bicycle-architecture-compiler/blob/e4b18b9e850ec84c78ab7366058705a89cdb18b7/crates/bicycle_common/src/lib.rs#L255).

### Summary of the gates

On the left is the name of the rust `enum` variant. On the right is the display format.

* __SyndromeCycle__ `sc` Idle operation.
* __CSSInitZero__ `init0` Initialize all logical qubits in block to `|0>`.
* __CSSInitPlus__ `init+` Initialize all logical qubits in block to `|+>`.
* __DestructiveZ__ `measZ` Measure all logical qubits in block in Z basis.
* __DestructiveX__ `measX` Measure all logical qubits in block in X basis.
* __Automorphism__ `aut(_,_)` Perform a unitary automorphism gate, see Section 9.2 of [arXiv:2308.07915](https://arxiv.org/abs/2308.07915).
* __Measure__ `meas(_,_)` Measure the first and/or seventh qubit using the Logic Processing Unit (LPU).
* __JointMeasure__ `jMeas(_,_)` One half of a two-block gate. Measure the first and/or seventh qubit.
* __ParallelMeasure__`pMeas(_)` Measure the first and seventh qubit independently.
* __JointBellInit__ `jBell` One half of a two-block gate. Initialize 12 Bell pairs using transversal CX.
* __JointTransversalCX__ `jCnot` One half of a two-block gate. Perform 12 CX gates via transversal CX.
* __InitT__ `initT` Initialize all logical qubits in a block to `|T>`, at physical noise rate.
* __TGate__ `T(_,_,_)` Apply exp(i pi/8 P) for P either X or Z on first or seventh qubit.

# `bicycle_common`

Description of the instruction set of logical operations in bivariate bicycle codes, see `pub enum BicycleISA`.

Summary of the gates:

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
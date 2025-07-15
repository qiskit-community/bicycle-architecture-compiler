# `bicycle_random_numerics`

This crates generates circuits $\prod_j \exp(i \frac{\pi}{8} P_j)$, for random Pauli matrices $P_j$.
It uses `bicycle_benchmark` to generate the Pauli-generated rotations,
compiles them using `bicycle_compiler`,
and immediately feeds the resulting circuits into `bicycle_numerics`.

This workflow was used in Section 4 and Appendix A.10 of [Tour de Gross (2506.03094)](https://arxiv.org/abs/2506.03094) for benchmarking random Clifford+T circuits.

This package exists because seralizing and deserializing the output of `bicycle_compilers` from JSON incurs significant overhead.
It is also a good illustration of how the other crates can be used as libraries.
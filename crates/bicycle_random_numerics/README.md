# `bicycle_random_numerics`

This simple package generates circuits composed of $\exp(i \frac{\pi}{8} P)$ for random Pauli matrices $P$. It compiles them using `bicycle_compiler`, and immediately feeds the resuling circuits into `bicycle_numerics`.

This workflow was used in Section 4 and Appendix A.10 of [Tour de Gross (2506.03094)](https://arxiv.org/abs/2506.03094) for benchmarking random Clifford+T circuits.

This package exists because seralizing and deserializing the output of `bicycle_compilers` from JSON does not run sufficiently quickly.
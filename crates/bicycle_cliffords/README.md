# Gross Code CLiffords

As discussed in [this paper by Cross et al.](https://arxiv.org/abs/2407.18393), section 4.3,
we can construct logical Clifford rotations and measurements from the measurements enabled by the ancilla system attached to a Gross code.
In this repository,
we implement some searches to find faster implementations of arbitrary Clifford rotations and measurements.

We consider implementing Clifford rotations, $e^{i \frac{\pi}{4} P}$ for Pauli string $P$.
and Clifford measurements that are defined by the measurement basis

$$\set{I + (-1)^b e^{i \frac{\pi}{4} P}}_{b = 0}^1.$$

By injecting a magic state $\ket T \coloneqq T\ket +$ [^game] (Fig. 7),
a Clifford measurements of $Z \otimes P$ can implement a $\pi/8$ rotation of $P$ up to Clifford corrections.

[^game]: Game of Surface Codes [doi](https://doi.org/10.22331/q-2019-03-05-128)

## How to construct rotations and measurements

TODO
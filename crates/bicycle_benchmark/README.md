# bicyle_benchmark

The executable `bicycle_benchmark` prints an infinite stream of `nqubit`
Pauli-based rotations to `stdout`.

The executable is invoked like this.
```sh
../../target/release/bicycle_benchmark -- nqubits > /dev/null
```

An example of a rotation instruction printed for `nqubits` equal to `3` is
```json
{"Rotation":{"basis":["I","Y","Y"],"angle":"0.78539816339744830961566084581"}}
```

This executable provide and example of the library function provided by this crate.
The executable itself is not used elsewhere.

But the library function `bicycle_benchmark::random::random_rotations` is used
by the executable `bicycle_random_numerics` found in the crate of the same name.

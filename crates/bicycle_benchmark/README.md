# `bicyle_benchmark`

The executable `bicycle_benchmark` prints an infinite stream random of `nqubit`
Pauli-based rotations to `stdout`.

The executable is invoked like this.
```sh
../../target/release/bicycle_benchmark -- nqubits > /dev/null
```

An example of a rotation instruction printed for `nqubits` equal to `3` is
```json
{"Rotation":{"basis":["I","Y","Y"],"angle":"0.78539816339744830961566084581"}}
```

The executable `bicycle_benchmark` can be used like this
```sh
cargo run --package bicycle_benchmark <args> | cargo run --package bicycle_compiler <args> | cargo run --package bicycle_numerics <args>
```

However, the executable `bicycle_random_numerics`, found in the crate of the same name,
instead uses the library function `bicycle_benchmark::random::random_rotations`.
This is in order to avoid the overhead of (de)serialization to JSON.

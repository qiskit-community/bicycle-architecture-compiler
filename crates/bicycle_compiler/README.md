# `bicycle_compiler`

This crate compiles Pauli-Based Computation (PBC) circuits into circuits built from the Bicycle ISA defined in the `bicycle_common` crate.

### Input Program

See Appendix A.9 of [Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094) for a review of PBC. PBC refers to a variety of quantum circuit families and compilation methods in the literature, but generally consists of gates acting on $n$ qubits that are defined by $n$-qubit Pauli matrices. For the purposes of this repository, a PBC circuit consists of multi-qubit Pauli rotations $\exp(i \phi P)$, and multi-qubit Pauli measurements.

The input program must be in a PBC form.
We choose the following format ([inspiration](https://doi.org/10.5281/zenodo.11391890)).
```json
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","Y"],"angle":"0.125"}}
{"Rotation":{"basis":["Z","Z","I","I","I","I","I","I","I","I","I","I"],"angle":"0.5"}}
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","I"],"angle":"-0.125"}}
{"Measurement":{"basis":["Z","X","I","I","I","I","I","I","I","I","I","I"],"flip_result":true}}
{"Measurement":{"basis":["X","I","I","I","I","Z","I","I","I","I","I","I"],"flip_result":false}}

```

The above format gives examples of the only two operations allowed in such a PBC program: rotations and measurements.
All operations should act on the same number of logical qubits.
Rotations $\exp(i \phi P)$ are specified by objects with with a `Rotation` dictionary, which has the `basis` field for the Pauli $P$ and the `angle` field for $\phi$.
Measurements are specified by objects with a `Measurement` dictionary, which also has a `basis` field, and whether the resulting measurement result should be flipped (currently not used). The `flip_result` is intended to support a future implementation of 'measurement projections' as defined in equation (1) of [arXiv:2506.03094](https://arxiv.org/abs/2506.03094) in Section 3.

### Running the program
You can pipe a program of the above format into the binary by running

```
cat example/simple.json | jq --compact-output '.[]' | cargo run --release -- gross
```

This may take a while because a Clifford synthesis table will be built in-memory.
See below on how to speed this up.

### Output

The output looks like
```json
[
    [[0,{"Measure":{"p1":"Z","p7":"I"}}]],
    [[0,{"Automorphism":{"x":3,"y":2}}]],
    ...,
    [
        [0,{"JointMeasure":{"p1":"Z","p7":"I"}}],
        [1,{"JointMeasure":{"p1":"Z","p7":"I"}}]
    ],
    ...
]
[
    ...
]
[ ... ]
[ ... ]
...
```
This output is similarly delineated by newlines so that each line corresponds to one input PBC operation.
Within a line is a sequence of operations, that come either as single or paired instructions.
Each instruction has an associated block and operation.
In particular, joint operations between blocks are paired as two instructions.


### Speeding up Clifford synthesis

The PBC Gross compiler requires a lookup table on how to synthesize Clifford rotations,
$e^{i \frac{\pi}{4} P}$ for $P$ a Pauli string on 12 qubits,
and the package `bicycle_cliffords` produces one.
Because this takes some time (about a minute on my laptop),
there is an option to save the lookup table on disk.
```
Usage: bicycle_compiler <CODE> generate <MEASUREMENT_TABLE>
```
We select `<CODE>` from `gross` or `two-gross` and `<MEASUREMENT_TABLE>` is an output file.
We can then use the measurement table to speed up compilation.
For example:

Build the table
```sh
> ./target/release/bicycle_compiler gross generate ./data/table_gross
```

Use the table
```sh
> cat ./crates/bicycle_compiler/example/simple.json |  jq --compact-output '.[]' | ./target/release/bicycle_compiler gross --measurement-table ./data/table_gross > bicycle_circuit.json
```
Once you have created a measurement table, it can be reused as many times as you want (it is read-only).
Note that changes to the contents of the table (i.e., in `bicycle_cliffords`) require regenerating the table.

# `bicycle_compiler`

This crate compiles circuits defined by sequences of Pauli-generated rotations, $\exp(i P \phi/2)$ for $n$-qubit Pauli string $P$,
and Pauli measurements (also known as _Pauli-based Compilation_ (PBC) circuits)
into bicycle circuits.
See Appendix A.9 of [Tour de Gross arXiv:2506.03094](https://arxiv.org/abs/2506.03094) for a review of how PBC circuits may be obtained from Clifford+$R_z$ circuits.
We represent PBC circuits in a line-based format:
```json
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","Y"],"angle":"0.125"}}
{"Rotation":{"basis":["Z","Z","I","I","I","I","I","I","I","I","I","I"],"angle":"0.5"}}
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","I"],"angle":"-0.125"}}
{"Measurement":{"basis":["Z","X","I","I","I","I","I","I","I","I","I","I"],"flip_result":true}}
{"Measurement":{"basis":["X","I","I","I","I","Z","I","I","I","I","I","I"],"flip_result":false}}
```
where each line is a JSON-object representing either a Pauli-generated rotation or a Pauli measurement.
All operations should act on the same number of logical qubits.
Rotations $\exp(i \phi P)$ are specified by objects with with a `Rotation` field, which has the `basis` field for the Pauli $P$ and the `angle` field for $\phi \in \mathbb R$.
Measurements are specified by objects with a `Measurement` field, which also has a `basis` field and whether the resulting measurement result should be flipped (currently not used).
The `flip_result` is intended to support a future implementation of 'measurement projections' as defined in equation (1) of [arXiv:2506.03094](https://arxiv.org/abs/2506.03094) in Section 3.

## Usage
Some example PBC circuits are provided in the `examples` directory.
Their JSON format is specified by `pbc_schema.json`.
These files are not in the line-based format required by the compiler;
to translate a JSON file into the line-based format, you can use `jq`
as in

```
cat example/simple.json | jq --compact-output '.[]' | cargo run --release -- gross
```

This may take a while because a Clifford synthesis table will be built in-memory.
See below on how to speed this up.

The output looks (with some newlines inserted for readability) like
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
Within a line is a sequence of bicycle instruction, that come either as single or paired instructions.
Each bicycle instruction has an associated code module and operation.
In particular, joint operations between blocks are paired as two instructions.

For a more advanced example on how the compiler can be used,
see how it is used as a library in the `bicycle_random_numerics` crate
or as a binary in [custom_circuits.ipynb](../../notebooks/custom_circuits.ipynb).

## Multi-block circuits

Each gross-code module encodes 11 data qubits (plus 1 pivot qubit used
internally by the compiler).  When a PBC circuit acts on more than 11
logical qubits the compiler automatically distributes it across multiple
modules:  the number of modules is $\lceil n/11 \rceil$, where $n$ is
the basis length.

For example, a basis of length 22 spans **2 modules** and a basis of
length 33 spans **3 modules**.  The compiler pads any basis that is not
a multiple of 11 with identity (`I`) entries.

Multi-block compilation introduces inter-module operations:

* **GHZ-state preparation** via `JointMeasure` instructions on adjacent
  modules.
* **T-gate injection** on the last module in the path.
* **GHZ uncomputation** via local measurements on each module.

### Examples

Single block (11 qubits):
```sh
cat example/one_block.json | jq --compact-output '.[]' | cargo run --release -- gross
```

Two blocks (22 qubits, both modules active):
```sh
cat example/two_blocks.json | jq --compact-output '.[]' | cargo run --release -- gross
```

Three blocks (33 qubits, all modules active):
```sh
cat example/three_blocks.json | jq --compact-output '.[]' | cargo run --release -- gross
```

In the output you will see instructions addressed to modules `0`, `1`,
`2`, etc., as well as paired `JointMeasure` instructions that entangle
adjacent modules.

## Caching up Clifford synthesis

The compiler uses a lookup table on how to synthesize Clifford rotations,
$e^{i \frac{\pi}{4} P}$ for $P$ a Pauli string on 12 qubits,
and uses the package `bicycle_cliffords` produces one.
Because this takes some time (about a minute on my laptop),
there is an option to cache the lookup table on disk:
```
bicycle_compiler <CODE> generate <MEASUREMENT_TABLE>
```
where `<CODE>` can be `gross` or `two-gross` and `<MEASUREMENT_TABLE>` is an output file name.
This will write a Clifford synthesis table in a binary serialized format to the file `MEASUREMENT_TABLE`.
We can then use the measurement table to reduce start-up costs of compilation.
For example:

Build the table
```sh
> bicycle_compiler gross generate table_gross
```

Use the table
```sh
> cat example/simple.json |  jq --compact-output '.[]' | bicycle_compiler gross --measurement-table table_gross
```
Once you have created a measurement table, it can be reused as many times as you want (it is read-only).
Note that changes to the contents of the table (i.e., in `bicycle_cliffords`) require manually regenerating the table.

# Pauli-based compilation for the Gross Code

This is a compiler targeting the Gross code architecture
for programs given in Pauli-based compilation (PBC) form.
It consist of a Rust library and a main binary.

## Installation

### Gridsynth
For synthesizing rotations by angles other than $\pm\pi/4$,
please ensure that a `python` executable is available in your path with the `pygridsynth~=1.1` package installed.
The following command should succeed
```
python -m pygridsynth 0.5 1e-3
```
and something like (the exact output may differ)
```
THTHTSHTHTHTHTHTSHTHTHTHTSHTHTSHTSHTSHTSHTSHTSHTSHTHTSHTHTSHTSHTHTSHTSHTHTSHSSWWWWWWW
```

This can be achieved by setting a local virtual environment as follows
```
pyenv virtualenv pbc-gross
pyenv local pbc-gross
pip install "pygridsynth~=1.1"
```

## Usage

### Input Program
The input program must be in a PBC form.
We choose the following format ([inspiration](https://doi.org/10.5281/zenodo.11391890))
```json
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","Y"],"angle":"0.125"}}
{"Rotation":{"basis":["Z","Z","I","I","I","I","I","I","I","I","I","I"],"angle":"0.5"}}
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","I"],"angle":"-0.125"}}
{"Measurement":{"basis":["Z","X","I","I","I","I","I","I","I","I","I","I"],"flip_result":true}}
{"Measurement":{"basis":["X","I","I","I","I","Z","I","I","I","I","I","I"],"flip_result":false}}

```
which shows the only two operations allowed in such a PBC program: Rotations and Measurements.
All operations should act on the same number of logical qubits.
Rotations are specified objections with a Rotation field and includes the basis and the rotation angle.
Measurements are specified by objects with a Measurement field, and includes the basis, then whether the resulting measurement result should be flipped (currently not used).

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
and the package `gross_code_cliffords` produces one.
Because this takes some time (about a minute on my laptop),
there is an option to save the lookup table on disk.
```
Usage: pbc_gross <CODE> generate <MEASUREMENT_TABLE>
```
We select `<CODE>` from `gross` or `two-gross` and `<MEASUREMENT_TABLE>` is an output file.
We can then use the measurement table to speed up compilation.
For example:
```
./target/release/pbc_gross gross generate table_gross
cat ./src/pbc_gross/example/simple.json |  jq --compact-output '.[]' | ./target/release/pbc_gross gross --measurement-table table_gross
```
Once you have created a measurement table, it can be reused as many times as you want (it is read-only).
Note that changes to the contents of the table (i.e., in `gross_code_cliffords`) require regenerating the table.

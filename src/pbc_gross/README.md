# Pauli-based compilation for the Gross Code

This is a compiler targeting the Gross code architecture
for programs given in Pauli-based compilation (PBC) form.
It consist of a Rust library and a main binary.

## Input Program
The input program must be in a PBC form.
We choose the following format ([inspiration](https://doi.org/10.5281/zenodo.11391890))
```json
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","Y"],"angle":0.125}}
{"Rotation":{"basis":["Z","Z","I","I","I","I","I","I","I","I","I","I"],"angle":0.5}}
{"Rotation":{"basis":["X","X","I","I","I","I","I","I","I","I","I","I"],"angle":-0.125}}
{"Measurement":{"basis":["Z","X","I","I","I","I","I","I","I","I","I","I"],"flip_result":true}}
{"Measurement":{"basis":["X","I","I","I","I","Z","I","A","I","I","I","I"],"flip_result":false}}

```
which shows the only two operations allowed in such a PBC program: Rotations and Measurements.
All operations should act on the same number of logical qubits.
Rotations are specified objections with a Rotation field and includes the basis and the rotation angle.
Measurements are specified by objects with a Measurement field, and includes the basis, then whether the resulting measurement result should be flipped (currently not used).

Note that the input program is a JSON "array" that can be delineated by whitespace (or newlines).
We can generate the above input from a JSON file by running, for example,
```
cat examples/simple.json | jq --compact-output '.[]'
```
which uses the `jq` program.

## Running the program

(TODO) You can pipe a program of the above format into the binary by running

```
cat example/simple.json | jq --compact-output '.[]' | cargo run --release
```

## Output

The output looks like
```json
[0,{"Measure":{"p1":"X","p7":"I"}}]
[1,{"Measure":{"p1":"Z","p7":"I"}}]
[0,{"JointMeasure":{"p1":"Z","p7":"I"}}]
[1,{"JointMeasure":{"p1":"Z","p7":"I"}}]
[0,{"Automorphism":{"x":3,"y":2}}]
[0,{"Measure":{"p1":"X","p7":"I"}}]
[0,{"Automorphism":{"x":3,"y":4}}]
[1,{"Rotation":{"basis":["X","Y","I","I","I","I","I","I","I","I","I"],"angle":0.125}}]
[0,{"Measure":{"p1":"Z","p7":"I"}}]
[1,{"Measure":{"p1":"Z","p7":"I"}}]
...
```
This output is similarly delineated by newlines.

## NOT NEEDED: Gridsynth installation
Please ensure that a `python` executable is available in your path with the `pygridsynth~=1.1` package installed.
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
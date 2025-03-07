# Pauli-based compilation for the Gross Code

## Installation
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
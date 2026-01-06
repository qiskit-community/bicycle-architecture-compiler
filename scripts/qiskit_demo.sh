#! /bin/sh

set -eo pipefail

# Change to this script's directory
cd "$(dirname "$0")" || exit

# path to the bicycle compiler executables
COMPILER_PATH=../target/release

if [ -z $1 ]; then
    N=100
    echo "Set default number of qubits to n=100."
else
    N=$1
fi

CODE="two-gross"
P="1e-4" 

# we'll force pre-generating the Clifford table, since that takes a long time
MEASUREMENT_TABLE="table_${CODE}.dat"
if [[ ! -f ${MEASUREMENT_TABLE} ]]; then
    echo "${MEASUREMENT_TABLE} not found, generating it..."
    ${COMPILER_PATH}/bicycle_compiler ${CODE} generate ${MEASUREMENT_TABLE} || {
        echo "Failed to generate measurement table."
        exit 1
    }
fi

# we yield the PBC instruction from the Python script and then consume them in the bicycle
# compiler to yield Gross code instruction, which are then consumed by the numerics to produce
# error estimates
python qiskit_demo.py $N \
    | ${COMPILER_PATH}/bicycle_compiler ${CODE} --measurement-table table_${CODE}.dat \
    | ${COMPILER_PATH}/bicycle_numerics $N ${CODE}_$P

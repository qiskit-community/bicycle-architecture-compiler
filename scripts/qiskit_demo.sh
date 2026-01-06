#!/bin/sh
# Copyright contributors to the Bicycle Architecture Compiler project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

set -eo pipefail

# Change to this script's directory.
cd "$(dirname "$0")" || exit

if [ -z $1 ]; then
    N=100
    echo "Set default number of qubits to n=100."
else
    N=$1
fi

COMPILER_PATH=../target/release # path to the bicycle compiler executables
INPUT_DATA_DIR="../data" # path to store measurement tables
CODE="two-gross" # type of code, can be "gross" or "two-gross"
P="1e-4" # physical error rate, can be "1e-3" or "1e-4"

# We'll force pre-generating the Clifford table for the specified code, since that takes a long 
# time. You can also generate the tables for both gross and two-gross code using 
# the generate_measurement_tables.sh script.
MEASUREMENT_TABLE="${INPUT_DATA_DIR}/table_${CODE}.dat"
if [[ ! -f ${MEASUREMENT_TABLE} ]]; then
    echo "${MEASUREMENT_TABLE} not found, generating it..."
    ${COMPILER_PATH}/bicycle_compiler ${CODE} generate ${MEASUREMENT_TABLE} || {
        echo "Failed to generate measurement table."
        exit 1
    }
fi

# We yield the PBC instruction from the Python script and then consume them in the bicycle
# compiler to yield Gross code instruction, which are then consumed by the numerics to produce
# error estimates.
python qiskit_demo.py $N \
    | ${COMPILER_PATH}/bicycle_compiler ${CODE} --measurement-table ${MEASUREMENT_TABLE} \
    | ${COMPILER_PATH}/bicycle_numerics $N ${CODE}_$P

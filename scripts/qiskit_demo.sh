#!/usr/bin/env bash
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

set -euo pipefail

# Change to this script's directory.
cd "$(dirname "$0")" || exit
N="${1:-10}"
>&2 echo "Using n=${N} x ${N} qubits"

COMPILER_PATH=../target/release # path to the bicycle compiler executables
INPUT_DATA_DIR="../data" # path to store measurement tables
mkdir -p "$INPUT_DATA_DIR"
CODE="two-gross" # type of code, can be "gross" or "two-gross"
P="1e-4" # physical error rate, can be "1e-3" or "1e-4"

# Ensure the Clifford tables are generated, since that takes a long time.
./generate_measurement_tables.sh
MEASUREMENT_TABLE="${INPUT_DATA_DIR}/table_${CODE}" # Location of measurement table

# We yield the PBC instruction from the Python script and then consume them in the bicycle
# compiler to yield Gross code instruction, which are then consumed by the numerics to produce
# error estimates.
python qiskit_demo.py $N \
    | ${COMPILER_PATH}/bicycle_compiler ${CODE} --measurement-table ${MEASUREMENT_TABLE} \
    | ${COMPILER_PATH}/bicycle_numerics $N ${CODE}_$P

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

set -euo pipefail

##
## Run the executable `random_numerics` several times, in parallel,
## for various input parameter values. This script depends on
## finding the GNU parallel program in your path.
##
## This script should be run from generate_tables_and_random_numerics.sh
## It should not be run directly.
##
## This script is a simpler alternative to run_random_numerics.py
## It does the same thing that run_random_numerics.py does
## 

# Change to this script's directory
cd "$(dirname "$0")" || exit

# This data was computed and written by the executable ./target/release/pbc_gross
input_data_dir="../data"

parallel --no-notice --colsep "," \
         "../target/release/random_numerics --model {1} --noise {2} --qubits {3} --measurement-table $input_data_dir/table_{1} > ../tmp/out_{1}_{2}_{3}_{4}.csv" \
         :::: parameters.csv \
         ::: $(seq 1 8)

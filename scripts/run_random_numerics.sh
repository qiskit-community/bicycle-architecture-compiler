#!/bin/sh
set -euo pipefail

##
## This script is only necessary in case run_random_numerics.py fails, for some
## reason. We do not expect that it will fail.
## This script is a simpler alternative to run_random_numerics.py
## It does the same thing that run_random_numerics.py does
##
## This script should be run from generate_data.sh.
## It should not be run directly.
##
## This script requires the utility GNU parallel

# Change to this script's directory
cd "$(dirname "$0")" || exit

input_data_dir="../data"

parallel --no-notice --colsep "," \
         "../target/release/random_numerics --model {1} --noise {2} --qubits {3} --measurement-table $input_data_dir/table_{1} > ../tmp/out_{1}_{2}_{3}_{4}.csv" \
         :::: parameters.csv \
         ::: $(seq 1 8)

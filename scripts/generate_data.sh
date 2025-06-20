#!/bin/bash
set -euo pipefail

# Change to this script's directory
cd "$(dirname "$0")" || exit

# Build binaries. Only prints output on failure
if ! cargo build --release > /dev/null 2>&1; then
    echo "Error: `cargo build` failed."
    exit 1
fi

input_data_dir="../data"

tables_exist=true
if [ ! -e "$input_data_dir/table_gross" ]; then
    echo "Error: $input_data_dir/table_gross does not exist."
    tables_exist=false
fi
if [ ! -e "$input_data_dir/table_two-gross" ]; then
    echo "Error: $input_data_dir/table-two_gross does not exist."
    tables_exist=false
fi

# Cache measurement tables
if ! $tables_exist; then
    echo "Run ./target/release/pbc_gross gross generate data/table_gross"
    echo "    ./target/release/pbc_gross two-gross generate data/table_two-gross"
    exit 1
fi

# Read parameters from parameters.csv and run each parameter 8 times
echo "Running random_numerics"

# This python script should work correctly. If it does not, try the shell-script in the
# next line.
./run_random_numerics.py

# Alternative shell implementation of run_random_numerics.py
# If you use this script, you must first install the utility GNU paralell.
# ./run_random_numerics.sh

echo "Data generation complete. Concatenating output to '$input_data_dir/data.csv'."
mkdir -p ../tmp/
awk '(NR == 1) || (FNR > 1)' ../tmp/out_*.csv > "$input_data_dir/data.csv"
rm ../tmp/out_*.csv

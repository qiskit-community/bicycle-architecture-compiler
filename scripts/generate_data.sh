#!/bin/sh
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

if command -v parallel >/dev/null 2>&1; then
    echo "Using GNU parallel."
    ./run_random_numerics.sh
elif command -v python3 >/dev/null 2>&1 && python3 -c "import sys; sys.exit(0)" >/dev/null 2>&1; then
    echo "Python3 is available and functional. Using python3"
    ./run_random_numerics.py
else
    echo "You must install either GNU parallel or a functional Python 3." >&2
    exit 1
fi

echo "Data generation complete. Concatenating output to '$input_data_dir/data.csv'."
mkdir -p ../tmp/
awk '(NR == 1) || (FNR > 1)' ../tmp/out_*.csv > "$input_data_dir/data.csv"
rm ../tmp/out_*.csv

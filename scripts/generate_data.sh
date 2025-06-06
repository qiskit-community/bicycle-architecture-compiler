#!/bin/bash
set -euo pipefail

# Change to this script's directory
cd "$(dirname "$0")" || exit

# Build binaries. Only prints output on failure
if ! cargo build --release > /dev/null 2>&1; then
    echo "Error: `cargo build` failed."
    exit 1
fi

datadir="../data"

tables_exist=true
if [ ! -e "$datadir/table_gross" ]; then
    echo "Error: $datadir/table_gross does not exist."
    tables_exist=false
fi
if [ ! -e "$datadir/table_two-gross" ]; then
    echo "Error: $datadir/table-two_gross does not exist."
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
parallel --no-notice --colsep "," \
         "../target/release/random_numerics --model {1} --noise {2} --qubits {3} --measurement-table $datadir/table_{1} > ../tmp/out_{1}_{2}_{3}_{4}.csv" \
         :::: parameters.csv \
         ::: $(seq 1 8)

echo "Data generation complete. Concatenating output to '$datadir/data.csv'."
mkdir -p ../tmp/
awk '(NR == 1) || (FNR > 1)' ../tmp/out_*.csv > "$datadir/data.csv"
rm ../tmp/out_*.csv

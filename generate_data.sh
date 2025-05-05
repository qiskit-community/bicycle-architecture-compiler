#!/bin/bash
set -euo pipefail

# Build binaries
cargo build --release

# Cache measurement tables
# ./target/release/pbc_gross gross generate table_gross
# ./target/release/pbc_gross two-gross generate table_two-gross

# Read parameters from parameters.csv and run each parameter 10 times
parallel --colsep "," "./target/release/random_numerics --model {1} --noise {2} --qubits {3} --measurement-table table_{1} > out_{1}_{2}_{3}_{4}.csv" \
     :::: parameters.csv \
     ::: $(seq 1 10)

echo "Data generation complete. Concatenating output."
awk '(NR == 1) || (FNR > 1)' out_*.csv > data.csv
rm out_*.csv
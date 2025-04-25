#!/bin/bash
set -euo pipefail

models=("gross 1e-3 121" "gross 1e-3 1353" "gross 1e-3 13728" \
    "gross 1e-4 110" "gross 1e-4 1342" "gross 1e-4 13717" \
    "two-gross 1e-3 55" "two-gross 1e-3 704" "two-gross 1e-3 7150" \
    "two-gross 1e-4 440" "two-gross 1e-4 6886"    )

for i in "${models[@]}"
do
    set -- $i
    cargo run --release --package benchmark $3 | \
        cargo run --release --package pbc_gross $1 | \
        cargo run --release --package numerics $3 "$1_$2" \
        > "out_$1_$2_$3.csv" &
done
wait
echo "Data generation complete. Concatenating output."

awk '(NR == 1) || (FNR > 1)' out_*.csv > data.csv
# rm out_*.csv
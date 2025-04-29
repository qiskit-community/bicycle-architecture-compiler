#!/bin/bash
set -euo pipefail

models=("gross 1e-3 121" "gross 1e-3 1353" \
# "gross 1e-3 13728" \
    "gross 1e-4 110" "gross 1e-4 1342" "gross 1e-4 13717" \
    "two-gross 1e-3 55" "two-gross 1e-3 704" \
# "two-gross 1e-3 7150" \
    "two-gross 1e-4 440" "two-gross 1e-4 6886"    )

for i in "${models[@]}"
do
    set -- $i
    cargo run --release --package random_numerics -- --model $1 --noise $2 --qubits $3 \
        > "out_$1_$2_$3.csv" &
done
wait
echo "Data generation complete. Concatenating output."

awk '(NR == 1) || (FNR > 1)' out_*.csv > data.csv
# rm out_*.csv
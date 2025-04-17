#!/bin/bash
set -euo pipefail

models=("gross 1e3 275" "gross 1e3 2750" \
     "gross 1e4 2750" "gross 1e4 27500" \
     "two-gross 1e3 143" "two-gross 1e3 1430"\
     "two-gross 1e4 1430" "two-gross 1e4 14322"\
    )

for i in "${models[@]}"
do
    set -- $i
    cargo run --release $3 "$1$2" | tail -n +2 | while read line; do echo "$1,$2,$line"; done > "out_$1_$2_$3.csv" &
done
wait
echo "Data generation complete. Concatenating output."

echo "code,p,i,qubits,blocks,rotations,automorphisms,measurements,joint measurements,cumulative measurement depth,syndrome time,error rate" > "data.csv"
cat out_*.csv >> "data.csv"
rm out_*.csv
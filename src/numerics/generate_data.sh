#!/bin/bash
set -euo pipefail

for i in "gross 1e3 275" "gross 1e3 2750" "gross 1e4 2750" "gross 1e4 27500" \
    "two-gross 1e3 143" "two-gross 1e3 1430" "two-gross 1e4 1430" "two-gross 1e4 14322"
do
    set -- $i
    cargo run --release $3 "$1$2" | tail -n +2 | while read line; do echo "$1,$2,$line"; done > "$1_$2_$3.csv" &
done
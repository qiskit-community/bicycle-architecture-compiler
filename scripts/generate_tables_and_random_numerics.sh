#!/bin/sh
set -euo pipefail

# Change to this script's directory
cd "$(dirname "$0")" || exit

# Generate measurement tables (if they are not present) and run random numerics
# with parameters specified in parameters.csv

# Ensure that the  measurement tables have been generated.
./generate_measurement_tables.sh

# Read parameters from parameters.csv and run each parameter 8 times
echo "Running random_numerics"

# Ensure that the temporary directory exists
# Data genererated by several `random_numerics` processes will be written to this
# temp directory.
# The data will be collated and written to `bicycle-architecture-compiler/data/random_numerics_output.csv`
# Finally, the temporary files will be deleted.

mkdir -p ../tmp/

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

input_data_dir="../data"
echo "Data generation complete. Concatenating output to '$input_data_dir/random_numerics_output.csv'."
awk '(NR == 1) || (FNR > 1)' ../tmp/out_*.csv > "$input_data_dir/random_numerics_output.csv"
rm ../tmp/out_*.csv

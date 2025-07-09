#!/bin/sh
set -euo pipefail

# Ensure that measurement tables have been generated.
#
# This script checks that measurement tables are present in
# `bicycle-architecture-compiler/data/table_gross` and
# `bicycle-architecture-compiler/data/table_gross`
#
# If they are not present, this script runs `bicycle_compiler` to generate them.

# Change to this script's directory
cd "$(dirname "$0")" || exit

# Build binaries. Only prints output on failure
if ! cargo build --release > /dev/null 2>&1; then
    echo "Error: `cargo build` failed."
    exit 1
fi

input_data_dir="../data"

gross_table_path="$input_data_dir/table_gross"
twogross_table_path="$input_data_dir/table_two-gross"

gross_table_exists=true
twogross_table_exists=true

if [ -e "$gross_table_path" ]; then
    # Check if the file size is zero
    if [ ! -s "$gross_table_path" ]; then
        echo "File $gross_table_path exists but is of size zero. Deleting..."
        rm "$gross_table_path"
    fi
fi

if [ -e "$twogross_table_path" ]; then
    if [ ! -s "$twogross_table_path" ]; then
        echo "File $twogross_table_path exists but is of size zero. Deleting..."
        rm "$twogross_table_path"
    fi
fi

if [ ! -e "$gross_table_path" ]; then
    gross_table_exists=false
fi
if [ ! -e "$twogross_table_path" ]; then
    twogross_table_exists=false
fi

# Kill all subprocesses when receiving SIGINT or SIGTERM
# This kills the process group. But first clears the handler.
# We install this trap before running commands in the background
trap "trap - SIGTERM && kill -- -$$" SIGINT SIGTERM

pids=""

pbc_program="bicycle_compiler"
pbc_gross_com="../target/release/$pbc_program gross generate"
pbc_two_gross_com="../target/release/$pbc_program two-gross generate"

# Cache measurement tables
if [ "$gross_table_exists" = "false" ] ; then
    echo "Measurement table 'table_gross' not found. Generating measurement table."
    echo $pbc_gross_com $gross_table_path
    $pbc_gross_com $gross_table_path &
    pids="$pids $!"
fi

if [ "$twogross_table_exists" = "false" ] ; then
    echo "Measurement table 'table_two-gross' not found. Generating measurement table."
    echo $pbc_two_gross_com $twogross_table_path
    $pbc_two_gross_com $twogross_table_path &
    pids="$pids $!"
fi

if  ! [ "$pids" = "" ]; then
    echo Waiting on pids "$pids"
fi

for pid in $pids; do
    echo "Waiting for $pbc_program process $pid"
    if wait $pid; then
        echo "$pbc_program process $pid completed successfully."
    else
        status=$?
        echo "$pbc_program process $pid was terminated or failed. Status $status"
        # We choose to kill any remaining programs and exit.
        for pid in $pids; do
            if kill -0 $pid 2>/dev/null; then
                echo "Killing remaining $pbc_program process $pid"
                kill $pid
            fi
        done
        echo "$pbc_program processes terminated or failed. Exiting"
        exit 1
    fi
done

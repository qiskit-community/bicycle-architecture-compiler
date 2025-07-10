#!/usr/bin/python3

import asyncio
import subprocess
from pathlib import Path
import os
import sys
import argparse

##
## Note: This file can be used as module. Eg.
## from generate_random_numerics import generate_random_numerics
##
## However, this does not work easily with Jupyter because it has a competing event loop.
## In this case, this file may be called as a program, albeit with less controll and feedback.
## See the code at the bottom of this file.

##
## Run the executable `random_numerics` several times, in parallel,
## for various input parameter values.
##
## This script should be run from generate_tables_and_random_numerics.sh
## It should not be run directly.
##
## This script does the same thing that run_random_numerics.sh does.
## But run_random_numerics.sh depends on finding the GNU parallel program in your path.
##

## Change directory to this files's directory
# try:
#     os.chdir(os.path.dirname(os.path.abspath(__file__)))
# except OSError as e:
#     print(f"Failed to change directory: {e}")

THIS_DIR = os.path.dirname(os.path.abspath(__file__))

# Top level directory of bicycle-architecture-compiler
TOP_DIR = Path(THIS_DIR, '..')

# `random_numerics` output will be written to files in this directory
# This script will not remove files from this directory
TMP_DIR = Path(TOP_DIR, 'tmp')

# `random_numerics` will be run for several different sets of parameters
PARAMETER_INPUT_PATHNAME = Path(TOP_DIR, 'scripts', 'parameters.csv')

# For each set of parameters, `random_numerics` will be run this many times
# with different random seeds
NUM_RANDOMIZATIONS = 8

# Directroy of measurement-table data,  used as input to `random_numerics`.
INPUT_DATA_DIR = Path(TOP_DIR, 'data')

COLLATED_PATHNAME = f"{INPUT_DATA_DIR}/random_numerics_output.csv"

# Path to the rust binary executable
EXECUTABLE_PATH = Path(TOP_DIR, 'target', 'release', 'bicycle_random_numerics')

def ensure_directory_exists(directory_path):
    if not os.path.exists(directory_path):
        os.makedirs(directory_path)

# Run the exectuable once for one set of input parameters
async def run_command(cmd, output_file, verbose=True):
    with open(output_file, 'wb') as f:
        process = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=f,
            stderr=asyncio.subprocess.PIPE
        )
        # Wait for the process to complete
        _, stderr = await process.communicate()

        if stderr:
            print(f"[stderr]\n{stderr.decode()}")

    if verbose:
        print(f"Run {' '.join([str(x) for x in cmd[1:7]])} exited with {process.returncode}")
        sys.stdout.flush()

# Read the input parameter file
def read_parameters(pathname):
    param_list = []
    with open(pathname, 'r') as f:
        for line in f:
            (model, noise, qubits) = line.strip().split(',')
            param_list.append([model, noise, qubits])
    return param_list

def _output_pathname(model, noise, qubits, trial_num):
    output_filename = f"out_{model}_{noise}_{qubits}_{trial_num}.csv"
    output_path = Path(TMP_DIR) / output_filename
    return output_path

async def run_all_trials(verbose=True):
    param_list = read_parameters(PARAMETER_INPUT_PATHNAME)

    task_data = []
    for trial_num in range(NUM_RANDOMIZATIONS):
        for (model, noise, qubits) in param_list:
            output_path = _output_pathname(model, noise, qubits, trial_num)
            cmd = [
                EXECUTABLE_PATH,
                '--model', model,
                '--noise', noise,
                '--qubits', qubits,
                '--measurement-table', f"{INPUT_DATA_DIR}/table_{model}"
            ]
            task_data.append([cmd, output_path])

    # Run all tasks and collect ids in a list
    tasks = [run_command(cmd, output_path, verbose) for (cmd, output_path) in task_data]

    # Wait for all tasks to complete before returning
    await asyncio.gather(*tasks)


def generate_random_numerics(overwrite=True, verbose=True):
    if overwrite or not os.path.exists(COLLATED_PATHNAME):
        ensure_directory_exists(TMP_DIR)
        asyncio.run(run_all_trials(verbose=verbose))

def collate_random_numerics(overwrite=True):
    if overwrite or not os.path.exists(COLLATED_PATHNAME):
        command = (f"awk '(NR == 1) || (FNR > 1)' {TMP_DIR}/out_*.csv")
        output_file = f"{INPUT_DATA_DIR}/random_numerics_output.csv"

        with open(output_file, "w") as outfile:
            subprocess.run(command, stdout=outfile, shell=True, check=True)

def generate_and_collate_random_numerics(overwrite=True, verbose=False):
    if verbose:
        if not os.path.exists(COLLATED_PATHNAME):
            print("./data/random_numerics_output.csv not found. Generating data.")
        elif overwrite:
            print("./data/random_numerics_output.csv exists, but overwrite is True. Generating data.")
        else:
            print("./data/random_numerics_output.csv exists. Will not overwrite")
    sys.stdout.flush()
    generate_random_numerics(overwrite, verbose=verbose)
    collate_random_numerics(overwrite)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run random_numerics many times.")
    parser.add_argument(
        '--overwrite',
        action='store_true',
        help="Pass this flag to overwrite existing files.",
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help="Pass this flag to print a small amount of diagnostic information.",
    )
    args = parser.parse_args()
    generate_and_collate_random_numerics(overwrite=args.overwrite, verbose=args.verbose)

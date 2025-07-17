#!/usr/bin/env python3
# Copyright contributors to the Bicycle Architecture Compiler project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import asyncio
from pathlib import Path
import os

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

## Change directory to the script's directory
try:
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
except OSError as e:
    print(f"Failed to change directory: {e}")

# Top level of pbc-compiler
TOP_DIR = ".."

# `random_numerics` output will be written to files in this directory
# This script will not remove files from this directory
TMP_DIR = Path(TOP_DIR, "tmp")

# `random_numerics` will be run for several different sets of parameters
PARAMETER_INPUT_PATHNAME = Path("parameters.csv")

# For each set of parameters, `random_numerics` will be run this many times
# with different random seeds
NUM_RANDOMIZATIONS = 8

# Directroy of measurement-table data,  used as input to `random_numerics`.
INPUT_DATA_DIR = Path(TOP_DIR, "data")

# Path to the rust binary executable
EXECUTABLE_PATH = Path(TOP_DIR, "target", "release", "random_numerics")


# Run the exectuable once for one set of input parameters
async def run_command(cmd, output_file):
    with open(output_file, "wb") as f:
        process = await asyncio.create_subprocess_exec(
            *cmd, stdout=f, stderr=asyncio.subprocess.PIPE
        )
        # Wait for the process to complete
        _, stderr = await process.communicate()

        if stderr:
            print(f"[stderr]\n{stderr.decode()}")

    print(f"{' '.join([str(x) for x in cmd])} exited with {process.returncode}")


# Read the input parameter file
def read_parameters(pathname):
    param_list = []
    with open(pathname, "r") as f:
        for line in f:
            (model, noise, qubits) = line.strip().split(",")
            param_list.append([model, noise, qubits])
    return param_list


def _output_pathname(model, noise, qubits, trial_num):
    output_filename = f"out_{model}_{noise}_{qubits}_{trial_num}.csv"
    output_path = Path(TMP_DIR) / output_filename
    return output_path


async def main():
    param_list = read_parameters(PARAMETER_INPUT_PATHNAME)

    task_data = []
    for trial_num in range(NUM_RANDOMIZATIONS):
        for model, noise, qubits in param_list:
            output_path = _output_pathname(model, noise, qubits, trial_num)
            cmd = [
                EXECUTABLE_PATH,
                "--model",
                model,
                "--noise",
                noise,
                "--qubits",
                qubits,
                "--measurement-table",
                f"{INPUT_DATA_DIR}/table_{model}",
            ]
            task_data.append([cmd, output_path])

    # Run all tasks and collect ids in a list
    tasks = [run_command(cmd, output_path) for (cmd, output_path) in task_data]

    # Wait for all tasks to complete before returning
    await asyncio.gather(*tasks)


asyncio.run(main())

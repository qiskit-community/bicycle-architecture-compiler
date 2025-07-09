from .read_numerics import read_numerics

from .random_numerics import (
    process_random_numerics,
    partition_data,
    group_partitioned,
    compute_means,
    plot_means,
    read_and_plot,
    )

import subprocess
import os

THIS_DIR = os.path.dirname(os.path.abspath(__file__))

PROGRAM_PATH = f"{THIS_DIR}/generate_random_numerics.py"

def run_random_numerics(overwrite=True, verbose=True):
    command = [PROGRAM_PATH]
    if overwrite:
        command.append("--overwrite")
    if verbose:
        command.append("--verbose")

    subprocess.run(command, stdout=None, stderr=None, check=True)

# from .generate_random_numerics import generate_random_numerics, collate_random_numerics

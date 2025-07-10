import numpy as np
import matplotlib.pyplot as plt

# Map estimated number of physical qubits to number of logical qubits.
#
# Tuples are (code, p, q)
# code - model name, either "gross" or "two-gross"
# p - physical error rate, either 1e-3 or 1e-4
# q - estimated number of physical qubits, counted in thousands.
#
# See "Tour de gross" Figure 10a
#
_PHYSICAL_TO_LOGICAL = {
    ("gross", 1e-3, 5) : 121,
    ("two-gross", 1e-3, 50) : 704,
    ("gross", 1e-4, 5) : 110,
    ("gross", 1e-4, 50) : 1342,
    ("two-gross", 1e-4, 50) : 440,
    ("two-gross", 1e-4, 500) : 6886,
}

# Tuples of parameters that define a data set.
# Each set contains eight random trials.
DATA_SETS = list(_PHYSICAL_TO_LOGICAL.keys())

# Write data set parameters into a string for a plot legend.
def format_set_tuple(set_tuple):
    (code, p, q) = set_tuple
    if p == 1e-3:
        ps = "$10^{-3}$"
    else:
        ps = "$10^{-4}$"
    return f"{ps}, {q}K, {code}"


# Filter data on a particular set of input parameters.
# Return an np.array of dtype `bool` that will be used to index into arrays.
# The returned array has `True` only for indices satisfying the criteria.
#
# Input:
# data - dict containing results of random numerics
# code - name of code, or model. Either 'gross' or 'two-gross'
# p - physical error rate
# q - number of physical qubits (q x 1000 is actual number)
# def find_numerics_by_params(data, code, p, q):
#     n_logical = _PHYSICAL_TO_LOGICAL[(code, p, q)]
#     idxs = (data["code"] == code) * (data["p"] == p) * (data["qubits"] == n_logical)
#     return idxs

# Input the big data structure and a tuple of random_circuit input parameters.
# Call `find_numerics_by_params` with these parameters.
# `DATA_SETS` lists all possible values of `set_tuple` for which we have
# generated data.
def find_numerics_set(data, set_tuple):
    # Look up the number of logical qubits
    n_logical = _PHYSICAL_TO_LOGICAL[set_tuple]
    (code, p, q) = set_tuple
    # Find the intersection of indices satisfying each conditin
    idxs = (data["code"] == code) * (data["p"] == p) * (data["qubits"] == n_logical)
    return idxs

# Partition the input data by data set parameters
# That is, partition a single data structure into data sets.
def partition_data(data):
    nlines = len(data["code"])
    partitioned = {}
    total_lines = 0
    for set_tuple in DATA_SETS:
        idx = find_numerics_set(data, set_tuple)
        num_lines = sum(idx)
        total_lines = total_lines + num_lines
        part_data = {}
        for (column_name, col_data) in data.items():
            part_data[column_name] = col_data[idx]
            assert len(part_data[column_name]) == num_lines
        partitioned[set_tuple]  = part_data
        (code, p, q) = set_tuple
        print(f"{num_lines} lines in data set (code={code}, p={p}, q={q})")

    # Make sure that every input line is accounted for.
    assert nlines == total_lines
    return partitioned

# Convert data for each trial from 1-d arrays of length m x n, to 2-d arrays
# of shape (m, n), where m is the number of trials, and n is the number of points
# (input instructions) in each trial.
# We don't want and can't have ragged 2-d arrays, wo we truncate to the smallest
# n over the trials.
#
# `pdata` - All the input data, partitioned into data sets. In each column,
#           the data for each trial is arranged sequentially.
def group_trials(pdata):
    starts = trial_indices(pdata['i']) # starting idx of each trial
    min_length = np.min(np.diff(starts)) # length of shortest trial
    out_data = {}
    for (column_name, col_data) in pdata.items():
        # Collect data from this column for each trial in a list
        trial_data = [col_data[i:i+min_length] for i in starts]
        # Create a 2-d array from this list, and store it
        out_data[column_name] = np.vstack(trial_data)

    return out_data

# Find the index of the start of each trial
# `trial_nums` - column labeled "i" in input file
# This has the form 1,2,...,n_1,1,2...,n_2,...
# where n_1, n_2, are indices of last point written
# in a trial.
# Recall that n_1, n_2, etc. may differ because the stopping
# criteria are not deterministic.
def trial_indices(trial_nums: np.array):
    # Find indices of end of trials (except the last)
    ends = np.where(np.diff(trial_nums) < 0)[0]
    # Find indices of starts of trials
    starts = (ends + 1)
    # Insert 0 for the start of the first trial
    starts = np.insert(starts, 0, 0)
    return starts

def group_partitioned(partitioned: dict):
    """
    Reshapes 1-d arrays of `m x n` elements in `partitioned` to 2-d arrays of shape `(m, n)`.

    `partitioned` is random circuit output data partitioned by data set. The keys to `partitioned` are
    tuples of parameters specifying the data set.

    In the input arrays, the data for distinct trials is organized sequentially. In the output
    arrays, the first index specifies the trial number, and the second, the data point within
    the trial.
    """
    grouped_partitioned = {}
    i = 0
    for (set_tuple, part_data) in partitioned.items():
        n = len(part_data["code"])
        grouped_part_data = group_trials(part_data)
        grouped_partitioned[set_tuple] = grouped_part_data
    return grouped_partitioned

def process_random_numerics(data: dict):
    """
    Partition columns holding data from all data sets into one column for each data set.

    The keys of the output dict are tuples specifying input parameters for the data set.
    Each of the values of the output dict is a dict with the same structure as the input dict `data`.
    Each of these values contains a part of the data contained in `data`.
    """
    partitioned_data = partition_data(data)
    grouped_data = group_partitioned(partitioned_data)
    compute_means(grouped_data)
    return grouped_data

# Compute the mean of the column labeled "total_error" and write
# the result to the input data in a column (dict entry) "mean_error"
def compute_means(grouped_data: dict):
    """
    Compute the mean of the entry 'total_error' in each data set contained in `grouped_data`.

    The keys of `grouped_data` are tuples of parameters characterizing a data set. The
    values are `dict`s containing keys matching labels on columns in output of `random_numerics`.
    For each data set, an additional key, value pair is entered. The key is "mean_error" and
    the value is the average of "total_error" across trials within one data set.
    """
    for (set_tuple, data) in grouped_data.items():
        mean_error = np.mean(data["total_error"], axis=0)
        data["mean_error"] = mean_error

def plot_means(grouped_data: dict):
    """
    Plot a curve for the column "mean_error" for each data set in `grouped_data`.
    """
    for (set_tuple, data) in grouped_data.items():
        mean_error = data["mean_error"]
        n = len(mean_error)
        plt.plot(np.arange(1, n + 1),  mean_error, label=format_set_tuple(set_tuple))
    plt.title('Random circuits')
    plt.xscale('log')
    plt.yscale('log')
    plt.xlabel('# Logical T gates')
    plt.ylabel("Circuit failure probability")
    plt.legend(loc='upper left', bbox_to_anchor=(1.05, 1), borderaxespad=0.)
    plt.grid(which='both', linestyle='--', linewidth=0.7)
    plt.minorticks_off()
    plt.show(block=False)

from .read_numerics import read_numerics

def read_and_plot(filepath: str):
    """
    Reads data produced by `random_numerics` from a file, processes it, computes means, and plots the results.

    Parameters
    ----------
    filepath : str
        The path to the file containing the collated output of `random_numerics` processes

    Returns
    -------
    tuple
        A tuple containing the raw data and the processed grouped data.

    Examples
    --------
    >>> data, grouped_data = read_and_plot("./data/random_numerics_output.csv")
    """
    data = read_numerics(filepath)
    grouped_data = process_random_numerics(data)
    compute_means(grouped_data)
    plot_means(grouped_data)
    return data, grouped_data


import subprocess
import os

THIS_DIR = os.path.dirname(os.path.abspath(__file__))
PROGRAM_PATH = f"{THIS_DIR}/generate_random_numerics.py"

def run_random_numerics(overwrite=True, verbose=True):
    """
    Run `random_numerics` several times each of various parameter sets and collate the results.
    """
    command = [PROGRAM_PATH]
    if overwrite:
        command.append("--overwrite")
    if verbose:
        command.append("--verbose")
    subprocess.run(command, stdout=None, stderr=None, check=True)

# Test that asking for number of physical qubits reliably gets entries with the
# correct number of logical qubits (and other parameters)
def test_num_logical(data):
    qbits = numerics["qubits"]
    result = (
        all(qbits[find_numerics_set(data, ("gross", 1e-3, 5))] == 121) and
        all(qbits[find_numerics_set(data, ("two-gross", 1e-3, 50))] == 704) and
        all(qbits[find_numerics_set(data, ("gross", 1e-4, 5))] == 110) and
        all(qbits[find_numerics_set(data, ("two-gross", 1e-4, 50))] == 440) and
        all(qbits[find_numerics_set(data, ("two-gross", 1e-4, 500))] == 6886)
    )
    return result

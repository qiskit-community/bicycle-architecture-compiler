import numpy as np
import matplotlib.pyplot as plt

# Example use:
# # Read data from csv file and partition by data set.
# In [1]: %time data = read_numerics("./data/random_numerics_output.csv") # takes 20s
#
# In [2]: partdata = partition_data(data)
# 104 lines in data set (code=gross, p=0.001, q=5)
# 800000 lines in data set (code=two-gross, p=0.001, q=50)
# 800000 lines in data set (code=gross, p=0.0001, q=5)
# 296390 lines in data set (code=gross, p=0.0001, q=50)
# 800000 lines in data set (code=two-gross, p=0.0001, q=50)
# 800000 lines in data set (code=two-gross, p=0.0001, q=500)

# Error rates
p0 = 1e-3
p1 = 1e-4

# Number of trials run for each parameter set
NUM_TRIALS = 8

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
    ("gross", p0, 5) : 121,
    ("two-gross", p0, 50) : 704,
    ("gross", p1, 5) : 110,
    ("gross", p1, 50) : 1342,
    ("two-gross", p1, 50) : 440,
    ("two-gross", p1, 500) : 6886,
}

# Tuples of parameters that define a data set.
# Eight random trials were done for each tuple of parameters
DATA_SETS = list(_PHYSICAL_TO_LOGICAL.keys())

# Write data set parameters into a string for
# a plot legend.
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
def find_numerics_by_params(data, code, p, q):
    n_logical = _PHYSICAL_TO_LOGICAL[(code, p, q)]
    idxs = (data["code"] == code) * (data["p"] == p) * (data["qubits"] == n_logical)
    return idxs

# Input the big data structure and a tuple of random_circuit input parameters.
# Call `find_numerics_by_params` with these parameters.
# `DATA_SETS` lists all possible values of `set_tuple` for which we have
# generated data.
def find_numerics_set(data, set_tuple):
    (code, p, q) = set_tuple
    return find_numerics_by_params(data, code, p, q)

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

def group_trials(pdata):
    nlines = len(pdata["code"])
    n_tgates = nlines // NUM_TRIALS
    out_data = {}
    for (column_name, col_data) in pdata.items():
        out_data[column_name] = col_data.reshape(NUM_TRIALS, n_tgates)

    return out_data

def group_trials_hack(pdata):
    inds = pdata["i"]
    # Two data sets are shorter by one.
    # Check that this gets all 8 sets.
    ngood = max(inds)-1
    assert sum(inds == ngood) == 8
    # Get the indices of the last instr. number common to all sets
    good_last_ind = np.where(inds == ngood)[0]
    out_data = {}
    for (column_name, col_data) in pdata.items():
        trials = [col_data[i-ngood+1:i+1] for i in good_last_ind]
        out_data[column_name] = np.array(trials)
    return out_data

def group_partitioned(partitioned):
    grouped_partitioned = {}
    i = 0
    for (set_tuple, part_data) in partitioned.items():
        n = len(part_data["code"])
        if n % 8 != 0:
            grouped_part_data = group_trials_hack(part_data)
        else:
            grouped_part_data = group_trials(part_data)
        grouped_partitioned[set_tuple] = grouped_part_data
    return grouped_partitioned

def compute_means(pdata):
    for (set_tuple, data) in pdata.items():
        mean_error = np.mean(data["total_error"], axis=0)
        data["mean_error"] = mean_error

def plot_mean(pdata):
    for (set_tuple, data) in pdata.items():
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
    plt.show()

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

import numpy as np

# Read a csv file assuming the first line labels the columns
# Return a dict whose keys are the column labels and
# whose values are the columns as numpy arrays.
# This function will read the output of bicycle_numerics::run_numerics
#
def read_numerics(filename):
    rows = _raw_read_numerics(filename)
    return {name: rows[name] for name in rows.dtype.names}

def _raw_read_numerics(filename):
    return np.genfromtxt(filename, delimiter=',', names=True, dtype=None, encoding=None);

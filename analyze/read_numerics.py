import numpy as np

# Read a csv file assuming the first line labels the columns
# Return a dict whose keys are the column labels and
# whose values are the columns as numpy arrays.
# This function will read the output of bicycle_numerics::run_numerics
#
def read_numerics(filepath):
    """
    Read csv file `filepath` and return a `dict` whose keys are the labels of columns
    in the csv file, and whose values are numpy arrays containing the columns.

    It is assumed that the first line of `filepath` labels the columns
    """
    rows = np.genfromtxt(filepath, delimiter=',', names=True, dtype=None, encoding=None)
    return {name: rows[name] for name in rows.dtype.names}

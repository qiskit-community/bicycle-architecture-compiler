# Scripts Usage

## Top-level scripts

These scripts are intended to be run directly by the user.

- [`local_QA.sh`](./local_QA.sh) Performs locally the same checks that will be run on github upon making a pull request (PR).
   In practice, syncing local and remote versions of various components can be difficult. The result is that
   occasionally, local tests pass, while remote tests fail.

   See comments in [`local_QA.sh`](./local_QA.sh) for more details.

- [`generate_measurement_tables.sh`](./generate_measurement_tables.sh) Runs `bicycle_compiler` to generate
  measurement tables `table_gross` and `table_two-gross` in [`../data/`](../data) if they are not already
  present. If the measurement tables _are_ already present, do nothing and exit with success.

  See comments in [`generate_measurement_tables.sh`](./generate_measurement_tables.sh) for more details.

- [`generate_tables_and_random_numerics.sh`](./generate_tables_and_random_numerics.sh) Generate measurement
  tables if they do not already exist. Then run `random_numerics` with several sets of parameters, specified
  in [`parameters.csv`](./parameters.csv).

  See comments in [`generate_tables_and_random_numerics.sh`](./generate_tables_and_random_numerics.sh) for more
  details.

## Other files

- [`run_random_numerics.py`](./run_random_numerics.py) and [`run_random_numerics.sh`](./run_random_numerics.sh)
  These are not meant to be run directly, but rather from [`generate_tables_and_random_numerics.sh`](./generate_tables_and_random_numerics.sh).

- [`parameters.csv`](./parameters.csv) This file contains input parameters for the executable `random_numerics`. One each line, the
  fields are: model name, noise level, number of qubits. The script [`generate_tables_and_random_numerics.sh`](./generate_tables_and_random_numerics.sh)
  runs `random_numerics` concurrently for each set of parameters (line) in [`parameters.csv`](./parameters.csv)

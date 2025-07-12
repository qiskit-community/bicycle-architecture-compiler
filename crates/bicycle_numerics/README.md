# `bicycle_numerics`

This package collects in circuits of bicycle ISA instructions
and assigns logical error rates, $P_i$, for each bicycle instruction as well as timing information.
Using this, it computes the execution time (in syndrome cycles) and circuit failure probability.

The input of the program is the output of `bicycle_compiler`.

The output of the program is of the form
```csv
code,p,i,qubits,idles,t_injs,automorphisms,measurements,joint_measurements,measurement_depth,end_time,total_error
gross,0.0001,1,121,113,92,16,123,1,21,19924,1.2673e-7
gross,0.0001,2,121,2068,92,14,114,1,42,38811,2.6381e-7
gross,0.0001,3,121,2068,92,14,114,1,63,57698,4.0089e-7
gross,0.0001,4,121,2068,92,14,114,1,84,76585,5.3797e-7
...
```


## Usage
For example, consider we have a circuit saved in `simulation_circuit_twogross.json`,
then we can collect numerics for that circuit by running
```
cat simulation_circuit_twogross.json | cargo run --release -- 120 two-gross_1e-4
```

The help output is

```
Usage: bicycle_numerics [OPTIONS] <QUBITS> <MODEL>

Arguments:
  <QUBITS>
  <MODEL>   [possible values: gross_1e-3, gross_1e-4, two-gross_1e-3, two-gross_1e-4]

Options:
  -e, --max-error <MAX_ERROR>  [default: 0.3333333333333333]
  -i, --max-iter <MAX_ITER>    [default: 1000000]
  -h, --help                   Print help
```

1. The `max-error` is the circuit failure probability to halt at. (TODO: Optionally enable max)
2. The `max-iter` is a maximum number of iterations to process and halt. (TODO: Optionally enable max)

## Counting the total number of instructions
The output of the numerics includes the number of gates in each row of input circuit.
If you would like totals, then the `cumulative_instructions.awk` can compute those for you.
To use it, you need a recent `awk` with `--csv` support, such as GNU `gawk`.
Then run
```
shell> cat results.csv | gawk --csv -f cumulative_instruction.awk
```

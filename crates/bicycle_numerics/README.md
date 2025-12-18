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
Compute numerics for bicycle circuits

Usage: bicycle_numerics [OPTIONS] <QUBITS> <MODEL>

Arguments:
  <QUBITS>
          Number of logical qubits in the input circuit (do not include pivot ancillas)

  <MODEL>
          Choose which architecture the circuit is run on

          Possible values:
          - gross_1e-3:     Gross codes with physical noise rate p=10^-3
          - gross_1e-4:     Gross codes with physical noise rate p=10^-4
          - two-gross_1e-3: Two-gross codes with physical noise rate p=10^-3
          - two-gross_1e-4: Two-gross codes with physical noise rate p=10^-4
          - fake_slow:      A model that has no physical noise, p=0, and worst-case timing information between all of the previous models

Options:
  -e, --max-error <MAX_ERROR>
          Set a limit to the error rate when the numerics should halt

  -i, --max-iter <MAX_ITER>
          Set a limit to the number of input lines (PBC gates) before halting

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
```

1. The `max-error` is the circuit failure probability to halt at.
2. The `max-iter` is a maximum number of iterations to process and halt.

## Counting the total number of instructions
The output of the numerics includes the number of gates in each row of input circuit.
If you would like totals, then the `cumulative_instructions.awk` can compute those for you.
To use it, you need a recent `awk` with `--csv` support, such as GNU `gawk`.
Then run
```
shell> cat results.csv | gawk --csv -f cumulative_instruction.awk
```

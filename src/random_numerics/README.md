# Random benchmarks
This package combines generating random benchmarks,
feeding them into the pbc_gross compiler and optimizing the results,
and finally collecting numerics.
The reason it exists is to avoid serialization overhead which became prohibitive for using JSON.
There are surely better solutions that using JSON but just programmatically joining the libraries together
offers the quickest and fastest solution for now.
## (C) Copyright IBM 2025
##
## This code is licensed under the Apache License, Version 2.0. You may
## obtain a copy of this license in the LICENSE.txt file in the root directory
## of this source tree or at http:##www.apache.org/licenses/LICENSE-2.0.
##
## Any modifications or derivative works of this code must retain this
## copyright notice, and modified files need to carry a notice indicating
## that they have been altered from the originals.

{
    i+=$5;
    t+=$6;
    a+=$7;
    m+=$8;
    j+=$9
}
END{
    print "idles,automorphisms,measurements,joint measurements,t_injs";
    printf "%i & %i & %i & %i & %i\n", i,a,m,j,t;
}

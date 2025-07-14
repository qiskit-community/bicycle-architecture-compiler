# Copyright contributors to the Bicycle Architecture Compiler project

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

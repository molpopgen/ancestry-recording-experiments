#!/usr/bin/bash

N=1000
nsteps=1000
nsteps=300
L=10000

for rho in 500 # 1000 10000
do
    echo "dynamic -> $rho"
    /usr/bin/time -f "%e %M" ./target/release/benchmark -N $N -r $rho -L $L --seed 101 --nsteps $nsteps -d 1.0 dynamic

    echo "tskit -> $rho"
    /usr/bin/time -f "%e %M" ./target/release/benchmark -N $N -r $rho -L $L --seed 101 --nsteps $nsteps -d 1.0 tskit -s 1
done

#!/usr/bin/bash

N=10000
nsteps=5000
nsteps=300
L=10000

for rho in 10000
do
    echo "dynamic -> $rho"
    /usr/bin/time -f "%e %M" ./target/release/benchmark -N $N -r $rho -L $L --seed 101 --nsteps $nsteps -d 1.0 dynamic

    for interval in 1 100 1000
    do
        echo "tskit -> $rho, $interval"
        /usr/bin/time -f "%e %M" ./target/release/benchmark -N $N -r $rho -L $L --seed 101 --nsteps $nsteps -d 1.0 tskit -s $interval
    done
done

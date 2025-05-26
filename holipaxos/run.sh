#!/bin/bash

pids=()
mkdir -p stats stats-history
output=stats-history/$2

cleanup() {
    for pid in "${pids[@]}"; do
        kill "$pid"
    done
    cp -r stats ${output}/
    exit 1
}

trap cleanup SIGINT

./bin/replicant -id $1 &
pids+=($!)

top -b -d 1 -p $! | sed -u -n "7p;8~9p" | awk '{ print(systime(), $0); fflush(); }' | tee stats/cpu-mem.dat &
pids+=($!)

#ifstat -n -t > stats/network.dat &
#pids+=($!)

wait
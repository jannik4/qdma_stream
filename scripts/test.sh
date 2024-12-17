#!/bin/bash

# Set default count_h2c to 4 if not provided
count_h2c=${1:-4}

# Set default count_c2h to count_h2c if not provided
count_c2h=${2:-$count_h2c}

cd "$(dirname "$0")"

# Start queues
./start.sh $count_h2c $count_c2h
echo

# H2C
echo "### Run H2C ###"
for (( i=0; i<count_h2c; i++ )); do
    dma-to-device -d /dev/qdmac1000-ST-$i -s 4096 -c 262144 &
done
wait
echo

# H2C Rust
echo "### Run H2C Rust ###"
../target/release/examples/h2c_simple $count_h2c
echo

# C2H
echo "### Run C2H ###"
for (( i=0; i<count_c2h; i++ )); do
    dma-from-device -d /dev/qdmac1000-ST-$i -s 4096 -c 262144 &
done
wait
echo

# C2H Rust
echo "### Run C2H Rust ###"
../target/release/examples/c2h_simple $count_c2h
echo

# Stop queues
./stop.sh $count_h2c $count_c2h

#!/bin/bash

# Set default count to 4 if not provided
count=${1:-4}

cd "$(dirname "$0")"

# Start queues
./start.sh $count
echo

# H2C
echo "### Run H2C ###"
for (( i=0; i<count; i++ )); do
    dma-to-device -d /dev/qdmac1000-ST-$i -s 4096 -c 262144 &
done
wait
echo

# H2C Rust
echo "### Run H2C Rust ###"
../target/release/examples/simple $count
echo

# C2H
echo "### Run C2H ###"
for (( i=0; i<count; i++ )); do
    dma-from-device -d /dev/qdmac1000-ST-$i -s 4096 -c 262144 &
done
wait
echo

# Stop queues
./stop.sh $count

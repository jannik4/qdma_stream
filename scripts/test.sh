#!/bin/bash

cd "$(dirname "$0")"

echo "### Starting ###"
./start.sh
echo

# H2C
echo "### Run H2C ###"
dma-to-device -d /dev/qdmac1000-ST-0 -s 4096 -c 262144 &
dma-to-device -d /dev/qdmac1000-ST-1 -s 4096 -c 262144 &
dma-to-device -d /dev/qdmac1000-ST-2 -s 4096 -c 262144 &
dma-to-device -d /dev/qdmac1000-ST-3 -s 4096 -c 262144 &
wait
echo

# H2C Rust
echo "### Run H2C Rust ###"
../target/release/examples/simple
echo

# C2H
echo "### Run C2H ###"
dma-from-device -d /dev/qdmac1000-ST-0 -s 4096 -c 262144 &
dma-from-device -d /dev/qdmac1000-ST-1 -s 4096 -c 262144 &
dma-from-device -d /dev/qdmac1000-ST-2 -s 4096 -c 262144 &
dma-from-device -d /dev/qdmac1000-ST-3 -s 4096 -c 262144 &
wait
echo

echo "### Stopping ###"
./stop.sh

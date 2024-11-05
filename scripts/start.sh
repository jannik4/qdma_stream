#!/bin/bash

# Set default count to 4 if not provided
count=${1:-4}

echo "### Starting ${count} queues ###"

# H2C (Host to Card)
for (( i=0; i<count; i++ )); do
    dma-ctl qdmac1000 q add idx $i mode st dir h2c
    dma-ctl qdmac1000 q start idx $i dir h2c fetch_credit h2c
done

# C2H (Card to Host)
for (( i=0; i<count; i++ )); do
    dma-ctl qdmac1000 q add idx $i mode st dir c2h
    dma-ctl qdmac1000 q start idx $i dir c2h
done

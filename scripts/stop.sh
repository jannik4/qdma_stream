#!/bin/bash

# Set default count to 4 if not provided
count=${1:-4}

echo "### Stopping ${count} queues ###"

# H2C (Host to Card)
for (( i=0; i<count; i++ )); do
    dma-ctl qdmac1000 q stop idx $i dir h2c
    dma-ctl qdmac1000 q del idx $i dir h2c
done

# C2H (Card to Host)
for (( i=0; i<count; i++ )); do
    dma-ctl qdmac1000 q stop idx $i dir c2h
    dma-ctl qdmac1000 q del idx $i dir c2h
done

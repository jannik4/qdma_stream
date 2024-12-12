#!/bin/bash

# Set default count_h2c to 4 if not provided
count_h2c=${1:-4}

# Set default count_c2h to count_h2c if not provided
count_c2h=${2:-$count_h2c}

echo "### Stopping ${count} queues ###"

# H2C (Host to Card)
for (( i=0; i<count_h2c; i++ )); do
    dma-ctl qdmac1000 q stop idx $i dir h2c
    dma-ctl qdmac1000 q del idx $i dir h2c
done

# C2H (Card to Host)
for (( i=0; i<count_c2h; i++ )); do
    dma-ctl qdmac1000 q stop idx $i dir c2h
    dma-ctl qdmac1000 q del idx $i dir c2h
done

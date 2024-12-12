#!/bin/bash

# Set default count_h2c to 4 if not provided
count_h2c=${1:-4}

# Set default count_c2h to count_h2c if not provided
count_c2h=${2:-$count_h2c}

echo "### Starting ${count} queues ###"

# H2C (Host to Card)
for (( i=0; i<count_h2c; i++ )); do
    dma-ctl qdmac1000 q add idx $i mode st dir h2c
    dma-ctl qdmac1000 q start idx $i dir h2c fetch_credit h2c
done

# C2H (Card to Host)
for (( i=0; i<count_c2h; i++ )); do
    dma-ctl qdmac1000 q add idx $i mode st dir c2h
    dma-ctl qdmac1000 q start idx $i dir c2h
done

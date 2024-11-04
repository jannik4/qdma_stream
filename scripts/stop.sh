#!/bin/bash

# H2C
dma-ctl qdmac1000 q stop idx 0 dir h2c
dma-ctl qdmac1000 q stop idx 1 dir h2c
dma-ctl qdmac1000 q stop idx 2 dir h2c
dma-ctl qdmac1000 q stop idx 3 dir h2c

dma-ctl qdmac1000 q del idx 0 dir h2c
dma-ctl qdmac1000 q del idx 1 dir h2c
dma-ctl qdmac1000 q del idx 2 dir h2c
dma-ctl qdmac1000 q del idx 3 dir h2c

# C2H
dma-ctl qdmac1000 q stop idx 0 dir c2h
dma-ctl qdmac1000 q stop idx 1 dir c2h
dma-ctl qdmac1000 q stop idx 2 dir c2h
dma-ctl qdmac1000 q stop idx 3 dir c2h

dma-ctl qdmac1000 q del idx 0 dir c2h
dma-ctl qdmac1000 q del idx 1 dir c2h
dma-ctl qdmac1000 q del idx 2 dir c2h
dma-ctl qdmac1000 q del idx 3 dir c2h

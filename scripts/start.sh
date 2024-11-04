#!/bin/bash

# H2C
dma-ctl qdmac1000 q add idx 0 mode st dir h2c
dma-ctl qdmac1000 q add idx 1 mode st dir h2c
dma-ctl qdmac1000 q add idx 2 mode st dir h2c
dma-ctl qdmac1000 q add idx 3 mode st dir h2c

dma-ctl qdmac1000 q start idx 0 dir h2c fetch_credit h2c
dma-ctl qdmac1000 q start idx 1 dir h2c fetch_credit h2c
dma-ctl qdmac1000 q start idx 2 dir h2c fetch_credit h2c
dma-ctl qdmac1000 q start idx 3 dir h2c fetch_credit h2c

# C2H
dma-ctl qdmac1000 q add idx 0 mode st dir c2h
dma-ctl qdmac1000 q add idx 1 mode st dir c2h
dma-ctl qdmac1000 q add idx 2 mode st dir c2h
dma-ctl qdmac1000 q add idx 3 mode st dir c2h

dma-ctl qdmac1000 q start idx 0 dir c2h
dma-ctl qdmac1000 q start idx 1 dir c2h
dma-ctl qdmac1000 q start idx 2 dir c2h
dma-ctl qdmac1000 q start idx 3 dir c2h

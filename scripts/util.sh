#!/bin/bash

# Exit
exit 0

# Write/Read
sudo dma-to-device -d /dev/qdmac1000-ST-0 -s 4096
sudo dma-from-device -d /dev/qdmac1000-ST-0 -s 4096

# Dump
sudo dma-ctl qdmac1000 q dump idx 0 dir h2c | grep Credit
sudo dma-ctl qdmac1000 q dump idx 0 dir c2h

# ---------------- example design c2h ----------------
qid=4
packet_size=4096
packet_count=10

sudo dma-ctl qdmac1000 reg write bar 2 0x00 $qid # qid
sudo dma-ctl qdmac1000 q add idx $qid mode st dir c2h
sudo dma-ctl qdmac1000 q start idx $qid dir c2h cmptsz 0
sudo dma-ctl qdmac1000 reg write bar 2 0x04 $packet_size # packet size
sudo dma-ctl qdmac1000 reg write bar 2 0x20 $packet_count # num of packets
sudo dma-ctl qdmac1000 reg write bar 2 0x08 2 # trigger c2h data generator
sudo dma-from-device -d /dev/qdmac1000-ST-$qid -s $packet_size -c $packet_count
sudo dma-ctl qdmac1000 reg write bar 2 0x08 0x22 # finish data generator
sudo dma-ctl qdmac1000 q stop idx $qid dir c2h
sudo dma-ctl qdmac1000 q del idx $qid dir c2h
# ---------------- example design c2h ----------------

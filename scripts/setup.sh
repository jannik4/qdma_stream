#!/bin/bash
modprobe qdma-pf
dma-ctl dev list
bash -c 'echo 32 > /sys/bus/pci/devices/0000\:c1\:00.0/qdma/qmax'
dma-ctl dev list

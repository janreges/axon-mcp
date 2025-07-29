#!/bin/bash
echo "[$(date +%H:%M:%S)] $*" | tee -a coordination.log
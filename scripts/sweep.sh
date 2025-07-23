#!/usr/bin/env bash
net=${1:-192.168.86}       # default network; override with first arg
echo "Scanning $net.0/24 …"
for i in {1..254}; do
  ( ping -c1 -W1 "$net.$i" &>/dev/null && echo "$net.$i" ) &
done
wait
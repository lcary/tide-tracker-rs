#!/usr/bin/env bash
mkdir -p "$(pwd)/work"
docker run --rm -it --privileged \
  -v "$(pwd)/work":/opt/rpi-image-gen/work \
  -v "$(pwd)/overlays":/opt/rpi-image-gen/device/rootfs-overlay \
  -v "$(pwd)/scripts":/opt/rpi-image-gen/scripts \
  tide-tracker-img:bookworm \
  -c scripts/tidetracker.yaml

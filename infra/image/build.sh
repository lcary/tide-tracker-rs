#!/usr/bin/env bash
set -euxo pipefail

# Build Raspberry Pi OS image with Tide Tracker and Wi-Fi portal
echo "Building Tide Tracker Pi image with minimal captive portal..."

# Clean previous builds
rm -rf work/*

# Run rpi-image-gen with our configuration
python rpi-image-gen.py -c tidetracker.yaml -o work/

echo "Build complete! Image available in work/ directory"

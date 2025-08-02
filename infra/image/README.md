# Tide Tracker Pi Image Builder

This directory contains the infrastructure for building a custom Raspberry Pi OS image preloaded with Tide Tracker.

## Quick Start

1. **Build the Docker image:**
   ```bash
   cd infra/image
   ./build-docker.sh
   ```

2. **Generate the Pi image:**
   ```bash
   ./run-build.sh
   ```

3. **Flash the result:**
   The final image will be at `work/tide-tracker/artefacts/tide-tracker.img`

## What's Included

- **64-bit Bookworm Raspberry Pi OS** for Pi Zero 2 W
- **Tide Tracker binary** downloaded on first boot
- **Balena WiFi Connect** for captive portal setup
- **SSH enabled** by default
- **SPI enabled** for e-ink display
- **Systemd services** pre-configured and enabled
- **Watchdog** enabled for reliability

## First Boot Process

1. Pi boots with WiFi Connect captive portal
2. Connect to "TideTracker-Setup" network (password: "pi-tides")
3. Configure WiFi through web interface
4. System downloads and installs Tide Tracker binary
5. Services start automatically

## Services

- `tide-tracker.service` - Main application
- `tide-tracker.timer` - Updates every 10 minutes
- `wifi-connect.service` - Captive portal fallback
- `tide-tracker-setup.service` - First boot setup (runs once)

## Configuration

Default config is in `/etc/tide-tracker/tide-config.toml` and can be customized for different NOAA stations.

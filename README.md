# Tide Tracker

A lean, memory-efficient tide tracking application for Raspberry Pi Zero 2 W with Waveshare 4.2" e-ink display. Built entirely in Rust for maximum performance and reliability in embedded environments.

## Features

- **Real-time tide data** from NOAA with 10-minute granularity (145 samples over 24 hours)
- **Offline fallback** using semidiurnal sine wave model when network fails
- **Ultra-low memory** footprint (< 1MB peak usage)
- **E-ink optimized** rendering with 2px stroke width for crisp display
- **ASCII development mode** for testing on macOS/Linux without hardware
- **Robust caching** with 30-minute TTL to minimize network requests
- **Systemd integration** for reliable scheduled updates
- **WiFi Connect integration** for easy WiFi setup via captive portal

## Hardware Requirements

### Raspberry Pi (64-bit)
- Any modern Raspberry Pi, but only tested for Zero 2 W currently
- 500MB+ RAM recommended
- Headless Linux (Raspberry Pi OS 64-Bit recommended)
- SPI configuration
  - For hardware SPI: SPI **ENABLED** (`sudo raspi-config` → Interface Options → SPI → Enable → Reboot)
  - SSH should be enabled for remote access (`sudo raspi-config` → Interface Options → SSH → Enable)

### Waveshare 4.2" E-ink Display
- Resolution: 400 × 300 pixels
- Monochrome (black/white)
- SPI interface

## Installation

One-shot installation script (intended to be run on the Raspberry Pi, mind you):

```bash
curl -LsSf https://raw.githubusercontent.com/lcary/tide-tracker-rs/main/install.sh | bash
```

Note that this command runs sudo. If you are not comfortable with that and want to run each step yourself, or
for more information on installing the tide tracker, see: [INSTALLATION.md](./docs/INSTALLATION.md)

## Wi-Fi Setup

The Tide Tracker includes a minimal captive portal for easy Wi-Fi configuration on headless devices. When no internet connection is detected, it automatically creates a "TideTracker-Setup" hotspot for configuration.

### Installation
```bash
sudo ./scripts/wifi-portal-setup.sh
```

### Usage
- **Automatic**: Service runs on boot, activates portal when offline
- **Connect**: Look for "TideTracker-Setup" network (password: "pi-tides")
- **Configure**: Your device should show a captive portal page automatically
- **Monitor**: `journalctl -u wifi-portal.service -f`
- **Update**: `sudo ./scripts/wifi-portal-update.sh` (updates portal scripts only)

### How it works
1. On boot, waits 30 seconds for existing Wi-Fi connection
2. If no connection: starts AP mode on wlan0 with SSID "TideTracker-Setup"
3. Serves captive portal web page on 192.168.4.1
4. User submits Wi-Fi credentials via web form
5. Device connects to target network and disables portal
6. Normal operation resumes on the new Wi-Fi network

## User Manual

The general user manual still needs to be written.
The "power" user manual, with detailed steps for assembling the tide tracker (e.g. soldering, building the frame, etc.), can be found 
[here](https://docs.google.com/document/d/1YIPxZLHlb4GVWcRMvzlihrW_i_gc0iPF_CqIJD5hd4c/edit?tab=t.0).

See also the [wiring guide](./docs/WIRING.md) for more info on how to wire the Raspberry Pi to the e-Paper device.

## Development

### Building the Binary

The project successfully cross-compiles for ARM targets.

Building for Raspberry Pi (ARM64) requires Docker (e.g. Docker Desktop), then running:

```bash
./scripts/build_rpi.sh
```

For more information on building the tide tracker, see: [BUILD.md](./docs/BUILD.md)

### Project Structure
```
src/
├── lib.rs           # Core data structures
├── main.rs          # Application entry point
├── fallback.rs      # Offline sine wave model
├── tide_data.rs     # NOAA data fetching & caching
├── renderer.rs      # E-ink and ASCII rendering
└── tests/
    └── data_tests.rs # Unit tests

scripts/
├── wifi-portal-setup.sh     # Minimal captive portal installation script
├── wifi-portal-update.sh    # Portal update script
├── wifi-portal.sh           # Main portal service script
├── portal-web/              # Captive portal web content
│   ├── index.html           # Portal configuration page
│   └── cgi-bin/
│       └── wifi-connect.sh  # CGI handler for Wi-Fi credentials
├── nm-config/               # NetworkManager configuration
│   ├── dns.conf             # DNS config for captive portal
│   └── captive.conf         # DNS wildcard redirect
└── systemd/
    └── wifi-portal.service  # Systemd service definition

infra/
└── image/           # Docker-based Pi image building
    ├── Dockerfile
    ├── build.sh
    └── overlays/    # Rootfs overlay files
```

### Running Tests
```bash
cargo test
cargo test -- --nocapture  # See test output
```

### Debugging
```bash
# Enable debug logging
RUST_LOG=debug cargo run -- --stdout

# Check memory usage
sudo systemctl status tide-tracker.service
```

## Troubleshooting

### E-ink Display Issues
```bash
# Test GPIO pins
pinctrl get
```

**Hardware Pin Conflicts:**
If you have bad solder joints or pin conflicts, override GPIO pins in `tide-config.toml`:
```toml
[display.hardware]
cs_pin = 7    # Use GPIO 7 instead of 8
rst_pin = 27  # Use GPIO 27 instead of 17
```

### Network Issues
```bash
# Test NOAA endpoint
curl "https://tidesandcurrents.noaa.gov/noaatidepredictions.html?id=8410140"

# Check cache
cat /tmp/tide_cache.json
```

### Memory Issues
```bash
# Monitor memory usage
sudo systemctl status tide-tracker.service | grep Memory

# Check for memory leaks
valgrind --leak-check=full ./target/release/tide-tracker --stdout
```

## License

- MIT License

## Development Plan

- [x] Initial implementation
- [x] Test ASCII output (macOS)
- [x] Test Hardware build (Raspberry Pi)
- [x] Support configuration file loading
- [x] Test rendering (Waveshare e-Paper)
- [x] Fix persistence (e-Paper)
- [x] Fix mangled simple chart data (e-Paper)
- [x] Test actual chart rendering (e-Paper)
- [x] Replace deprecated rppal with gpio-cdev (or pull in rppal code)
- [x] Set up cronjob/svc
- [x] Test Pi device restart resilience
- [x] Build and install in frame
- [x] Optimization using SPI
- [x] Wifi connect with balena OS
- [ ] Show high/low times
- [ ] Support automatic config creation w/ location check

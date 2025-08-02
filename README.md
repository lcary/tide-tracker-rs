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

## Hardware Requirements (Optional)

### Raspberry Pi (64-bit)
- Any modern Raspberry Pi, but only tested for Zero 2 W currently
- 500MB+ RAM recommended
- Headless Linux (Raspberry Pi OS Lite recommended)
- SPI configuration
  - For hardware SPI: SPI **ENABLED** (`sudo raspi-config` → Interface Options → SPI → Enable → Reboot)
  - SSH should be enabled for remote access (`sudo raspi-config` → Interface Options → SSH → Enable)

#### Enable SPI and SSH on Raspberry Pi
```bash
sudo raspi-config
# Interface Options → SPI → Enable
# Interface Options → SSH → Enable
sudo reboot
```

## WiFi Connect Integration

WiFi Connect provides a captive portal for easy WiFi setup on headless devices. When no internet connection is detected, it automatically creates a "TideTracker-Setup" hotspot (password: "pi-tides") for configuration.

### Installation
```bash
sudo ./scripts/wifi-setup.sh
```

### Usage
- **Automatic**: Service runs on boot, activates portal when offline
- **Manual**: `sudo systemctl start wifi-connect.service`
- **Monitor**: `journalctl -u wifi-connect.service -f`
- **Update**: `sudo ./scripts/wifi-update.sh` (updates connectivity script only)

### Troubleshooting
```bash
# Check status
systemctl status wifi-connect.service

# Test connectivity  
ping -c 3 1.1.1.1

# Manual debug mode
sudo systemctl stop wifi-connect.service
sudo /usr/local/sbin/wifi-connect --portal-ssid "TideTracker-Setup" --portal-passphrase "pi-tides"
```

### Waveshare 4.2" E-ink Display
- Resolution: 400 × 300 pixels
- Monochrome (black/white)
- SPI interface

## Cross-Compilation

The project successfully cross-compiles for ARM targets.
### Building for Raspberry Pi (ARM64)

#### Option 1: Using Docker (Recommended)

Build for Raspberry Pi using Docker with hardware features (e-ink display)
```
./scripts/build_rpi.sh
```
(Runs `cross build --release --target=aarch64-unknown-linux-gnu --features hardware`)

#### Option 2: GitHub Actions CI
The project includes GitHub Actions workflows that automatically build ARM64 binaries:
- Push to main branch triggers ARM64 cross-compilation
- Release tags automatically build and upload ARM64 binaries

- **Platform Separation**: ✅ macOS/Linux incompatibilities resolved
- **Recommended Method**: Use `cross` (Option 1) for reliable builds
- **Native Toolchain**: Requires `aarch64-unknown-linux-gnu-gcc` but may have dependency conflicts

**If you encounter build errors:** The `ring` crate (used by `rustls` for TLS) requires cross-compilation toolchain. Use `cross` (Docker-based) which handles all dependencies automatically.

## Wiring Diagram

Connect the Waveshare 4.2" e-ink display to your Raspberry Pi:

**Standard Wiring:**
```
Raspberry Pi GPIO     →    E-ink Display
─────────────────────────────────────
3.3V (Pin 1)      →    VCC
GND (Pin 6)       →    GND
GPIO 10 (Pin 19)  →    DIN (MOSI)
GPIO 11 (Pin 23)  →    CLK (SCLK)
GPIO 8 (Pin 24)   →    CS
GPIO 25 (Pin 22)  →    DC
GPIO 17 (Pin 11)  →    RST
GPIO 24 (Pin 18)  →    BUSY
```

**Alternative Wiring (for hardware conflicts):**
```
Raspberry Pi GPIO     →    E-ink Display
─────────────────────────────────────
3.3V (Pin 1)      →    VCC
GND (Pin 6)       →    GND
GPIO 10 (Pin 19)  →    DIN (MOSI)  
GPIO 11 (Pin 23)  →    CLK (SCLK)
GPIO 7 (Pin 26)   →    CS           # Alternative CS pin
GPIO 25 (Pin 22)  →    DC
GPIO 27 (Pin 13)  →    RST          # Alternative RST pin  
GPIO 24 (Pin 18)  →    BUSY
```

### Pin Layout Reference
```
     3.3V → [ 1] [ 2]
            [ 3] [ 4]
            [ 5] [ 6] ← GND
      ALT CS→[ 7] [ 8]
            [ 9] [10]
    RST → [11] [12]
   ALT RST→[13] [14]
           [15] [16]
           [17] [18] ← BUSY
   MOSI → [19] [20]
           [21] [22] ← DC
    CLK → [23] [24] ← CS
           [25] [26] ← ALT CS (GPIO 7)
```

**Legend:**
- Standard pins: CS=8, RST=17  
- Alternative pins: ALT CS=7, ALT RST=27

## Installation & Setup (on macOS)

### 1. Install Rust

Also install cross for cross-compilation: `cargo install cross`

### 2. Build

 - Start Docker
 - `./scripts/build_rpi.sh`
 - `scp target/aarch64-unknown-linux-gnu/release/tide-tracker pi@0.0.0.0:~`

## Installation & Setup (on Pi)

### 1. Enable SPI and SSH
See above for `raspi-config` instructions.

### 2. Install Rust on Raspberry Pi
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 3. Clone and Build
```bash
git clone <repository-url>
cd tide-tracker
cargo build --release
```

### 4. Test Installation
```bash
# Test with ASCII output (no hardware required)
cargo run --release -- --stdout

# Test on actual e-ink display
sudo ./target/release/tide-tracker
```

## Usage

### Development Mode (ASCII Output)
Perfect for testing on your development machine:
```bash
cargo run --release --bin tide-tracker -- --stdout
```

Output example:
```
Loaded configuration for station: Portland, ME
                      ••••••••
                   •••        •••
9   │            ••              ••                                                                  •••
                •                  •                                                            •••••   ••••
8   │         ••                    ••                                                        ••            •••
             •                        •                                                     ••                 ••
            •                          •                                                   •                     •
7   │     ••                            •                                                ••                       ••
         •                               ••                                             •                           •
6   │   •                                  •                                           •                             •
       •                                    •                                         •                               ••
5   │••                                      •                                       •                                  •
                                              •                                     •                                    •
                                               •                                  ••                                      •
4   │                                           •                                •                                         ••
                                                 •                              •                                            •
3   │                                             •                            •                                              •                     ••
                                                   •                          •                                                ••                  •
2   │                                               •                       •X                                                   ••              ••
                                                     ••                    •                                                       ••         •••
                                                       •                 ••                                                          •••••••••
1   │                                                   ••             ••
                                                          •••       •••
0   │                                                        •••••••
     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     |
     -12h                                                                   Now                                                                   +12h
```

### Production Mode (E-ink Display)
```bash
sudo ./target/release/tide-tracker
```

## Systemd Integration

### 1. Install Binary
```bash
sudo cp target/release/tide-tracker /usr/local/bin/
sudo chmod +x /usr/local/bin/tide-tracker
```

### 2. Create Service File
Create `/etc/systemd/system/tide-tracker.service`:
```ini
[Unit]
Description=Tide Tracker Display Update
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/tide-tracker
User=root
Group=root
StandardOutput=journal
StandardError=journal

# Memory limits for Raspberry Pi
MemoryMax=4M
MemoryHigh=2M

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictRealtime=true
```

### 3. Create Timer File
Create `/etc/systemd/system/tide-tracker.timer`:
```ini
[Unit]
Description=Update tide tracker every 10 minutes
Requires=tide-tracker.service

[Timer]
OnBootSec=2min
OnUnitActiveSec=10min
AccuracySec=1min

[Install]
WantedBy=timers.target
```

### 4. Enable and Start
```bash
sudo systemctl daemon-reload
sudo systemctl enable tide-tracker.timer
sudo systemctl start tide-tracker.timer

# Check status
sudo systemctl status tide-tracker.timer
sudo journalctl -u tide-tracker.service -f
```

## Configuration

### GPIO Pin Configuration

The e-ink display GPIO pins are configurable via the `tide-config.toml` file. **Do NOT configure cs_pin for hardware SPI; CS is managed by the kernel SPI driver.**

**Default Pin Mapping:**
```toml
[display.hardware]
# cs_pin = 8   # REMOVE or comment out this line for hardware SPI
rst_pin = 17
busy_pin = 24
dc_pin = 25
```

**Important Notes:**
- Ensure your physical wiring matches your configuration
- Do not set cs_pin in config for hardware SPI
- Changes require restarting the tide-tracker service
- The configuration is loaded from the current directory's `tide-config.toml`

### Tide Station
The default configuration uses Boston Harbor (NOAA Station ID: 8410140). To change:
1. Find your station at https://tidesandcurrents.noaa.gov/
2. Edit `src/tide_data.rs` and update the URL with your station ID

### Cache Settings
- **Location**: `/tmp/tide_cache.json`
- **TTL**: 30 minutes
- **Purpose**: Reduces network requests and improves reliability

### Memory Optimization
The application is optimized for Raspberry Pi's memory:
- Pre-allocated vectors with known capacity
- Minimal string allocations
- Efficient binary serialization
- No memory leaks across runs

## Development

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
├── wifi-setup.sh         # WiFi Connect installation script
├── wifi-update.sh        # WiFi Connect update script
├── wifi-connect-loop.sh  # Connectivity monitoring script
└── systemd/
    └── wifi-connect.service # Systemd service definition

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
- [ ] Support automatic config creation w/ location check

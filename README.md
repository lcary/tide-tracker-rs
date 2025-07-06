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

## Hardware Requirements (Optional)

### Raspberry Pi (64-bit)
- Any modern Raspberry Pi, but only tested for Zero 2 W currently
- 500MB+ RAM recommended
- Headless Linux (Raspberry Pi OS Lite recommended)  
- SPI configuration
  - for 4.2": SPI DISABLED (`sudo raspi-config` → Interface Options → SPI → Disabled → Reboot)
    - https://www.waveshare.com/wiki/4.2inch_e-Paper_Module_(B)_Manual#Python

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
./build_rpi.sh
```
(Runs `cross build --release --target=aarch64-unknown-linux-gnu --features hardware`)

#### Option 2: GitHub Actions CI
The project includes GitHub Actions workflows that automatically build ARM64 binaries:
- Push to main branch triggers ARM64 cross-compilation
- Release tags automatically build and upload ARM64 binaries

### Cross-Compilation Notes

- **Code Status**: ✅ All Rust code compiles successfully for ARM64
- **Dependencies**: ✅ Hardware-specific deps are properly conditional
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
 - `./build_rpi.sh`
 - `scp target/aarch64-unknown-linux-gnu/release/tide-tracker pi@0.0.0.0:~`

## Installation & Setup (on Pi)

### 1. Install Rust on Raspberry Pi
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Clone and Build
```bash
git clone <repository-url>
cd tide-tracker
cargo build --release
```

### 3. Test E-ink Display (Optional)
Before running the main application, verify your e-ink display works:

```bash
# Quick test suite (Python + Rust)
./scripts/test_display.sh

# Python version (requires: pip3 install waveshare-epd pillow)
python3 scripts/test_display.py

# Rust version (uses same hardware libraries as main app)
cargo run --bin test_display --features hardware --release
```

These test scripts will:
- ✅ Check SPI connectivity
- 🎨 Display test patterns and text  
- 🧹 Clear the display
- 💤 Put display to sleep properly

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

The e-ink display GPIO pins are configurable via the `tide-config.toml` file. This allows you to override the default wiring if you have pin conflicts or hardware issues.

**Default Pin Mapping:**
```toml
[display.hardware]
cs_pin = 8    # GPIO 8 (Pin 24) - SPI Chip Select
dc_pin = 25   # GPIO 25 (Pin 22) - Data/Command
rst_pin = 17  # GPIO 17 (Pin 11) - Reset
busy_pin = 24 # GPIO 24 (Pin 18) - Busy status
```

**Alternative Configuration Example:**
If you have hardware conflicts (e.g., bad solder joints), you can override pins:
```toml
[display.hardware]
cs_pin = 7    # GPIO 7 (Pin 26) - Alternative CS pin
rst_pin = 27  # GPIO 27 (Pin 13) - Alternative reset pin
# Keep other pins as default
dc_pin = 25
busy_pin = 24
```

**Important Notes:**
- Ensure your physical wiring matches your configuration
- Changes require restarting the tide-tracker service
- Test with `./scripts/test_display.sh` after making changes
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
# Check SPI, expect 0 and 1
ls /dev/spi*

# Test GPIO pins
pinctrl get

# Test display with current configuration
./scripts/test_display.sh

# Check configuration is loaded correctly  
cargo run --bin tide-tracker --release -- --stdout | head -3
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

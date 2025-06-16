# Tide Tracker

A lean, memory-efficient tide tracking application for Raspberry Pi Zero W with Waveshare 4.2" e-ink display. Built entirely in Rust for maximum performance and reliability in embedded environments.

## Features

- **Real-time tide data** from NOAA with 10-minute granularity (145 samples over 24 hours)
- **Offline fallback** using semidiurnal sine wave model when network fails
- **Ultra-low memory** footprint (< 1MB peak usage)
- **E-ink optimized** rendering with 2px stroke width for crisp display
- **ASCII development mode** for testing on macOS/Linux without hardware
- **Robust caching** with 30-minute TTL to minimize network requests
- **Systemd integration** for reliable scheduled updates

## Hardware Requirements

### Raspberry Pi Zero W
- 512 MB RAM
- Headless Linux (Raspberry Pi OS Lite recommended)
- SPI enabled (`sudo raspi-config` → Interface Options → SPI → Enable)

### Waveshare 4.2" E-ink Display
- Resolution: 400 × 300 pixels
- Monochrome (black/white)
- SPI interface

## Wiring Diagram

Connect the Waveshare 4.2" e-ink display to your Raspberry Pi Zero W:

```
Pi Zero W GPIO    →    E-ink Display
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

### Pin Layout Reference
```
     3.3V → [ 1] [ 2]
            [ 3] [ 4]
            [ 5] [ 6] ← GND
            [ 7] [ 8]
            [ 9] [10]
    RST → [11] [12]
           [13] [14]
           [15] [16]
           [17] [18] ← BUSY
   MOSI → [19] [20]
           [21] [22] ← DC
    CLK → [23] [24] ← CS
           [25] [26]
```

## Installation & Setup

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

### 3. Test Installation
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
cargo run --release -- --stdout
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

# Memory limits for Pi Zero W
MemoryMax=2M
MemoryHigh=1M

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

### Tide Station
The default configuration uses Boston Harbor (NOAA Station ID: 8410140). To change:
1. Find your station at https://tidesandcurrents.noaa.gov/
2. Edit `src/tide_data.rs` and update the URL with your station ID

### Cache Settings
- **Location**: `/tmp/tide_cache.json`
- **TTL**: 30 minutes
- **Purpose**: Reduces network requests and improves reliability

### Memory Optimization
The application is optimized for Pi Zero W's limited memory:
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
# Check SPI is enabled
ls /dev/spi*

# Verify GPIO permissions
sudo usermod -a -G gpio,spi $USER

# Test GPIO pins
gpio readall
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

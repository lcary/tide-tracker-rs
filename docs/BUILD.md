# Build

This document contains build information for building the Raspberry Pi.

## Cross-Compilation

The project successfully cross-compiles for ARM targets.

### Building for Raspberry Pi (ARM64)

#### Option 1: Using Docker (Recommended)

Build for Raspberry Pi using Docker with hardware features (e-ink display):

```bash
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
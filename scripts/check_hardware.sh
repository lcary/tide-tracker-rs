#!/bin/bash
# Hardware diagnostic script for Raspberry Pi e-ink display setup
# Run this on the Pi to check what devices are available

echo "🔍 Checking hardware availability for e-ink display..."
echo

echo "📊 System Info:"
echo "  Kernel: $(uname -r)"
echo "  Architecture: $(uname -m)"
echo "  Model: $(cat /proc/device-tree/model 2>/dev/null || echo 'Unknown')"
echo

echo "🔌 SPI Devices:"
if ls /dev/spidev* 2>/dev/null; then
    echo "  ✅ SPI devices found"
    for dev in /dev/spidev*; do
        echo "    $dev ($(ls -l $dev | cut -d' ' -f1,3,4))"
    done
else
    echo "  ❌ No SPI devices found"
    echo "     Run: sudo raspi-config -> Interface Options -> SPI -> Enable"
fi
echo

echo "🎛️  GPIO Devices:"
if ls /dev/gpiochip* 2>/dev/null; then
    echo "  ✅ GPIO devices found"
    for dev in /dev/gpiochip*; do
        echo "    $dev ($(ls -l $dev | cut -d' ' -f1,3,4))"
    done
else
    echo "  ❌ No GPIO devices found"
fi
echo

echo "📋 GPIO Pin Status (for configured pins):"
# Check if GPIO pins are available
for pin in 7 24 25 27; do
    if [ -d "/sys/class/gpio/gpio$pin" ]; then
        echo "  GPIO $pin: Already exported"
    elif [ -e "/sys/class/gpio/export" ]; then
        echo "  GPIO $pin: Available"
    else
        echo "  GPIO $pin: Unknown status"
    fi
done
echo

echo "🔧 Configuration File Check:"
if [ -f "tide-config.toml" ]; then
    echo "  ✅ tide-config.toml found"
    echo "  📄 Display hardware config:"
    grep -A 5 "\[display.hardware\]" tide-config.toml 2>/dev/null || echo "    No hardware config section found"
else
    echo "  ❌ tide-config.toml not found in current directory"
fi
echo

echo "👤 Permissions Check:"
echo "  Current user: $(whoami)"
echo "  Groups: $(groups)"
if groups | grep -q "spi\|gpio\|sudo"; then
    echo "  ✅ User has appropriate group membership"
else
    echo "  ⚠️  User may need to be added to 'spi' and 'gpio' groups"
    echo "     Run: sudo usermod -a -G spi,gpio $(whoami)"
fi
echo

echo "📦 Rust Environment:"
if command -v cargo >/dev/null 2>&1; then
    echo "  ✅ Cargo found: $(cargo --version)"
    echo "  📁 Target directory:"
    if [ -d "target" ]; then
        echo "    $(du -sh target 2>/dev/null || echo 'Unknown size')"
    else
        echo "    No target directory found"
    fi
else
    echo "  ❌ Cargo not found - install Rust toolchain"
fi
echo

echo "🚀 Recommended next steps:"
echo "1. Enable SPI if not already: sudo raspi-config"
echo "2. Add user to groups: sudo usermod -a -G spi,gpio \$(whoami)"
echo "3. Reboot after making changes: sudo reboot"
echo "4. Build on device: cargo build --release --features hardware"
echo "5. Test hardware: sudo ./target/release/tide-tracker"
echo

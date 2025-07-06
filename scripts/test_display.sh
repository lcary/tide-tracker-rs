#!/bin/bash
# E-ink Display Test Suite
# Tests the Waveshare 4.2" e-ink display on Raspberry Pi Zero 2 W

set -e

echo "🚀 E-ink Display Test Suite"
echo "=========================="
echo "Testing Waveshare 4.2\" display on Raspberry Pi Zero 2 W"
echo ""

# Check if we're on a Raspberry Pi
if [ -f /proc/cpuinfo ]; then
    if ! grep -q "Raspberry Pi" /proc/cpuinfo; then
        echo "⚠️  WARNING: This doesn't appear to be a Raspberry Pi"
        read -p "Continue anyway? [y/N]: " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 0
        fi
    fi
fi

# Check SPI
if [ ! -e /dev/spidev0.0 ]; then
    echo "❌ ERROR: SPI device not found at /dev/spidev0.0"
    echo "   Enable SPI: sudo raspi-config → Interface Options → SPI → Enable"
    echo "   Then reboot: sudo reboot"
    exit 1
fi

echo "✅ SPI device found at /dev/spidev0.0"
echo ""

# Test 1: Python version (if available)
echo "📋 Test 1: Python E-ink Test"
echo "-----------------------------"
if command -v python3 &> /dev/null; then
    echo "🐍 Running Python test script..."
    if python3 scripts/test_display.py; then
        echo "✅ Python test completed successfully"
    else
        echo "❌ Python test failed"
        echo "💡 This is usually due to missing Python dependencies"
        echo "   Install with: pip3 install waveshare-epd pillow"
    fi
else
    echo "⚠️  Python3 not found, skipping Python test"
fi

echo ""

# Test 2: Rust version
echo "📋 Test 2: Rust E-ink Test"
echo "---------------------------"
if command -v cargo &> /dev/null; then
    echo "🦀 Running Rust test..."
    if cargo run --bin test_display --features hardware --release; then
        echo "✅ Rust test completed successfully"
    else
        echo "❌ Rust test failed"
        echo "💡 Make sure the hardware feature is enabled and you're on Linux"
    fi
else
    echo "⚠️  Cargo/Rust not found, skipping Rust test"
fi

echo ""
echo "🏁 Test suite completed!"
echo ""
echo "📝 If both tests passed, your e-ink display is working correctly."
echo "🔧 If tests failed, check:"
echo "   1. SPI enabled: sudo raspi-config → Interface Options → SPI"
echo "   2. Correct wiring to Waveshare 4.2\" display"
echo "   3. Permissions: run as sudo or add user to spi/gpio groups"
echo "   4. Dependencies installed for chosen test method"

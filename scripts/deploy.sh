#!/bin/bash
# Deployment script for tide-tracker with e-ink display support
# Run this after building to deploy the updated binary to your Pi

set -e

echo "🚀 Tide Tracker Deployment Script"
echo "================================="
echo

# Check if binary exists
BINARY_PATH="/Users/lcary/w/tide-tracker/target/aarch64-unknown-linux-gnu/release/tide-tracker"
if [ ! -f "$BINARY_PATH" ]; then
    echo "❌ Binary not found at $BINARY_PATH"
    echo "   Run: cross build --release --target=aarch64-unknown-linux-gnu --features hardware"
    exit 1
fi

echo "✅ Binary found: $(file $BINARY_PATH | cut -d: -f2-)"
echo "📊 Binary size: $(du -h $BINARY_PATH | cut -f1)"
echo

echo "🔧 New Features in this build:"
echo "   • Full e-ink display support using epd4in2b_v2 module"
echo "   • Three-color display (Black/White/Red)"
echo "   • Configurable GPIO pins (CS, DC, RST, BUSY)"
echo "   • Improved error handling and diagnostics"
echo "   • Following Waveshare Python examples pattern"
echo

echo "📋 Configuration ready:"
echo "   • CS pin: GPIO 7 (Pin 26) - Your custom pin"
echo "   • DC pin: GPIO 25 (Pin 22) - Default"
echo "   • RST pin: GPIO 27 (Pin 13) - Your custom pin"
echo "   • BUSY pin: GPIO 24 (Pin 18) - Default"
echo

echo "🚀 To deploy to Pi:"
echo "1. Copy binary:"
echo "   scp $BINARY_PATH pi@your-pi-ip:/home/pi/"
echo
echo "2. Test on Pi:"
echo "   ssh pi@your-pi-ip"
echo "   sudo ./tide-tracker"
echo
echo "3. Expected output:"
echo "   🔧 E-ink hardware integration with configurable GPIO pins"
echo "   📋 GPIO pin configuration: [your pins]"
echo "   🚀 Initializing SPI and GPIO for e-ink display..."
echo "   🎨 Creating e-ink display driver (4.2\" b/w/red v2)..."
echo "   📊 Rendering tide data..."
echo "   📡 Updating e-ink display..."
echo "   ✅ E-ink display updated successfully"
echo

echo "🔍 If issues occur:"
echo "   • Check the detailed error messages for specific problems"
echo "   • Ensure SPI is properly configured (enabled or disabled as needed)"
echo "   • Verify GPIO pin connections match your configuration"
echo "   • Run with 'sudo' for GPIO access permissions"
echo

echo "Ready for deployment! 🎉"

#!/bin/bash
# Build script for Raspberry Pi hardware support
# Run this on the Pi to build the project with hardware features

set -e

echo "🛠️  Building tide-tracker with hardware support..."
echo

# Check if we're on the right platform
if [ "$(uname -m)" != "aarch64" ]; then
    echo "⚠️  Warning: This doesn't appear to be an ARM64 system"
    echo "   Current architecture: $(uname -m)"
    echo "   Expected: aarch64"
    echo
fi

# Check for Rust
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Cargo not found. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
fi

echo "📦 Rust toolchain: $(cargo --version)"
echo

# Build with hardware features
echo "🔨 Building with hardware features..."
if cargo build --release --features hardware; then
    echo "✅ Build successful!"
    echo
    echo "📁 Binary location: ./target/release/tide-tracker"
    echo "📊 Binary size: $(du -h target/release/tide-tracker | cut -f1)"
    echo
    echo "🚀 To run:"
    echo "   sudo ./target/release/tide-tracker"
    echo
    echo "🔍 To debug hardware issues:"
    echo "   sudo ./target/release/tide-tracker 2>&1 | head -20"
else
    echo "❌ Build failed. Check the error messages above."
    exit 1
fi

#!/usr/bin/env bash
set -euxo pipefail

# WiFi Connect v4.x installation script for Raspberry Pi OS 64-bit (Bookworm)
# Downloads, installs, and configures WiFi Connect captive portal

WIFI_CONNECT_VERSION="${WIFI_CONNECT_VERSION:-4.11.84}"

# Check if already installed
if [ -x "/usr/local/sbin/wifi-connect" ] && [ -d "/usr/share/wifi-connect/ui" ]; then
    echo "WiFi Connect already installed, skipping..."
    exit 0
fi

# Install required packages
echo "Installing NetworkManager and dependencies..."
apt-get update
apt-get install -y --no-install-recommends network-manager curl

# Disable dhcpcd to avoid conflicts with NetworkManager
echo "Disabling dhcpcd service..."
systemctl disable --now dhcpcd.service || true

# Enable NetworkManager
echo "Enabling NetworkManager..."
systemctl enable --now NetworkManager.service

# Detect architecture and set asset name
ARCH=$(dpkg --print-architecture)
case "$ARCH" in
    arm64|aarch64)
        ASSET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

echo "Detected architecture: $ARCH, using asset: $ASSET"

DOWNLOAD_URL="https://github.com/balena-os/wifi-connect/releases/download/v${WIFI_CONNECT_VERSION}/wifi-connect-${ASSET}.tar.gz"
TEMP_DIR=$(mktemp -d)

echo "Downloading WiFi Connect v${WIFI_CONNECT_VERSION}..."
curl -L -o "${TEMP_DIR}/wifi-connect.tar.gz" "$DOWNLOAD_URL"

# Extract and install binary
echo "Installing WiFi Connect binary..."
cd "$TEMP_DIR"
tar -xzf wifi-connect.tar.gz
mkdir -p /usr/local/sbin
cp wifi-connect /usr/local/sbin/wifi-connect
chmod +x /usr/local/sbin/wifi-connect

# Install UI assets
echo "Installing WiFi Connect UI assets..."
mkdir -p /usr/share/wifi-connect
if [ -d "ui" ]; then
    cp -r ui /usr/share/wifi-connect/
else
    echo "Warning: UI directory not found in release archive"
fi

# Cleanup
rm -rf "$TEMP_DIR"

# Install wifi-connect-loop.sh script
echo "Installing wifi-connect-loop.sh script..."
mkdir -p /usr/local/bin
cp "$(dirname "$0")/wifi-connect-loop.sh" /usr/local/bin/wifi-connect-loop.sh
chmod +x /usr/local/bin/wifi-connect-loop.sh

# Install systemd service
echo "Installing systemd service..."
cp "$(dirname "$0")/systemd/wifi-connect.service" /etc/systemd/system/wifi-connect.service
systemctl daemon-reload
systemctl enable wifi-connect.service

echo "WiFi Connect v${WIFI_CONNECT_VERSION} installation complete!"
echo "Service will start automatically on boot or can be started with: systemctl start wifi-connect.service"

#!/usr/bin/env bash
set -euo pipefail

# Download WiFi Connect binary and UI assets on first boot
WIFI_VER="v4.11.84"
WIFI_BINARY="/usr/local/bin/wifi-connect"
WIFI_UI_DIR="/usr/share/wifi-connect/ui"

if [[ ! -f "$WIFI_BINARY" ]]; then
    echo "Downloading WiFi Connect ${WIFI_VER}..."
    TMPDIR="$(mktemp -d)"
    curl -L \
        "https://github.com/balena-os/wifi-connect/releases/download/${WIFI_VER}/wifi-connect-aarch64-unknown-linux-gnu.tar.gz" \
        | tar -xz -C "$TMPDIR"
    install -Dm755 "$TMPDIR/wifi-connect" "$WIFI_BINARY"
    rm -rf "$TMPDIR"
    echo "WiFi Connect installed successfully"
fi

# Download Tide Tracker binary on first boot
if [[ ! -f "/usr/local/bin/tide-tracker" ]]; then
    echo "Downloading Tide Tracker binary..."
    /usr/local/bin/tide-get-binary.sh
    mv tide-tracker /usr/local/bin/tide-tracker
    chmod +x /usr/local/bin/tide-tracker
    echo "Tide Tracker installed successfully"
fi

# Enable services
systemctl enable tide-tracker.service tide-tracker.timer wifi-connect.service

echo "First boot setup completed"

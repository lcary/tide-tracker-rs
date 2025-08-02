#!/usr/bin/env bash
set -euxo pipefail

# WiFi Connect loop script - checks internet connectivity and starts captive portal if needed

# Test internet connectivity
echo "Checking internet connectivity..."
if ping -c 3 -W 5 1.1.1.1 >/dev/null 2>&1; then
    echo "Internet connection available, WiFi Connect not needed"
    exit 0
fi

echo "No internet connection detected, starting WiFi Connect captive portal..."

# Start WiFi Connect with custom portal settings
exec /usr/local/sbin/wifi-connect \
    --portal-ssid "TideTracker-Setup" \
    --portal-passphrase "pi-tides" \
    --timeout 30 \
    --activity-timeout 600

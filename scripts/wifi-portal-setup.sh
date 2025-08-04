#!/usr/bin/env bash
set -euxo pipefail

# Minimal Captive Portal Wi-Fi Setup Installation Script
# Sets up NetworkManager-based captive portal for Raspberry Pi OS Bookworm

echo "Installing minimal captive portal Wi-Fi setup..."

# Check if already installed
if [ -x "/usr/local/sbin/wifi-portal.sh" ] && [ -f "/etc/systemd/system/wifi-portal.service" ]; then
    echo "Wi-Fi portal already installed, skipping..."
    exit 0
fi

# Install required packages
echo "Installing NetworkManager and dependencies..."
apt-get update
apt-get install -y --no-install-recommends network-manager busybox

# Disable dhcpcd to avoid conflicts with NetworkManager
echo "Disabling dhcpcd service..."
systemctl disable --now dhcpcd.service || true

# Enable NetworkManager
echo "Enabling NetworkManager..."
systemctl enable --now NetworkManager.service

# Get script directory
SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"

# Install the main portal script
echo "Installing wifi-portal.sh script..."
cp "${SCRIPT_DIR}/wifi-portal.sh" /usr/local/sbin/wifi-portal.sh
chmod +x /usr/local/sbin/wifi-portal.sh

# Install web content
echo "Installing portal web content..."
mkdir -p /usr/local/share/tideportal/cgi-bin
cp "${SCRIPT_DIR}/portal-web/index.html" /usr/local/share/tideportal/
cp "${SCRIPT_DIR}/portal-web/cgi-bin/wifi-connect.sh" /usr/local/share/tideportal/cgi-bin/
chmod +x /usr/local/share/tideportal/cgi-bin/wifi-connect.sh

# Install NetworkManager configuration for DNS interception
echo "Configuring NetworkManager for captive portal..."
mkdir -p /etc/NetworkManager/conf.d
mkdir -p /etc/NetworkManager/dnsmasq-shared.d
cp "${SCRIPT_DIR}/nm-config/dns.conf" /etc/NetworkManager/conf.d/
cp "${SCRIPT_DIR}/nm-config/captive.conf" /etc/NetworkManager/dnsmasq-shared.d/

# Install systemd service
echo "Installing systemd service..."
cp "${SCRIPT_DIR}/systemd/wifi-portal.service" /etc/systemd/system/wifi-portal.service
systemctl daemon-reload
systemctl enable wifi-portal.service

# Restart NetworkManager to apply DNS config
echo "Restarting NetworkManager to apply configuration..."
systemctl restart NetworkManager.service

echo "Minimal captive portal Wi-Fi setup installation complete!"
echo "Service will start automatically on boot when no Wi-Fi is connected."
echo "Setup network: TideTracker-Setup (password: pi-tides)"

#!/usr/bin/env bash
set -euxo pipefail

# Wi-Fi Portal Update Script
# Updates portal scripts and restarts the service

echo "Updating Wi-Fi portal scripts..."

# Get script directory
SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"

# Check if source files exist
if [ ! -f "${SCRIPT_DIR}/wifi-portal.sh" ]; then
    echo "Error: wifi-portal.sh not found in ${SCRIPT_DIR}"
    exit 1
fi

if [ ! -f "${SCRIPT_DIR}/portal-web/cgi-bin/wifi-connect.sh" ]; then
    echo "Error: CGI script not found in ${SCRIPT_DIR}"
    exit 1
fi

# Stop the service
echo "Stopping wifi-portal service..."
systemctl stop wifi-portal.service || true

# Update the main portal script
echo "Updating wifi-portal.sh..."
cp "${SCRIPT_DIR}/wifi-portal.sh" /usr/local/sbin/wifi-portal.sh
chmod +x /usr/local/sbin/wifi-portal.sh

# Update the CGI script
echo "Updating CGI script..."
cp "${SCRIPT_DIR}/portal-web/cgi-bin/wifi-connect.sh" /usr/local/share/tideportal/cgi-bin/wifi-connect.sh
chmod +x /usr/local/share/tideportal/cgi-bin/wifi-connect.sh

# Update the HTML page
echo "Updating portal page..."
cp "${SCRIPT_DIR}/portal-web/index.html" /usr/local/share/tideportal/index.html

# Restart the service
echo "Starting wifi-portal service..."
systemctl start wifi-portal.service

# Show status
echo "Service status:"
systemctl status wifi-portal.service --no-pager

echo "Wi-Fi portal update completed successfully!"
echo "Monitor with: journalctl -u wifi-portal.service -f"

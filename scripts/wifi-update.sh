#!/usr/bin/env bash
set -euxo pipefail

# WiFi Connect loop script updater
# Updates only the wifi-connect-loop.sh script and restarts the service

echo "Updating WiFi Connect loop script..."

# Get script directory
SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"

# Check if source file exists
if [ ! -f "${SCRIPT_DIR}/wifi-connect-loop.sh" ]; then
    echo "Error: wifi-connect-loop.sh not found in ${SCRIPT_DIR}"
    exit 1
fi

# Stop the service
echo "Stopping wifi-connect service..."
systemctl stop wifi-connect.service

# Update the loop script
echo "Updating wifi-connect-loop.sh..."
cp "${SCRIPT_DIR}/wifi-connect-loop.sh" /usr/local/bin/wifi-connect-loop.sh
chmod +x /usr/local/bin/wifi-connect-loop.sh

# Restart the service
echo "Starting wifi-connect service..."
systemctl start wifi-connect.service

# Show status
echo "Service status:"
systemctl status wifi-connect.service --no-pager

echo "WiFi Connect loop script updated successfully!"
echo "Monitor with: journalctl -u wifi-connect.service -f"

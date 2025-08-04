#!/usr/bin/env bash
# wifi-portal.sh: Manage captive portal AP and Wi-Fi configuration
set -euo pipefail

# Configuration
AP_SSID="TideTracker-Setup"
AP_PASS="pi-tides"                # WPA2 passphrase for the setup AP
AP_IFACE="wlan0"
AP_CON_NAME="tide-setup"          # Name for the NM connection profile for AP
AP_IP="192.168.4.1"
AP_NETMASK="/24"                  # CIDR netmask for AP IP
# The web content directory and CGI script:
WEB_ROOT="/usr/local/share/tideportal"
CGI_SCRIPT="$WEB_ROOT/cgi-bin/wifi-connect.sh"

# Wait for any existing Wi-Fi connection
echo "[wifi-portal] Waiting up to 30s for existing Wi-Fi..."
for i in {1..30}; do
    # Check if wlan0 is connected to Wi-Fi (state == connected and not AP mode)
    # nmcli dev shows state "connected" for active connections.
    WIFI_STATE=$(nmcli -t -f DEVICE,STATE,CONNECTION dev | grep "^$AP_IFACE:" || true)
    # Example nmcli output: "wlan0:connected:HomeWiFi" or "wlan0:disconnected:"
    if [[ "$WIFI_STATE" == *":connected:"* && "$WIFI_STATE" != *":ap:"* ]]; then
        echo "[wifi-portal] Wi-Fi is already connected (${WIFI_STATE##*:}). Portal not needed."
        exit 0
    fi
    sleep 1
done

echo "[wifi-portal] No Wi-Fi connection detected. Starting setup access point..."

# Create NM hotspot connection if not exists (idempotent)
if ! nmcli connection show "$AP_CON_NAME" &>/dev/null; then
    nmcli connection add type wifi ifname "$AP_IFACE" mode ap con-name "$AP_CON_NAME" ssid "$AP_SSID"
    nmcli connection modify "$AP_CON_NAME" 802-11-wireless.mode ap 802-11-wireless.band bg ipv4.method shared
    nmcli connection modify "$AP_CON_NAME" ipv4.addresses "$AP_IP$AP_NETMASK" ipv4.gateway "$AP_IP"
    nmcli connection modify "$AP_CON_NAME" 802-11-wireless-security.key-mgmt wpa-psk 802-11-wireless-security.psk "$AP_PASS"
    # Set country (reg domain) if needed, e.g.: nmcli connection modify "$AP_CON_NAME" 802-11-wireless.country US
fi

# Bring up the AP
nmcli connection up "$AP_CON_NAME" || {
    echo "[wifi-portal] ERROR: Failed to start AP mode on $AP_IFACE."
    exit 1
}
echo "[wifi-portal] Hotspot '$AP_SSID' activated on $AP_IFACE ($AP_IP)."

# Start the captive portal HTTP server (BusyBox httpd in foreground)
/bin/busybox httpd -f -p 80 -h "$WEB_ROOT" &
HTTPD_PID=$!
echo "[wifi-portal] Web server started (PID $HTTPD_PID)."

# Main loop: monitor for Wi-Fi credentials and connection
CONNECTED=false
while ! $CONNECTED; do
    # Check if wlan0 got a non-AP connection
    # (i.e., an active connection that is not our setup AP)
    NM_STATUS=$(nmcli -t -f DEVICE,STATE,CONNECTION dev | grep "^$AP_IFACE:")
    if [[ "$NM_STATUS" == *":connected:"* && "$NM_STATUS" != *"$AP_CON_NAME"* ]]; then
        echo "[wifi-portal] Connected to Wi-Fi network: ${NM_STATUS##*:}"
        CONNECTED=true
        break
    fi

    # If the portal AP got deactivated (e.g. due to a connect attempt) but no Wi-Fi connected, 
    # that means a connection was attempted and failed. Bring the AP back up.
    if [[ "$NM_STATUS" == *":disconnected:"* ]]; then
        echo "[wifi-portal] Wi-Fi connect attempt failed. Restarting AP..."
        nmcli connection down "$AP_CON_NAME" 2>/dev/null || true
        nmcli connection up "$AP_CON_NAME" || echo "Failed to bring AP back up."
        # (The portal webserver is likely still running, so no need to restart it)
    fi

    sleep 3
done

# If we reach here, Wi-Fi is connected.
# Cleanup: shut down portal services
echo "[wifi-portal] Stopping AP and portal services..."
nmcli connection down "$AP_CON_NAME" 2>/dev/null || true
kill $HTTPD_PID 2>/dev/null || true

# (Optional) Disable the wifi-portal service for future boots, since Wi-Fi is set:
# systemctl disable wifi-portal.service

# (Optional) Reboot to ensure device comes up on new Wi-Fi:
# echo "[wifi-portal] Rebooting to finalize setup..."; reboot

echo "[wifi-portal] Wi-Fi setup successful. Device is online."
exit 0

#!/usr/bin/env bash
echo "Content-type: text/html"
echo ""

# Read the POST data (SSID and password)
read -r -N "$CONTENT_LENGTH" POST_DATA

# Parse form fields (URL-encoded)
SSID=$(printf '%b' "${POST_DATA%%&*}" | sed -e 's/ssid=//' -e 's/+/ /g' -e 's/%/\\x/g')
PASS_RAW=$(printf '%b' "${POST_DATA#*&}" | sed -e 's/psk=//' -e 's/+/ /g' -e 's/%/\\x/g')
PSK=$(printf '%b' "$PASS_RAW")

# Launch Wi-Fi connect in background so we can immediately respond
if [ -n "$SSID" ]; then
    if [ -n "$PSK" ]; then
        nmcli device wifi connect "$SSID" password "$PSK" ifname "wlan0" &
    else
        nmcli device wifi connect "$SSID" ifname "wlan0" &
    fi
fi

# Respond with a simple HTML page
cat <<EOF
<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8"><title>Connecting...</title></head>
<body style="font-family:sans-serif;text-align:center;margin-top:2em;">
  <h2>Connecting to Wi-Fi <code>${SSID}</code>...</h2>
  <p>If the credentials are correct, your Tide Tracker will join this network.</p>
  <p><b>You may now disconnect from "TideTracker-Setup".</b></p>
  <p style="color:gray;font-size:0.9em;">
     (If the device fails to connect, this setup network will reappear for another try.)
  </p>
</body></html>
EOF

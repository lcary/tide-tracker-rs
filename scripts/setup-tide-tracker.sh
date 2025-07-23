#!/usr/bin/env bash
set -euo pipefail

### ──────────────────────────
### Argument parsing
### ──────────────────────────
BIN_SRC=""
CFG_SRC=""
ENABLE_OVERLAY=0

usage() {
  echo "Usage: sudo $0 --binary /path/to/tide-tracker --config /path/to/tide-config.toml [--enable-overlay]"
  echo "Consider running 'bash scripts/get-binary.sh' to download the latest release binary."
  echo "See also the sample tide-config.toml configuration at the repo root."
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)  BIN_SRC="$2"; shift 2 ;;
    --config)  CFG_SRC="$2"; shift 2 ;;
    --enable-overlay) ENABLE_OVERLAY=1; shift ;;
    *) usage ;;
  esac
done

[[ -n "$BIN_SRC" && -n "$CFG_SRC" ]] || usage
[[ -f "$BIN_SRC" ]] || { echo "Binary not found: $BIN_SRC"; exit 2; }
[[ -f "$CFG_SRC" ]] || { echo "Config not found: $CFG_SRC"; exit 2; }

### ──────────────────────────
### Constants / paths
### ──────────────────────────
BIN_DEST="/usr/local/bin/tide-tracker"
CFG_DIR="/etc/tide-tracker"
CFG_DEST="$CFG_DIR/tide-config.toml"
SERVICE_FILE="/etc/systemd/system/tide-tracker.service"
TIMER_FILE="/etc/systemd/system/tide-tracker.timer"
FULL_REFRESH_SVC="/etc/systemd/system/tide-midnight-refresh.service"
FULL_REFRESH_TIMER="/etc/systemd/system/tide-midnight-refresh.timer"

# Detect the user who should run the service (not root)
if [[ "$SUDO_USER" ]]; then
    SERVICE_USER="$SUDO_USER"
else
    SERVICE_USER="$(logname 2>/dev/null || echo pi)"
fi

echo "Installing tide-tracker to run as user: $SERVICE_USER"

### ──────────────────────────
### 1. Install binary & config
### ──────────────────────────
install -Dm755 "$BIN_SRC" "$BIN_DEST"
install -Dm644 "$CFG_SRC" "$CFG_DEST"

### ──────────────────────────
### 2. systemd service & timer (every 10 minutes)
### ──────────────────────────
# Always recreate service file to ensure proper configuration
cat <<EOF >"$SERVICE_FILE"
[Unit]
Description=Tide Tracker e-paper display update
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=$BIN_DEST
WorkingDirectory=$CFG_DIR
Environment=RUST_LOG=info
Environment=RUST_BACKTRACE=1
User=$SERVICE_USER
Group=$SERVICE_USER
StandardOutput=journal
StandardError=journal
SyslogIdentifier=tide-tracker

# Allow GPIO access
SupplementaryGroups=gpio spi

# Security (but allow GPIO access)
NoNewPrivileges=false
ProtectSystem=false
ProtectHome=true
PrivateTmp=true
EOF

# Always recreate timer file to ensure proper configuration
cat <<EOF >"$TIMER_FILE"
[Unit]
Description=Update tide tracker every 10 minutes
Requires=tide-tracker.service

[Timer]
OnBootSec=2min
OnUnitActiveSec=10min
AccuracySec=1min

[Install]
WantedBy=timers.target
EOF

# Reload and enable
systemctl daemon-reload
systemctl enable tide-tracker.service
systemctl enable tide-tracker.timer

### ──────────────────────────
### 3. Midnight full-refresh timer
### ──────────────────────────
if [[ ! -f "$FULL_REFRESH_SVC" ]]; then
cat <<EOF >"$FULL_REFRESH_SVC"
[Unit]
Description=Full e-paper refresh at midnight

[Service]
Type=oneshot
ExecStart=$BIN_DEST --full-refresh
WorkingDirectory=$CFG_DIR
User=$SERVICE_USER
EOF
fi

if [[ ! -f "$FULL_REFRESH_TIMER" ]]; then
cat <<EOF >"$FULL_REFRESH_TIMER"
[Unit]
Description=Timer that triggers midnight e-paper refresh

[Timer]
OnCalendar=*-*-* 00:00
Persistent=true
Unit=$(basename "$FULL_REFRESH_SVC")

[Install]
WantedBy=timers.target
EOF
  systemctl daemon-reload
  systemctl enable tide-midnight-refresh.timer
fi

### ──────────────────────────
### 4. Hardware watchdog
### ──────────────────────────
if ! grep -q '^dtparam=watchdog=on' /boot/config.txt; then
  echo 'Enabling BCM watchdog…'
  echo 'dtparam=watchdog=on' >> /boot/config.txt
fi

if ! dpkg -s watchdog &>/dev/null; then
  apt-get update
  apt-get install -y watchdog
fi
systemctl enable watchdog.service

### ──────────────────────────
### 5. Optional read-only overlay
### ──────────────────────────
if [[ "$ENABLE_OVERLAY" -eq 1 ]]; then
  if ! raspi-config nonint get_overlayfs | grep -q 1; then
    echo 'Activating read-only overlay filesystem…'
    raspi-config nonint enable_overlayfs
  else
    echo 'Overlay FS already enabled, skipping.'
  fi
fi

### ──────────────────────────
### 6. Finish
### ──────────────────────────
echo '✅  Tide Tracker system services installed.'
echo '   → Reboot now? (y/N)'
read -r reply
[[ "$reply" =~ ^[Yy]$ ]] && reboot

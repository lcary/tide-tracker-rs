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
FULL_REFRESH_SVC="/etc/systemd/system/tide-midnight-refresh.service"
FULL_REFRESH_TIMER="/etc/systemd/system/tide-midnight-refresh.timer"

### ──────────────────────────
### 1. Install binary & config
### ──────────────────────────
install -Dm755 "$BIN_SRC" "$BIN_DEST"
install -Dm644 "$CFG_SRC" "$CFG_DEST"

### ──────────────────────────
### 2. systemd service (boot & self-heal)
### ──────────────────────────
if [[ ! -f "$SERVICE_FILE" ]]; then
cat <<EOF >"$SERVICE_FILE"
[Unit]
Description=Tide Tracker e-paper display
After=network-online.target

[Service]
ExecStart=$BIN_DEST
WorkingDirectory=$CFG_DIR
Environment=RUST_LOG=info
Restart=always
RestartSec=5
User=pi

[Install]
WantedBy=multi-user.target
EOF
  systemctl daemon-reload
  systemctl enable tide-tracker.service
fi

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
User=pi
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

#!/usr/bin/env bash
set -euo pipefail

### ──────────────────────────
### Argument parsing
### ──────────────────────────
BIN_SRC=""
CFG_SRC=""
ENABLE_OVERLAY=0
UPDATE_BINARY=0
UPDATE_CONFIG=0

usage() {
  echo "Usage: sudo $0 --binary /path/to/tide-tracker --config /path/to/tide-config.toml [--enable-overlay] [--update-binary] [--update-config]"
  echo "  --update-binary   Only update the binary if it changed."
  echo "  --update-config   Only update the config if it changed."
  echo "Consider running 'bash scripts/get-binary.sh' to download the latest release binary."
  echo "See also the sample tide-config.toml configuration at the repo root."
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --binary)  BIN_SRC="$2"; shift 2 ;;
    --config)  CFG_SRC="$2"; shift 2 ;;
    --enable-overlay) ENABLE_OVERLAY=1; shift ;;
    --update-binary) UPDATE_BINARY=1; shift ;;
    --update-config) UPDATE_CONFIG=1; shift ;;
    *) usage ;;
  esac
done

[[ -n "$BIN_SRC" || $UPDATE_CONFIG -eq 1 ]] || usage
[[ -n "$CFG_SRC" || $UPDATE_BINARY -eq 1 ]] || usage

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
### 1. Install binary & config (or update)
### ──────────────────────────
if [[ $UPDATE_BINARY -eq 1 ]]; then
  if [[ -n "$BIN_SRC" && -f "$BIN_SRC" ]]; then
    echo "Updating binary..."
    install -Dm755 "$BIN_SRC" "$BIN_DEST"
    systemctl restart tide-tracker.service || true
    echo "✅ Binary updated and service restarted."
  else
    echo "Binary not found: $BIN_SRC"; exit 2
  fi
fi

if [[ $UPDATE_CONFIG -eq 1 ]]; then
  if [[ -n "$CFG_SRC" && -f "$CFG_SRC" ]]; then
    echo "Updating config..."
    install -Dm644 "$CFG_SRC" "$CFG_DEST"
    systemctl restart tide-tracker.service || true
    echo "✅ Config updated and service restarted."
  else
    echo "Config not found: $CFG_SRC"; exit 2
  fi
fi

if [[ $UPDATE_BINARY -eq 0 && $UPDATE_CONFIG -eq 0 ]]; then
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
# This board (Pi Zero 2 W) has no RTC (no /dev/rtc*); at boot the clock is
# only a restored fake-hwclock/timesyncd guess until NTP corrects it. The
# tide chart is centred entirely on "now", so rendering before the clock is
# right would draw the wrong time window. Wait for time-sync.target so the
# clock is actually correct before the first run.
After=time-sync.target
Wants=time-sync.target

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

# Durable, per-service storage for the tide prediction cache. systemd
# creates /var/lib/tide-tracker (owned by $SERVICE_USER) before each start
# and exports its path via \$STATE_DIRECTORY, which is where
# tide_data::fetch() persists cached predictions. This directory survives
# across invocations even with PrivateTmp=true below, unlike /tmp.
# (\$STATE_DIRECTORY is escaped: this heredoc is unquoted so that
# \$SERVICE_USER and \$BIN_DEST expand, and set -u would abort on an
# unexpanded systemd-provided variable.)
StateDirectory=tide-tracker

# Security (but allow GPIO access)
NoNewPrivileges=false
ProtectSystem=false
ProtectHome=true
# Safe to keep: the prediction cache now lives under \$STATE_DIRECTORY
# (/var/lib/tide-tracker), not /tmp, so a fresh private /tmp per oneshot
# invocation no longer defeats caching.
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
  # Always recreate these two, like tide-tracker.service above. They were
  # previously only written when absent, which meant any later fix to them
  # (such as the time-sync dependency below) silently never reached an
  # existing install.
cat <<EOF >"$FULL_REFRESH_SVC"
[Unit]
Description=Full e-paper refresh at midnight
# No RTC on this board; wait for the clock to be NTP-corrected before
# drawing, since the chart is centred on the current time (see
# tide-tracker.service for the full explanation).
After=time-sync.target
Wants=time-sync.target

[Service]
Type=oneshot
ExecStart=$BIN_DEST --full-refresh
WorkingDirectory=$CFG_DIR
User=$SERVICE_USER
EOF

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

  ### ──────────────────────────
  ### 4. Hardware watchdog
  ### ──────────────────────────
  # Raspberry Pi OS Bookworm moved the boot config to /boot/firmware/config.txt;
  # /boot/config.txt no longer exists there (older releases still use it).
  # Prefer the Bookworm path, fall back to the legacy path, and skip this
  # step entirely rather than creating a stray file if neither is present.
  if [[ -f /boot/firmware/config.txt ]]; then
    BOOT_CONFIG=/boot/firmware/config.txt
  elif [[ -f /boot/config.txt ]]; then
    BOOT_CONFIG=/boot/config.txt
  else
    BOOT_CONFIG=""
  fi

  if [[ -n "$BOOT_CONFIG" ]]; then
    if ! grep -q '^dtparam=watchdog=on' "$BOOT_CONFIG"; then
      echo 'Enabling BCM watchdog…'
      echo 'dtparam=watchdog=on' >> "$BOOT_CONFIG"
    fi
  else
    echo 'Warning: could not find /boot/firmware/config.txt or /boot/config.txt - skipping watchdog dtparam setup.'
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
  # Only prompt interactively when stdin is a real TTY; otherwise (e.g. when
  # this script is piped in non-interactively, or run from another script)
  # reading from stdin would hang or immediately fail, making the installer
  # unusable. Non-interactive runs just get told a reboot may be needed and
  # exit successfully rather than being blocked.
  if [[ -t 0 ]]; then
    echo '   → Reboot now? (y/N)'
    read -r reply
    [[ "$reply" =~ ^[Yy]$ ]] && reboot
  else
    echo '   → Non-interactive session detected: skipping reboot prompt.'
    echo '   → A reboot may be needed for the watchdog/overlay changes to take effect.'
  fi
fi

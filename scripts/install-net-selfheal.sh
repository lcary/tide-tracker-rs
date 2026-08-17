#!/usr/bin/env bash
set -euo pipefail

# Installs the escalating network self-heal service and its timer.
#
# Why this exists: the Pi Zero 2 W's brcmfmac Wi-Fi occasionally wedges badly
# enough that the display sits on stale data showing OFFLINE until someone
# physically power-cycles the box. This service walks a ladder of increasingly
# aggressive recovery steps so it fixes itself instead.
#
# Safe to re-run.

### ──────────────────────────
### Require root
### ──────────────────────────
if [[ "$(id -u)" -ne 0 ]]; then
    echo "This script must be run as root. Re-running with sudo..."
    exec sudo -E bash "$0" "$@"
fi

SCRIPT_DIR="$(realpath "$(dirname "${BASH_SOURCE[0]}")")"

### ──────────────────────────
### 1. Install the recovery script
### ──────────────────────────
echo "Installing net-selfheal.sh to /usr/local/bin..."
install -Dm755 "$SCRIPT_DIR/net-selfheal.sh" /usr/local/bin/net-selfheal.sh

### ──────────────────────────
### 2. Install systemd units
### ──────────────────────────
echo "Installing systemd service and timer..."
install -Dm644 "$SCRIPT_DIR/systemd/net-selfheal.service" /etc/systemd/system/net-selfheal.service
install -Dm644 "$SCRIPT_DIR/systemd/net-selfheal.timer" /etc/systemd/system/net-selfheal.timer

systemctl daemon-reload
systemctl enable --now net-selfheal.timer

### ──────────────────────────
### 3. Report
### ──────────────────────────
echo ""
echo "✅ net-selfheal installed."
echo "  Timer state: $(systemctl is-active net-selfheal.timer)"
echo "  Next run:"
systemctl list-timers net-selfheal.timer --no-pager | sed -n '2p' || true
echo ""
echo "  Watch it with: journalctl -u net-selfheal -f"
echo "  Force a check: sudo systemctl start net-selfheal.service"

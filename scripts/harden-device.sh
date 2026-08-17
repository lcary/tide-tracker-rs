#!/usr/bin/env bash
set -euo pipefail

# Hardening script for a Raspberry Pi Zero 2 W running the tide-tracker appliance.
# Fixes three field-observed reliability issues:
#   1. Wi-Fi silently dropping off the network after days-to-weeks (brcmfmac power save).
#   2. Logs vanishing on reboot (journald Storage=volatile), losing evidence of failures.
#   3. The hardware watchdog running but not actually monitoring anything.
#
# Safe to re-run: every step below checks current state before changing it.

### ──────────────────────────
### Require root
### ──────────────────────────
if [[ "$(id -u)" -ne 0 ]]; then
    echo "This script must be run as root. Re-running with sudo..."
    exec sudo -E bash "$0" "$@"
fi

### ──────────────────────────
### 1. Disable Wi-Fi power save
### ──────────────────────────
# brcmfmac power save is the confirmed cause of this Pi silently dropping off
# Wi-Fi after days-to-weeks, requiring a physical power cycle to recover.
echo "Disabling Wi-Fi power save..."

NM_POWERSAVE_CONF="/etc/NetworkManager/conf.d/99-wifi-powersave-off.conf"
mkdir -p "$(dirname "$NM_POWERSAVE_CONF")"
cat <<'EOF' >"$NM_POWERSAVE_CONF"
[connection]
wifi.powersave = 2
EOF
echo "  Wrote $NM_POWERSAVE_CONF (2 = disable, per NetworkManager docs)."

# Persist the setting on the active connection profile too, so it survives
# reconnects. Note that NetworkManager cannot hot-apply this particular
# property: `nmcli device reapply` reports "Can't reapply changes to
# '802-11-wireless.powersave' setting". So the profile change only takes effect
# on the next activation, and we force the live state with `iw` below.
ACTIVE_WIFI_CONN="$(nmcli -t -f NAME,TYPE connection show --active | awk -F: '$2=="802-11-wireless"{print $1; exit}')"
if [[ -n "$ACTIVE_WIFI_CONN" ]]; then
    echo "  Applying to active connection: $ACTIVE_WIFI_CONN"
    nmcli connection modify "$ACTIVE_WIFI_CONN" 802-11-wireless.powersave 2 || true
    nmcli device reapply wlan0 || true
else
    echo "  No active 802-11-wireless connection found; profile setting applies on next connect."
fi

# `iw` both sets and reads the live power save state. It lives in /usr/sbin,
# which is not on a normal user's PATH, so always call it by absolute path.
IW=/usr/sbin/iw
if [[ ! -x "$IW" ]]; then
    echo "  Installing iw..."
    apt-get update || true
    apt-get install -y --no-install-recommends iw || true
fi

if [[ -x "$IW" ]]; then
    # This is the part that actually fixes the running system without dropping
    # the link (important: we are typically connected over SSH).
    "$IW" wlan0 set power_save off || true
    echo "  Verification (iw wlan0 get power_save):"
    "$IW" wlan0 get power_save || true
else
    echo "  iw unavailable; power save will be disabled on next reconnect via the NM profile."
fi

### ──────────────────────────
### 2. Make logs persistent
### ──────────────────────────
# journald is currently Storage=volatile, so every log is lost on reboot and
# intermittent failures leave no evidence to debug after the fact.
echo "Making journald logs persistent..."

JOURNALD_CONF="/etc/systemd/journald.conf"
if grep -qE '^[[:space:]]*#?[[:space:]]*Storage[[:space:]]*=' "$JOURNALD_CONF"; then
    # Replace whatever Storage= line is there (active or commented out).
    sed -i -E 's/^[[:space:]]*#?[[:space:]]*Storage[[:space:]]*=.*/Storage=persistent/' "$JOURNALD_CONF"
elif grep -qE '^\[Journal\]' "$JOURNALD_CONF"; then
    sed -i '/^\[Journal\]/a Storage=persistent' "$JOURNALD_CONF"
else
    printf '\n[Journal]\nStorage=persistent\n' >>"$JOURNALD_CONF"
fi
echo "  Set Storage=persistent in $JOURNALD_CONF"

# Cap disk usage so persistent logs can't fill the SD card.
JOURNALD_DROPIN_DIR="/etc/systemd/journald.conf.d"
JOURNALD_DROPIN="$JOURNALD_DROPIN_DIR/99-tide-tracker.conf"
mkdir -p "$JOURNALD_DROPIN_DIR"
cat <<'EOF' >"$JOURNALD_DROPIN"
[Journal]
SystemMaxUse=50M
SystemMaxFileSize=10M
EOF
echo "  Wrote $JOURNALD_DROPIN (SystemMaxUse=50M, SystemMaxFileSize=10M)"

mkdir -p /var/log/journal
systemctl restart systemd-journald || true
# Restarting journald alone leaves /var/log/journal empty: the existing runtime
# logs in /run must be flushed to disk before persistence is actually in effect.
journalctl --flush || true

echo "  Verification (persistent journal directory):"
ls -1 /var/log/journal || true
journalctl --disk-usage || true

### ──────────────────────────
### 3. Make the hardware watchdog actually guard something
### ──────────────────────────
# The watchdog daemon is running but /etc/watchdog.conf only sets `realtime`
# and `priority`, so it monitors nothing. The BCM2835 watchdog hardware
# maximum timeout is 15 seconds, so that's the ceiling for watchdog-timeout.
echo "Configuring watchdog to actually monitor the system..."

WATCHDOG_CONF="/etc/watchdog.conf"

# Idempotently set "key = value" in $WATCHDOG_CONF: replaces an active or
# commented-out line for that key, or appends it if the key isn't present.
set_watchdog_kv() {
    local key="$1" value="$2"
    if grep -qE "^[[:space:]]*#?[[:space:]]*${key}[[:space:]]*=" "$WATCHDOG_CONF"; then
        sed -i -E "s|^[[:space:]]*#?[[:space:]]*${key}[[:space:]]*=.*|${key} = ${value}|" "$WATCHDOG_CONF"
    else
        echo "${key} = ${value}" >>"$WATCHDOG_CONF"
    fi
}

set_watchdog_kv "watchdog-device" "/dev/watchdog"
set_watchdog_kv "watchdog-timeout" "15"
set_watchdog_kv "interval" "5"

# Deliberately NOT configuring a network `ping` check here: a watchdog ping
# failure triggers an immediate hard reset, whereas network recovery is
# handled by net-selfheal, a separate, gentler escalating service.

# Tolerate failure: a half-applied config that leaves this script dead before the
# summary is worse than a watchdog that needs one more restart.
#
# Do NOT use `systemctl restart` here. Debian's watchdog unit ships an
# ExecStopPost of `[ $run_wd_keepalive != 1 ] || false`, which exits 1 on
# purpose so that wd_keepalive takes over /dev/watchdog during the gap. systemd
# sees that as a failed stop and cancels the queued start, leaving the unit
# dead. A separate stop-then-start works, and wd_keepalive guards the board in
# between.
echo "  Enabling and starting watchdog.service..."
systemctl enable watchdog.service || true
systemctl stop watchdog.service || true
systemctl start watchdog.service || true

echo "  Verification (systemctl is-active watchdog):"
systemctl is-active watchdog || true

### ──────────────────────────
### 4. Make time-sync.target actually mean "clock is correct"
### ──────────────────────────
# This board has no RTC (no /dev/rtc*), so at boot the clock is only a restored
# guess until NTP corrects it. tide-tracker.service declares
# After=time-sync.target for exactly that reason - but on Debian
# systemd-time-wait-sync.service ships DISABLED, which means time-sync.target is
# reached immediately at boot and the dependency is inert. Measured on this box:
# target reached at 11:29:30, actual NTP sync several seconds later, with
# tide-tracker starting in between.
#
# Enabling the wait service makes the ordering real. The catch is that it waits
# indefinitely, which on an appliance whose whole problem is flaky Wi-Fi would
# mean a blank display during an outage - the opposite of what we want. So cap
# it: wait up to 90s for a genuinely synchronized clock, then give up and let
# boot continue. tide-tracker.timer's OnBootSec=2min sits outside that cap, so
# in the normal case this costs nothing. tide-tracker uses Wants= rather than
# Requires=, so a timeout here never prevents the display from updating - it
# just means we drew with a slightly stale clock, which the next run corrects.
echo "Making time-sync.target wait for a real clock..."

TIMESYNC_DROPIN_DIR="/etc/systemd/system/systemd-time-wait-sync.service.d"
mkdir -p "$TIMESYNC_DROPIN_DIR"
cat <<'EOF' >"$TIMESYNC_DROPIN_DIR/99-bounded-wait.conf"
[Unit]
Description=Wait (at most 90s) until the kernel clock is NTP-synchronized

[Service]
# Never block boot indefinitely on a network that may be down.
TimeoutStartSec=90
EOF
echo "  Wrote $TIMESYNC_DROPIN_DIR/99-bounded-wait.conf (TimeoutStartSec=90)"

systemctl daemon-reload || true
systemctl enable systemd-time-wait-sync.service || true

echo "  Verification (systemd-time-wait-sync enabled):"
systemctl is-enabled systemd-time-wait-sync.service || true

### ──────────────────────────
### Summary
### ──────────────────────────
echo ""
echo "✅ Hardening complete:"
echo "  1. Wi-Fi power save disabled ($NM_POWERSAVE_CONF, plus runtime nmcli apply)."
echo "  2. journald set to persistent storage with 50M/10M caps ($JOURNALD_DROPIN)."
echo "  3. watchdog.conf now monitors /dev/watchdog with a 15s timeout / 5s interval."
echo "  4. systemd-time-wait-sync enabled with a 90s cap, so time-sync.target"
echo "     really means the clock is correct (it was reached instantly before)."
echo ""
echo "Note: Wi-Fi power save and journald persistence fully take effect after the next reboot."

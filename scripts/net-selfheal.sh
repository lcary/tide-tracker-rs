#!/usr/bin/env bash
set -euo pipefail

# Escalating network self-heal for the tide-tracker Pi Zero 2 W, run every 5
# minutes by net-selfheal.timer.
#
# Why this exists: the brcmfmac Wi-Fi driver on this board occasionally wedges
# hard enough that only a physical power cycle recovers it — a warm reboot
# does not reset the SDIO Wi-Fi chip. Rather than jump straight to a reboot
# (which may not even fix it) or wait for a human to notice the display is
# stuck on stale/OFFLINE data, this script climbs a ladder of increasingly
# aggressive recovery steps, one rung per consecutive failed check, and only
# reboots as an absolute last resort.
#
# Must run as root (NetworkManager control and modprobe both require it).

### ──────────────────────────
### Require root
### ──────────────────────────
if [[ "$(id -u)" -ne 0 ]]; then
    echo "This script must be run as root. Re-running with sudo..."
    exec sudo -E bash "$0" "$@"
fi

### ──────────────────────────
### Constants / state
### ──────────────────────────
WIFI_IFACE="wlan0"

# /run is tmpfs, so this resets on every boot — which is exactly what we want:
# a fresh boot should start the ladder over from rung zero, not remember
# yesterday's failures.
STATE_DIR="/run/tide-tracker"
STATE_FILE="$STATE_DIR/net-failures"
mkdir -p "$STATE_DIR"

# The real upstream the app actually depends on. Used as the last, most
# meaningful check: pings can succeed while NOAA (or DNS, or TLS) is broken.
NOAA_URL="https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?product=predictions&station=8418150&date=today&datum=MLLW&time_zone=lst_ldt&units=english&format=json"

### ──────────────────────────
### Connectivity check
### ──────────────────────────
# UP if ANY of these succeed, checked cheapest-first so a healthy box exits
# fast without ever hitting the network. The NOAA call is quoted so the shell
# doesn't treat the unescaped `&` in the query string as a background job.
check_connectivity() {
    ping -c 2 -W 5 1.1.1.1 &>/dev/null && return 0
    ping -c 2 -W 5 8.8.8.8 &>/dev/null && return 0
    curl -sf --max-time 15 -o /dev/null "$NOAA_URL" && return 0
    return 1
}

### ──────────────────────────
### Recovery rungs (gentlest to harshest)
### ──────────────────────────

# Rung 1: nudge the interface without touching anything system-wide.
rung_reconnect() {
    echo "  -> disconnecting/reconnecting $WIFI_IFACE via nmcli..."
    nmcli device disconnect "$WIFI_IFACE" || true
    sleep 2
    nmcli device connect "$WIFI_IFACE" || true

    # Fallback: also bring up the active (or first available) Wi-Fi profile by
    # name, in case `device connect` didn't fully reactivate it.
    local conn
    conn="$(nmcli -t -f NAME,TYPE connection show --active 2>/dev/null | awk -F: '$2=="802-11-wireless"{print $1; exit}')" || true
    if [[ -z "$conn" ]]; then
        conn="$(nmcli -t -f NAME,TYPE connection show 2>/dev/null | awk -F: '$2=="802-11-wireless"{print $1; exit}')" || true
    fi
    if [[ -n "$conn" ]]; then
        echo "  -> also bringing up connection profile: $conn"
        nmcli connection up "$conn" || true
    fi
}

# Rung 2: NetworkManager itself may be in a bad state (stuck DHCP lease,
# stale supplicant handle, etc.); restarting it is still far short of a reboot.
rung_restart_nm() {
    echo "  -> restarting NetworkManager..."
    systemctl restart NetworkManager || true
}

# Rung 3/4: reload the brcmfmac kernel module. This forces the driver to
# re-download firmware to the SDIO Wi-Fi chip on next probe, which is the
# software equivalent of physically power-cycling the board. This is why it
# sits above plain NetworkManager restarts on the ladder but below reboot.
rung_reload_driver() {
    echo "  -> reloading brcmfmac Wi-Fi driver (firmware re-download, power-cycle equivalent)..."
    # Unload order matters: brcmfmac_wcc and brcmutil depend on / are used by
    # brcmfmac, so they must come off first. Not every kernel has
    # brcmfmac_wcc loaded (or even built), so tolerate it being absent.
    modprobe -r brcmfmac_wcc || true
    modprobe -r brcmfmac || true
    modprobe -r brcmutil || true
    sleep 3
    modprobe brcmfmac || true

    # Re-probing the chip means re-downloading firmware over SDIO, so the
    # interface does not exist again immediately. Restarting NetworkManager
    # before it reappears makes the rung fail for no reason, so wait for it
    # (bounded — if it never returns, the next rung handles it).
    local waited=0
    while [[ ! -e "/sys/class/net/$WIFI_IFACE" ]] && [[ "$waited" -lt 30 ]]; do
        sleep 2
        waited=$((waited + 2))
    done
    if [[ -e "/sys/class/net/$WIFI_IFACE" ]]; then
        echo "  -> $WIFI_IFACE reappeared after ${waited}s"
    else
        echo "  -> WARNING: $WIFI_IFACE did not reappear within ${waited}s"
    fi

    # NetworkManager needs a kick after the driver comes back so it notices
    # the (re-created) interface and re-activates the saved profile on it.
    echo "  -> restarting NetworkManager so it re-activates the profile..."
    systemctl restart NetworkManager || true
}

# Rung 5: last resort. Deliberately LAST, not first — a warm reboot does NOT
# power-cycle the SDIO Wi-Fi chip, so it does not reliably fix this failure
# mode the way the brcmfmac driver reload (rungs 3/4) does. By the time we
# get here, the driver reload has already been tried twice and failed, so
# something broader than Wi-Fi may be wrong and a full reboot is warranted.
rung_reboot() {
    echo "  -> !!! all gentler recovery attempts failed; rebooting as a last resort !!!"
    systemctl reboot
}

### ──────────────────────────
### Main
### ──────────────────────────
count="$(cat "$STATE_FILE" 2>/dev/null || echo 0)"
if ! [[ "$count" =~ ^[0-9]+$ ]]; then
    # Missing or malformed state (e.g. first boot, or a truncated write) —
    # treat as "no prior failures" rather than erroring out.
    count=0
fi

if check_connectivity; then
    if [[ "$count" -gt 0 ]]; then
        echo "Connectivity recovered after $count failed check(s)."
    fi
    echo 0 >"$STATE_FILE"
    exit 0
fi

# Network is down. Persist the incremented counter BEFORE attempting any
# recovery step below: a rung that kills the script (or the network, or the
# box) must not also lose the ladder's progress, or we'd retry the same weak
# rung forever instead of escalating.
count=$((count + 1))
echo "$count" >"$STATE_FILE"

case "$count" in
    1|2)
        echo "transient failure ($count/3), taking no action yet"
        ;;
    3)
        echo "rung 1/5 (failure $count): reconnecting $WIFI_IFACE"
        rung_reconnect
        ;;
    4)
        echo "rung 2/5 (failure $count): restarting NetworkManager"
        rung_restart_nm
        ;;
    5)
        echo "rung 3/5 (failure $count): reloading Wi-Fi driver (attempt 1 of 2)"
        rung_reload_driver
        ;;
    6)
        echo "rung 4/5 (failure $count): reloading Wi-Fi driver (attempt 2 of 2)"
        rung_reload_driver
        ;;
    *)
        echo "rung 5/5 (failure $count): all gentler recovery exhausted, rebooting"
        rung_reboot
        ;;
esac

exit 0

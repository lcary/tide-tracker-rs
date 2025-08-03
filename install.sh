#!/usr/bin/env bash
# Tide-Tracker one-shot installer
set -euo pipefail

REPO_URL="https://github.com/lcary/tide-tracker-rs.git"
INSTALL_DIR="${INSTALL_DIR:-$HOME/tide-tracker-rs}"
CFG_FILE="${INSTALL_DIR}/tide-config.toml"
BIN_PATH="${INSTALL_DIR}/tide-tracker"

log() { printf "\e[32m➜ %s\e[0m\n" "$*"; }

# ---------------------------------------------------------------------
# 1. Clone or update repo (depth=1 keeps it light for curl|sh install)
# ---------------------------------------------------------------------
if [[ ! -d $INSTALL_DIR/.git ]]; then
  log "Cloning Tide Tracker repo → $INSTALL_DIR"
  git clone --depth=1 "$REPO_URL" "$INSTALL_DIR"
else
  log "Repo already present, pulling latest"
  git -C "$INSTALL_DIR" pull --ff-only
fi

cd "$INSTALL_DIR"

# ---------------------------------------------------------------------
# 2. Wi-Fi provisioning (needs root)
# ---------------------------------------------------------------------
if [[ $EUID -ne 0 ]]; then
  log "Re-running Wi-Fi setup with sudo"
  exec sudo --preserve-env=INSTALL_DIR,CFG_FILE,BIN_PATH "$0" "$@"
fi

log "Running Wi-Fi provisioning script"
./scripts/wifi-setup.sh

# ---------------------------------------------------------------------
# 3. Fetch or build the binary
# ---------------------------------------------------------------------
log "Downloading pre-built binary"
./scripts/get-binary.sh      # adjust if you add --arch/--release flags

# ---------------------------------------------------------------------
# 4. System service / cron hooks
# ---------------------------------------------------------------------
log "Installing Tide Tracker service"
./scripts/setup-tide-tracker.sh \
  --binary "$BIN_PATH" \
  --config "$CFG_FILE"

# ---------------------------------------------------------------------
# 5. Fin
# ---------------------------------------------------------------------
log "✅ Installation complete!"
echo "  • Logs:  journalctl -u tide-tracker -f"
echo "  • Config: $CFG_FILE"

# Installation

Detailed installation steps are listed below.

## WiFi Connect Integration

WiFi Connect provides a captive portal for easy WiFi setup on headless devices. When no internet connection is detected, it automatically creates a "TideTracker-Setup" hotspot (password: "pi-tides") for configuration.

### Installation
```bash
sudo ./scripts/wifi-setup.sh
```

### Usage
- **Automatic**: Service runs on boot, activates portal when offline
- **Manual**: `sudo systemctl start wifi-connect.service`
- **Monitor**: `journalctl -u wifi-connect.service -f`
- **Update**: `sudo ./scripts/wifi-update.sh` (updates connectivity script only)

### Troubleshooting
```bash
# Check status
systemctl status wifi-connect.service

# Test connectivity  
ping -c 3 1.1.1.1

# Manual debug mode
sudo systemctl stop wifi-connect.service
sudo /usr/local/sbin/wifi-connect --portal-ssid "TideTracker-Setup" --portal-passphrase "pi-tides"
```

## Systemd Integration

We use systemd to run the tide tracker service.

### 1. Install Binary
```bash
sudo cp target/release/tide-tracker /usr/local/bin/
sudo chmod +x /usr/local/bin/tide-tracker
```

### 2. Create Service File
Create `/etc/systemd/system/tide-tracker.service`:
```ini
[Unit]
Description=Tide Tracker Display Update
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/tide-tracker
User=root
Group=root
StandardOutput=journal
StandardError=journal

# Memory limits for Raspberry Pi
MemoryMax=4M
MemoryHigh=2M

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictRealtime=true
```

### 3. Create Timer File
Create `/etc/systemd/system/tide-tracker.timer`:
```ini
[Unit]
Description=Update tide tracker every 10 minutes
Requires=tide-tracker.service

[Timer]
OnBootSec=2min
OnUnitActiveSec=10min
AccuracySec=1min

[Install]
WantedBy=timers.target
```

### 4. Enable and Start
```bash
sudo systemctl daemon-reload
sudo systemctl enable tide-tracker.timer
sudo systemctl start tide-tracker.timer

# Check status
sudo systemctl status tide-tracker.timer
sudo journalctl -u tide-tracker.service -f
```
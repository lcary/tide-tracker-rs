# Wiring

## Wiring Diagram

Connect the Waveshare 4.2" e-ink display to your Raspberry Pi:

**Standard Wiring:**
```
Raspberry Pi GPIO     →    E-ink Display
─────────────────────────────────────
3.3V (Pin 1)      →    VCC
GND (Pin 6)       →    GND
GPIO 10 (Pin 19)  →    DIN (MOSI)
GPIO 11 (Pin 23)  →    CLK (SCLK)
GPIO 8 (Pin 24)   →    CS
GPIO 25 (Pin 22)  →    DC
GPIO 17 (Pin 11)  →    RST
GPIO 24 (Pin 18)  →    BUSY
```

**Alternative Wiring (for hardware conflicts):**
```
Raspberry Pi GPIO     →    E-ink Display
─────────────────────────────────────
3.3V (Pin 1)      →    VCC
GND (Pin 6)       →    GND
GPIO 10 (Pin 19)  →    DIN (MOSI)  
GPIO 11 (Pin 23)  →    CLK (SCLK)
GPIO 7 (Pin 26)   →    CS           # Alternative CS pin
GPIO 25 (Pin 22)  →    DC
GPIO 27 (Pin 13)  →    RST          # Alternative RST pin  
GPIO 24 (Pin 18)  →    BUSY
```

### Pin Layout Reference
```
     3.3V → [ 1] [ 2]
            [ 3] [ 4]
            [ 5] [ 6] ← GND
      ALT CS→[ 7] [ 8]
            [ 9] [10]
    RST → [11] [12]
   ALT RST→[13] [14]
           [15] [16]
           [17] [18] ← BUSY
   MOSI → [19] [20]
           [21] [22] ← DC
    CLK → [23] [24] ← CS
           [25] [26] ← ALT CS (GPIO 7)
```

**Legend:**
- Standard pins: CS=8, RST=17  
- Alternative pins: ALT CS=7, ALT RST=27
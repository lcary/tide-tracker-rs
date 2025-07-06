import spidev
import time

spi = spidev.SpiDev()
spi.open(0, 0)  # open bus 0, device 0 (CE0)

spi.max_speed_hz = 500000  # safe starting speed
spi.mode = 0b00

print("Sending test bytes over SPI...")

# send some data (e.g. 0xAA, 0x55, 0xFF) and print what we read back
resp = spi.xfer2([0xAA, 0x55, 0xFF])

print(f"Received: {resp}")

spi.close()

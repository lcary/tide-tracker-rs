#!/usr/bin/env python3
"""
Quick e-ink display test script for Raspberry Pi Zero 2 W
Tests the Waveshare 4.2" e-ink display connectivity and basic functionality.
Uses GPIO pin configuration from tide-config.toml.

Usage:
    uv run test_display.py

This script will:
1. Load GPIO pin configuration from tide-config.toml
2. Initialize the display with configurable pins
3. Show a test pattern
4. Display some text
5. Clear the display
"""

import sys
import sys
import os
import time
import traceback
import toml

def load_config():
    """Load configuration from tide-config.toml"""
    try:
        with open('tide-config.toml', 'r') as f:
            config = toml.load(f)
            hardware = config['display']['hardware']
            return hardware
    except Exception as e:
        print(f"⚠️  Warning: Could not load config: {e}")
        print("Using default GPIO pin configuration")
        return {
            'cs_pin': 8,
            'dc_pin': 25,
            'rst_pin': 17,
            'busy_pin': 24
        }

def gpio_to_pin(gpio):
    """Convert GPIO number to physical pin number for display"""
    mapping = {
        8: 24,   # CS
        17: 11,  # RST
        24: 18,  # BUSY
        25: 22,  # DC
        7: 26,   # Alternative CS
        27: 13,  # Alternative RST
    }
    return mapping.get(gpio, 0)

def test_display():
    """Test the e-ink display with a simple pattern and text."""
    try:
        # Load GPIO pin configuration
        hw_config = load_config()
        
        print("📋 Using GPIO pin configuration:")
        print(f"   CS (Chip Select): GPIO {hw_config['cs_pin']} (Pin {gpio_to_pin(hw_config['cs_pin'])})")
        print(f"   DC (Data/Command): GPIO {hw_config['dc_pin']} (Pin {gpio_to_pin(hw_config['dc_pin'])})")
        print(f"   RST (Reset): GPIO {hw_config['rst_pin']} (Pin {gpio_to_pin(hw_config['rst_pin'])})")
        print(f"   BUSY: GPIO {hw_config['busy_pin']} (Pin {gpio_to_pin(hw_config['busy_pin'])})")

        # Add the waveshare library path
        libdir = os.path.join(os.path.dirname(os.path.dirname(os.path.realpath(__file__))), 'lib')
        if os.path.exists(libdir):
            sys.path.append(libdir)
        
        import epaper 
        from PIL import Image, ImageDraw, ImageFont
        import logging
        
        # Set up logging
        logging.basicConfig(level=logging.DEBUG)
        
        print("🔧 Initializing Waveshare 4.2\" e-ink display...")
        
        # Initialize display
        # Note: The epaper library uses hardcoded pins, but we show the config for reference
        epd = epaper.epaper('epd4in2b_V2').EPD()
        epd.init()
        
        print(f"📐 Display size: {epd.width} x {epd.height} pixels")
        
        # Create a new image with white background
        image = Image.new('1', (epd.width, epd.height), 255)  # 255: white, 0: black
        draw = ImageDraw.Draw(image)
        
        # Test 1: Draw test pattern
        print("🎨 Drawing test pattern...")
        
        # Border
        draw.rectangle([(0, 0), (epd.width-1, epd.height-1)], outline=0, width=2)
        
        # Cross pattern
        draw.line([(0, 0), (epd.width, epd.height)], fill=0, width=1)
        draw.line([(0, epd.height), (epd.width, 0)], fill=0, width=1)
        
        # Grid
        for x in range(0, epd.width, 50):
            draw.line([(x, 0), (x, epd.height)], fill=0, width=1)
        for y in range(0, epd.height, 50):
            draw.line([(0, y), (epd.width, y)], fill=0, width=1)
        
        # Test 2: Add text
        print("📝 Adding text...")
        try:
            # Try to load a font, fallback to default if not available
            font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf', 24)
        except:
            font = ImageFont.load_default()
        
        # Title
        text = "E-INK DISPLAY TEST"
        bbox = draw.textbbox((0, 0), text, font=font)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]
        x = (epd.width - text_width) // 2
        y = 30
        draw.text((x, y), text, font=font, fill=0)
        
        # Info
        info_lines = [
            f"Resolution: {epd.width}x{epd.height}",
            f"Time: {time.strftime('%Y-%m-%d %H:%M:%S')}",
            "Raspberry Pi Zero 2 W",
            "Waveshare 4.2\" E-Paper",
            "",
            "If you can see this text,",
            "your display is working!"
        ]
        
        try:
            small_font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 16)
        except:
            small_font = ImageFont.load_default()
        
        y_offset = 80
        for line in info_lines:
            if line:  # Skip empty lines
                bbox = draw.textbbox((0, 0), line, font=small_font)
                text_width = bbox[2] - bbox[0]
                x = (epd.width - text_width) // 2
                draw.text((x, y_offset), line, font=small_font, fill=0)
            y_offset += 25

        red_image = Image.new('1', (epd.width, epd.height), 255)  # 255: white, 0: black        
        red_buf = epd.getbuffer(red_image) 
        # Test 3: Display the image
        print("📺 Updating display...")
        epd.display(epd.getbuffer(image), red_buf)
        
        print("✅ Test pattern displayed! Check your e-ink screen.")
        print("⏳ Waiting 10 seconds before clearing...")
        time.sleep(10)
        
        # Test 4: Clear display
        print("🧹 Clearing display...")
        epd.Clear()
        
        print("💤 Putting display to sleep...")
        epd.sleep()
        
        print("🎉 Display test completed successfully!")
        return True
        
    except ImportError as e:
        print("❌ ERROR: Missing dependencies")
        print(f"   {e}")
        print("\n🔧 To fix this, install the required packages:")
        print("   sudo apt update")
        print("   sudo apt install python3-pip python3-pil python3-numpy")
        print("   pip3 install waveshare-epd")
        print("\n💡 Alternative: Use the official Waveshare example code:")
        print("   git clone https://github.com/waveshare/e-Paper")
        print("   cd e-Paper/RaspberryPi_JetsonNano/python/examples")
        return False
        
    except Exception as e:
        print(f"❌ ERROR: Display test failed")
        print(f"   {e}")
        print("\n🔍 Common issues:")
        print("   1. SPI not enabled: sudo raspi-config → Interface Options → SPI → Enable")
        print("   2. Wiring problem: Check connections to Waveshare 4.2\" display")
        print("   3. Permissions: Run as sudo or add user to spi group")
        print("   4. Display type: Make sure you have the 4.2\" model")
        print("\n🔧 Debug info:")
        traceback.print_exc()
        return False

if __name__ == "__main__":
    print("🚀 Waveshare 4.2\" E-ink Display Test")
    print("=" * 40)
    
    # Check if running on Raspberry Pi
    try:
        with open('/proc/cpuinfo', 'r') as f:
            cpuinfo = f.read()
            if 'raspberry pi' not in cpuinfo.lower():
                print("⚠️  WARNING: This doesn't appear to be a Raspberry Pi")
                response = input("Continue anyway? [y/N]: ")
                if response.lower() != 'y':
                    sys.exit(0)
    except:
        pass
    
    # Check SPI
    if not os.path.exists('/dev/spidev0.0'):
        print("⚠️  WARNING: SPI device not found at /dev/spidev0.0")
        print("   Enable SPI: sudo raspi-config → Interface Options → SPI → Enable")
        print("   Then reboot: sudo reboot")
        sys.exit(1)
    
    success = test_display()
    sys.exit(0 if success else 1)

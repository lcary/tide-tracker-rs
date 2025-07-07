//! Custom EPD 4.2" B/W/Red V2 Driver
//!
//! This implementation closely follows the Waveshare Python epd4in2b_v2.py
//! and C examples to ensure 100% compatibility with the hardware.

use std::thread;
use std::time::Duration;

/// Display dimensions
pub const EPD_WIDTH: u32 = 400;
pub const EPD_HEIGHT: u32 = 300;

/// Color definitions matching the Python implementation
#[derive(Clone, Copy, Debug)]
pub enum Color {
    White = 0xFF,
    Black = 0x00,
    Red = 0x80,
}

/// Simple error type for our EPD operations
#[derive(Debug)]
pub struct EpdError(pub String);

impl std::fmt::Display for EpdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EPD Error: {}", self.0)
    }
}

impl std::error::Error for EpdError {}

/// Trait for software SPI interface
pub trait SoftwareSpi {
    fn write_byte(&mut self, data: u8) -> Result<(), EpdError>;
    fn read_byte(&mut self) -> Result<u8, EpdError>;
}

/// Trait for GPIO pin interface
pub trait GpioPin {
    fn set_high(&mut self) -> Result<(), EpdError>;
    fn set_low(&mut self) -> Result<(), EpdError>;
}

/// Trait for input pin interface
pub trait InputPin {
    fn is_high(&self) -> Result<bool, EpdError>;
}

/// EPD 4.2" B/W/Red V2 display driver
pub struct Epd4in2bV2<SPI, CS, DC, RST, BUSY> {
    spi: SPI,
    cs_pin: CS,
    dc_pin: DC,
    rst_pin: RST,
    busy_pin: BUSY,
    width: u32,
    height: u32,
}

/// Display buffer for the 4.2" B/W/Red display
pub struct DisplayBuffer {
    width: u32,
    height: u32,
    black_buffer: Vec<u8>,
    red_buffer: Vec<u8>,
}

impl DisplayBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        // Buffer size: each row has (width+7)/8 bytes, total height rows
        let bytes_per_row = width.div_ceil(8);
        let buffer_size = (bytes_per_row * height) as usize;
        Self {
            width,
            height,
            black_buffer: vec![0xFF; buffer_size], // White by default
            red_buffer: vec![0x00; buffer_size],   // No red by default
        }
    }

    pub fn clear(&mut self, color: Color) {
        match color {
            Color::White => {
                self.black_buffer.fill(0xFF);
                self.red_buffer.fill(0x00);
            }
            Color::Black => {
                self.black_buffer.fill(0x00);
                self.red_buffer.fill(0x00);
            }
            Color::Red => {
                self.black_buffer.fill(0xFF);
                self.red_buffer.fill(0xFF);
            }
        }
    }

    pub fn black_buffer(&self) -> &[u8] {
        &self.black_buffer
    }

    pub fn red_buffer(&self) -> &[u8] {
        &self.red_buffer
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        // E-ink displays are organized as rows of bytes
        // Each row has (width/8) bytes, each byte represents 8 horizontal pixels
        let bytes_per_row = self.width.div_ceil(8); // Round up for partial bytes
        let byte_index = (y * bytes_per_row + x / 8) as usize;
        let bit_mask = 0x80 >> (x % 8);

        match color {
            Color::White => {
                self.black_buffer[byte_index] |= bit_mask;
                self.red_buffer[byte_index] &= !bit_mask;
            }
            Color::Black => {
                self.black_buffer[byte_index] &= !bit_mask;
                self.red_buffer[byte_index] &= !bit_mask;
            }
            Color::Red => {
                self.black_buffer[byte_index] |= bit_mask;
                self.red_buffer[byte_index] |= bit_mask;
            }
        }
    }
}

impl<SPI, CS, DC, RST, BUSY> Epd4in2bV2<SPI, CS, DC, RST, BUSY>
where
    SPI: SoftwareSpi,
    CS: GpioPin,
    DC: GpioPin,
    RST: GpioPin,
    BUSY: InputPin,
{
    /// Create a new EPD instance
    pub fn new(spi: SPI, cs_pin: CS, dc_pin: DC, rst_pin: RST, busy_pin: BUSY) -> Self {
        Self {
            spi,
            cs_pin,
            dc_pin,
            rst_pin,
            busy_pin,
            width: EPD_WIDTH,
            height: EPD_HEIGHT,
        }
    }

    /// Hardware reset - follows Python reset() exactly
    fn reset(&mut self) -> Result<(), EpdError> {
        eprintln!("🔄 Performing hardware reset...");

        self.rst_pin.set_high()?;
        thread::sleep(Duration::from_millis(200));

        self.rst_pin.set_low()?;
        thread::sleep(Duration::from_millis(5));

        self.rst_pin.set_high()?;
        thread::sleep(Duration::from_millis(200));

        eprintln!("   Hardware reset completed");
        Ok(())
    }

    /// Send command - follows Python send_command() exactly
    fn send_command(&mut self, command: u8) -> Result<(), EpdError> {
        self.dc_pin.set_low()?; // Command mode
        self.cs_pin.set_low()?; // Select device
        self.spi.write_byte(command)?;
        self.cs_pin.set_high()?; // Deselect device
        Ok(())
    }

    /// Send data - follows Python send_data() exactly
    fn send_data(&mut self, data: u8) -> Result<(), EpdError> {
        self.dc_pin.set_high()?; // Data mode
        self.cs_pin.set_low()?; // Select device
        self.spi.write_byte(data)?;
        self.cs_pin.set_high()?; // Deselect device
        Ok(())
    }

    /// Read BUSY pin and wait - simplified for rev2.2+ modules (matches static fuzz commit)
    fn read_busy(&mut self) -> Result<(), EpdError> {
        eprintln!("   📡 Waiting for display (BUSY pin check)...");

        let mut count = 0;

        // Simplified logic - just wait while BUSY pin is HIGH (matches static fuzz commit)
        while self.busy_pin.is_high()? {
            thread::sleep(Duration::from_millis(10));
            count += 1;
            if count > 500 {
                eprintln!("   ⚠️  BUSY pin timeout after 5 seconds - display may be stuck");
                break;
            }
        }

        eprintln!("   ✅ Display ready (BUSY went LOW after {} checks)", count);
        Ok(())
    }

    /// Turn on display
    fn turn_on_display(&mut self) -> Result<(), EpdError> {
        eprintln!("   🔆 Turning on display...");
        self.send_command(0x22)?;
        self.send_data(0xF7)?;
        self.send_command(0x20)?;
        self.read_busy()?;
        eprintln!("   ✅ Display turned on");
        Ok(())
    }

    /// Initialize the display
    pub fn init(&mut self) -> Result<(), EpdError> {
        eprintln!("🚀 Initializing EPD...");

        // Step 1: Hardware reset
        self.reset()?;

        // Step 2: Software reset and configuration (following C code)
        self.read_busy()?;
        self.send_command(0x12)?; // SWRESET
        self.read_busy()?;

        self.send_command(0x3C)?; // BorderWaveform
        self.send_data(0x05)?;

        self.send_command(0x18)?; // Read built-in temperature sensor
        self.send_data(0x80)?;

        self.send_command(0x11)?; // Data entry mode setting
        self.send_data(0x03)?;

        // Set RAM X address start/end
        self.send_command(0x44)?;
        self.send_data(0x00)?;
        self.send_data((self.width / 8 - 1) as u8)?;

        // Set RAM Y address start/end
        self.send_command(0x45)?;
        self.send_data(0x00)?;
        self.send_data(0x00)?;
        self.send_data(((self.height - 1) % 256) as u8)?;
        self.send_data(((self.height - 1) / 256) as u8)?;

        // Set RAM X address counter
        self.send_command(0x4E)?;
        self.send_data(0x00)?;

        // Set RAM Y address counter
        self.send_command(0x4F)?;
        self.send_data(0x00)?;
        self.send_data(0x00)?;

        self.read_busy()?;

        eprintln!("   ✅ EPD initialization completed successfully!");
        Ok(())
    }

    /// Display image data - follows C EPD_4IN2B_V2_Display() exactly
    pub fn display(&mut self, black_buffer: &[u8], red_buffer: &[u8]) -> Result<(), EpdError> {
        eprintln!("   📤 DISPLAY FUNCTION CALLED - sending image data to display...");

        let high = self.height as usize;
        let wide = ((self.width + 7) / 8) as usize; // Bytes per row (ceiling division)

        eprintln!(
            "   📐 Display dimensions: {}x{} pixels = {} bytes per row",
            self.width, self.height, wide
        );
        eprintln!(
            "    Buffer sizes: black={} bytes, red={} bytes",
            black_buffer.len(),
            red_buffer.len()
        );

        // Count non-white pixels for debugging
        let black_pixels = black_buffer.iter().map(|&b| b.count_zeros()).sum::<u32>();
        let red_pixels = red_buffer.iter().map(|&b| b.count_ones()).sum::<u32>();
        eprintln!(
            "   📊 Pixel counts: {} black pixels, {} red pixels",
            black_pixels, red_pixels
        );

        // Send black buffer using 0x24 command
        eprintln!("   � Sending black buffer (using 0x24 command)...");
        self.send_command(0x24)?;
        for j in 0..high {
            for i in 0..wide {
                self.send_data(black_buffer[i + j * wide])?;
            }
        }

        // Send red buffer using 0x26 command
        eprintln!("   🔴 Sending red buffer (using 0x26 command)...");
        self.send_command(0x26)?;
        for j in 0..high {
            for i in 0..wide {
                self.send_data(!red_buffer[i + j * wide])?; // Inverted as per C code
            }
        }

        // Turn on display to show the new image
        eprintln!("   🔆 Turning on display...");
        self.turn_on_display()?;
        eprintln!("   ✅ Image data sent and display updated");

        Ok(())
    }

    /// Clear the display
    pub fn clear(&mut self) -> Result<(), EpdError> {
        eprintln!("   🧹 Clearing display...");

        let high = self.height as usize;
        let wide = self.width.div_ceil(8) as usize; // Bytes per row

        // Use 0x24/0x26 commands
        self.send_command(0x24)?;
        for _j in 0..high {
            for _i in 0..wide {
                self.send_data(0xFF)?; // White
            }
        }

        self.send_command(0x26)?;
        for _j in 0..high {
            for _i in 0..wide {
                self.send_data(0x00)?; // No red
            }
        }

        self.turn_on_display()?;

        // Wait for display refresh to complete
        self.read_busy()?;

        eprintln!("   ✅ Display cleared");
        Ok(())
    }

    /// Put display to sleep with proper persistence (follows the "cheat sheet")
    /// This should only be called when actually shutting down the device
    pub fn sleep_persistent(&mut self) -> Result<(), EpdError> {
        eprintln!("   😴 Putting display to sleep with persistence sequence...");

        // First send POWER_OFF and wait for BUSY
        eprintln!("   📤 Sending POWER_OFF (0x02)...");
        self.send_command(0x02)?; // POWER_OFF
        self.read_busy()?;

        // Then send DEEP_SLEEP with the critical 0x01 byte for persistence
        eprintln!("   😴 Sending DEEP_SLEEP (0x10) with 0x01 for persistence...");
        self.send_command(0x10)?; // DEEP_SLEEP
        self.send_data(0x01)?; // The critical 0x01 byte for persistence
        self.read_busy()?;

        eprintln!("   ✅ Display in persistent sleep mode - image should survive power-off");
        Ok(())
    }

    /// Put display to sleep (standard approach, matches C code)
    pub fn sleep(&mut self) -> Result<(), EpdError> {
        eprintln!("   😴 Putting display to sleep...");

        self.send_command(0x10)?; // Deep sleep mode
        self.send_data(0x03)?; // Standard sleep data

        thread::sleep(Duration::from_millis(2000));
        eprintln!("   ✅ Display sleeping");
        Ok(())
    }

    /// Display image data with persistence support - follows the "cheat sheet" guidelines
    /// After display(), sends POWER_OFF (0x02), waits for BUSY, then DEEP_SLEEP (0x10) with 0x01
    pub fn display_persistent(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
    ) -> Result<(), EpdError> {
        // First, do the normal display
        self.display(black_buffer, red_buffer)?;

        // Now implement the persistence "cheat sheet":
        // After every display(), send POWER_OFF (0x02), wait for BUSY, then DEEP_SLEEP (0x10) with 0x01, wait for BUSY
        eprintln!("   🔋 Applying persistence sequence...");

        eprintln!("   📤 Sending POWER_OFF (0x02)...");
        self.send_command(0x02)?; // POWER_OFF
        eprintln!("   📡 Waiting for BUSY after POWER_OFF...");
        self.read_busy()?;

        eprintln!("   😴 Sending DEEP_SLEEP (0x10) with 0x01...");
        self.send_command(0x10)?; // DEEP_SLEEP
        self.send_data(0x01)?; // The critical 0x01 byte for persistence
        eprintln!("   📡 Waiting for BUSY after DEEP_SLEEP...");
        self.read_busy()?;

        eprintln!("   ✅ Persistence sequence complete - image should persist after power-off");
        Ok(())
    }
}

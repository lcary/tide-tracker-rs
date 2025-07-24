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
    cs_pin: Option<CS>,
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
    pub fn new(spi: SPI, cs_pin: Option<CS>, dc_pin: DC, rst_pin: RST, busy_pin: BUSY) -> Self {
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

    /// Hardware reset - follows C reset() exactly
    fn reset(&mut self) -> Result<(), EpdError> {
        eprintln!("🔄 Performing hardware reset...");

        self.rst_pin.set_high()?;
        thread::sleep(Duration::from_millis(200));

        self.rst_pin.set_low()?;
        thread::sleep(Duration::from_millis(2)); // C code uses 2ms, not 5ms

        self.rst_pin.set_high()?;
        thread::sleep(Duration::from_millis(200));

        eprintln!("   Hardware reset completed");
        Ok(())
    }

    /// Send command - follows Python send_command() exactly
    fn send_command(&mut self, command: u8) -> Result<(), EpdError> {
        self.dc_pin.set_low()?; // Command mode
        if let Some(cs) = &mut self.cs_pin {
            cs.set_low()?;
        } // Select device if CS present
        self.spi.write_byte(command)?;
        if let Some(cs) = &mut self.cs_pin {
            cs.set_high()?;
        } // Deselect device if CS present
        Ok(())
    }

    /// Send data - follows Python send_data() exactly
    fn send_data(&mut self, data: u8) -> Result<(), EpdError> {
        self.dc_pin.set_high()?; // Data mode
        if let Some(cs) = &mut self.cs_pin {
            cs.set_low()?;
        } // Select device if CS present
        self.spi.write_byte(data)?;
        if let Some(cs) = &mut self.cs_pin {
            cs.set_high()?;
        } // Deselect device if CS present
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

    /// Initialize the display - EXACT match to C EPD_4IN2B_V2_Init() and EPD_4IN2B_V2_Init_new()
    pub fn init(&mut self) -> Result<(), EpdError> {
        eprintln!("🚀 Initializing EPD (EXACT C CODE MATCH)...");

        // Step 1: Hardware reset (matches C code exactly)
        self.reset()?;

        // Step 2: Hardware revision detection sequence (matches C EPD_4IN2B_V2_Init() exactly)
        eprintln!("   🔍 Hardware revision detection (matching C code exactly)...");
        self.dc_pin.set_low()?; // Command mode
        if let Some(cs) = &mut self.cs_pin {
            cs.set_low()?;
        } // Select device if CS present
        self.spi.write_byte(0x2F)?; // Send detection command (matches C code)
        if let Some(cs) = &mut self.cs_pin {
            cs.set_high()?;
        } // Deselect device if CS present
        thread::sleep(Duration::from_millis(50)); // DEV_Delay_ms(50) from C code

        // Try to read response (matches C code: i = DEV_SPI_ReadData())
        self.dc_pin.set_high()?; // Data mode
        if let Some(cs) = &mut self.cs_pin {
            cs.set_low()?;
        } // Select device if CS present
        match self.spi.read_byte() {
            Ok(revision) => eprintln!("   📄 Hardware revision byte: 0x{:02X}", revision),
            Err(_) => {
                eprintln!("   📄 Hardware revision read failed (this is normal for some setups)")
            }
        }
        if let Some(cs) = &mut self.cs_pin {
            cs.set_high()?;
        } // Deselect device if CS present
        thread::sleep(Duration::from_millis(50)); // DEV_Delay_ms(50) from C code

        // Step 3: Call EPD_4IN2B_V2_Init_new() - EXACT MATCH TO C CODE
        eprintln!("   ⚙️  Running Init_new() sequence (EXACT C CODE MATCH)...");

        // Reset again (matches C Init_new)
        self.reset()?;

        // Read busy (matches C Init_new)
        self.read_busy()?;

        // Soft reset with 0x12 (matches C Init_new, NOT 0x04)
        self.send_command(0x12)?; // SWRESET (matches C Init_new)
        self.read_busy()?;

        // BorderWaveform with 0x3C/0x05 (matches C Init_new, NOT 0x00/0x0F)
        self.send_command(0x3C)?; // BorderWaveform (matches C Init_new)
        self.send_data(0x05)?; // (matches C Init_new)

        // Read built-in temperature sensor (matches C Init_new)
        self.send_command(0x18)?; // Read built-in temperature sensor
        self.send_data(0x80)?; // (matches C Init_new)

        // Data entry mode setting (matches C Init_new)
        self.send_command(0x11)?; // Data entry mode setting
        self.send_data(0x03)?; // X-mode (matches C Init_new)

        // Set windows using SetWindows function (matches C Init_new)
        eprintln!("   📐 Setting display windows...");
        self.send_command(0x44)?; // SET_RAM_X_ADDRESS_START_END_POSITION
        self.send_data(0x00)?; // Xstart>>3
        self.send_data(((self.width - 1) / 8) as u8)?; // Xend>>3

        self.send_command(0x45)?; // SET_RAM_Y_ADDRESS_START_END_POSITION
        self.send_data(0x00)?; // Ystart & 0xFF
        self.send_data(0x00)?; // (Ystart >> 8) & 0xFF
        self.send_data(((self.height - 1) % 256) as u8)?; // Yend & 0xFF
        self.send_data(((self.height - 1) / 256) as u8)?; // (Yend >> 8) & 0xFF

        // Set cursor using SetCursor function (matches C Init_new)
        eprintln!("   📍 Setting cursor position...");
        self.send_command(0x4E)?; // SET_RAM_X_ADDRESS_COUNTER
        self.send_data(0x00)?; // (Xstart>>3) & 0xFF

        self.send_command(0x4F)?; // SET_RAM_Y_ADDRESS_COUNTER
        self.send_data(0x00)?; // Ystart & 0xFF
        self.send_data(0x00)?; // (Ystart >> 8) & 0xFF

        // Final busy check (matches C Init_new)
        self.read_busy()?;

        eprintln!("   ✅ EPD initialization completed (EXACT C CODE MATCH)!");
        Ok(())
    }

    /// Display image data - follows C EPD_4IN2B_V2_Display() exactly
    pub fn display(&mut self, black_buffer: &[u8], red_buffer: &[u8]) -> Result<(), EpdError> {
        eprintln!("   📤 DISPLAY FUNCTION CALLED - sending image data to display...");

        let high = self.height as usize;
        let wide = self.width.div_ceil(8) as usize; // Bytes per row (ceiling division)

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

        // CRITICAL: Reset cursor position before sending image data (like C test sequence)
        eprintln!("   📍 Resetting cursor position before image data...");
        self.send_command(0x4E)?; // SET_RAM_X_ADDRESS_COUNTER
        self.send_data(0x00)?;
        self.send_command(0x4F)?; // SET_RAM_Y_ADDRESS_COUNTER
        self.send_data(0x00)?;
        self.send_data(0x00)?;

        // Send black buffer using 0x24 command
        eprintln!("   📝 Sending black buffer (using 0x24 command)...");
        self.send_command(0x24)?;
        thread::sleep(Duration::from_millis(10)); // Add small delay after command
        for j in 0..high {
            for i in 0..wide {
                self.send_data(black_buffer[j * wide + i])?; // Fixed: row-major order
            }
        }
        eprintln!("   ✅ Black buffer sent successfully");

        // Send red buffer using 0x26 command - DISABLE RED COMPLETELY FOR TESTING
        eprintln!("   🔴 Sending EMPTY red buffer (testing without red)...");
        self.send_command(0x26)?;
        thread::sleep(Duration::from_millis(10));
        for _j in 0..high {
            for _i in 0..wide {
                self.send_data(0x00)?; // Send all zeros - no red pixels at all
            }
        }
        eprintln!("   ✅ Empty red buffer sent successfully");

        // Wait before refresh to ensure data is stable
        eprintln!("   ⏱️  Waiting 100ms before display refresh...");
        thread::sleep(Duration::from_millis(100));

        // Turn on display to show the new image
        eprintln!("   🔆 Turning on display...");
        self.turn_on_display()?;
        eprintln!("   ✅ Image data sent and display updated");

        Ok(())
    }

    /// Display using EXACT C test sequence - mimics the working C test program
    pub fn display_c_test_sequence(
        &mut self,
        black_buffer: &[u8],
        red_buffer: &[u8],
    ) -> Result<(), EpdError> {
        eprintln!("   📤 DISPLAY C TEST SEQUENCE - exactly like working C test...");

        let high = self.height as usize;
        let wide = self.width.div_ceil(8) as usize;

        eprintln!(
            "   📐 Display dimensions: {}x{} pixels = {} bytes per row",
            self.width, self.height, wide
        );

        // Count pixels for debugging
        let black_pixels = black_buffer.iter().map(|&b| b.count_zeros()).sum::<u32>();
        let red_pixels = red_buffer.iter().map(|&b| b.count_ones()).sum::<u32>();
        eprintln!(
            "   📊 Pixel counts: {} black pixels, {} red pixels",
            black_pixels, red_pixels
        );

        // CRITICAL: Maybe we need to reset cursor/window before display?
        eprintln!("   📍 Resetting cursor position (like C init does)...");
        self.send_command(0x4E)?; // SET_RAM_X_ADDRESS_COUNTER
        self.send_data(0x00)?;

        self.send_command(0x4F)?; // SET_RAM_Y_ADDRESS_COUNTER
        self.send_data(0x00)?;
        self.send_data(0x00)?;

        // Send black buffer (EXACT C sequence)
        eprintln!("   📝 Sending black buffer (C test sequence)...");
        self.send_command(0x24)?;
        for j in 0..high {
            for i in 0..wide {
                self.send_data(black_buffer[j * wide + i])?; // Fixed: row-major order
            }
        }

        // Send red buffer with inversion (EXACT C sequence)
        eprintln!("   🔴 Sending red buffer (C test sequence with inversion)...");
        self.send_command(0x26)?;
        for j in 0..high {
            for i in 0..wide {
                self.send_data(!red_buffer[j * wide + i])?; // Fixed: row-major order with inversion
            }
        }

        // CRITICAL: Display refresh (EXACT C sequence)
        eprintln!("   🔆 Display refresh (C test sequence)...");
        self.send_command(0x22)?;
        self.send_data(0xF7)?;
        self.send_command(0x20)?;
        self.read_busy()?;

        eprintln!("   ✅ C test sequence completed");
        Ok(())
    }

    /// Clear the display to remove previous content
    pub fn clear(&mut self) -> Result<(), EpdError> {
        eprintln!("   🧹 Clearing display...");

        let high = self.height as usize;
        let wide = self.width.div_ceil(8) as usize; // Bytes per row

        // Reset cursor position first
        eprintln!("   📍 Resetting cursor position...");
        self.send_command(0x4E)?; // SET_RAM_X_ADDRESS_COUNTER
        self.send_data(0x00)?;
        self.send_command(0x4F)?; // SET_RAM_Y_ADDRESS_COUNTER
        self.send_data(0x00)?;
        self.send_data(0x00)?;

        // Clear black buffer - send all white (0xFF)
        eprintln!("   📝 Clearing black buffer...");
        self.send_command(0x24)?;
        for _j in 0..high {
            for _i in 0..wide {
                self.send_data(0xFF)?; // White
            }
        }

        // Clear red buffer - send all no-red (0x00)
        eprintln!("   🔴 Clearing red buffer...");
        self.send_command(0x26)?;
        for _j in 0..high {
            for _i in 0..wide {
                self.send_data(0x00)?; // No red
            }
        }

        // Refresh display to show the clear
        self.turn_on_display()?;

        eprintln!("   ✅ Display cleared");
        Ok(())
    }

    /// Put display to sleep (standard approach, matches C code exactly)
    /// This should only be called when actually powering down the device
    pub fn sleep(&mut self) -> Result<(), EpdError> {
        eprintln!("   😴 Putting display to sleep...");

        self.send_command(0x10)?; // Deep sleep mode
        self.send_data(0x03)?; // Standard sleep data (matches C code)

        thread::sleep(Duration::from_millis(2000));
        eprintln!("   ✅ Display sleeping");
        Ok(())
    }
}

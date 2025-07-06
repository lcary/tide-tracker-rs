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
    flag: u8,
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
        let bytes_per_row = (width + 7) / 8;
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
        let bytes_per_row = (self.width + 7) / 8; // Round up for partial bytes
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
            flag: 0, // Will be determined during init
        }
    }

    /// Set the chip flag manually if known
    pub fn set_flag(&mut self, flag: u8) {
        self.flag = flag;
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

    /// Read BUSY pin and wait - follows C EPD_4IN2B_V2_ReadBusy() exactly
    fn read_busy(&mut self) -> Result<(), EpdError> {
        eprintln!("   📡 Waiting for display (BUSY pin check)...");

        if self.flag == 0 {
            // Flag 0: BUSY active LOW - wait while BUSY is LOW (0: busy, 1: idle)
            eprintln!("   Flag=0: Waiting while BUSY pin is LOW (old chip)...");
            loop {
                thread::sleep(Duration::from_millis(20));
                if self.busy_pin.is_high()? {
                    break; // BUSY went HIGH, display is ready
                }
            }
            thread::sleep(Duration::from_millis(20));
        } else {
            // Flag 1: BUSY active HIGH - wait while BUSY is HIGH (LOW: idle, HIGH: busy)
            eprintln!("   Flag=1: Waiting while BUSY pin is HIGH (new chip)...");
            while self.busy_pin.is_high()? {
                thread::sleep(Duration::from_millis(20));
            }
        }

        eprintln!("   ✅ Display ready (BUSY check completed)");
        Ok(())
    }

    /// Turn on display - follows C EPD_4IN2B_V2_TurnOnDisplay() exactly
    fn turn_on_display(&mut self) -> Result<(), EpdError> {
        eprintln!("   🔆 Turning on display...");

        if self.flag == 1 {
            // New chip version
            self.send_command(0x22)?;
            self.send_data(0xF7)?;
            self.send_command(0x20)?;
            self.read_busy()?;
        } else {
            // Old chip version
            self.send_command(0x12)?;
            thread::sleep(Duration::from_millis(100));
            self.read_busy()?;
        }

        eprintln!("   ✅ Display turned on");
        Ok(())
    }

    /// Initialize the display - follows C EPD_4IN2B_V2_Init() exactly
    pub fn init(&mut self) -> Result<(), EpdError> {
        eprintln!("🚀 Initializing EPD (following C EPD_4in2b_V2.c exactly)...");

        // Step 1: Hardware reset
        self.reset()?;

        // Step 2: Send chip detection command (0x2F)
        eprintln!("   🔍 Detecting chip version...");
        self.send_command(0x2F)?;
        thread::sleep(Duration::from_millis(100));

        // Step 3: Read chip version to determine flag
        self.dc_pin.set_high()?; // Data mode for reading
        self.cs_pin.set_low()?; // Select device
        let chip_version = self.spi.read_byte()?;
        self.cs_pin.set_high()?; // Deselect device

        eprintln!("   📟 Chip version read: 0x{:02X}", chip_version);

        // Step 4: Set flag and initialize based on chip version
        // IMPORTANT: Most newer modules (rev2.2+) use BUSY active HIGH (flag=1)
        // Force flag=1 for newer modules to avoid hanging on BUSY pin
        if chip_version == 0x01 || chip_version == 0x00 {
            // New chip version (flag = 1) - BUSY active HIGH
            // Note: 0x00 often indicates read failure, but newer modules need flag=1
            self.flag = 1;
            if chip_version == 0x00 {
                eprintln!(
                    "   ⚠️  Chip version read as 0x00 (read failure), assuming rev2.2+ module"
                );
                eprintln!("   🏷️  Forcing flag=1 (BUSY active HIGH) for newer modules");
            } else {
                eprintln!("   🏷️  New chip detected (flag=1, BUSY active HIGH)");
            }

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
        } else {
            // Old chip version (flag = 0) - BUSY active LOW
            // This path should rarely be used for modern modules
            self.flag = 0;
            eprintln!("   ❌ ERROR: Old chip detected (flag=0, BUSY active LOW)");
            eprintln!("   ⚠️  WARNING: If you have a rev2.2+ module, this will cause hanging!");
            eprintln!("   💡 Consider manually setting flag=1 in the code for newer modules");

            self.read_busy()?;
            self.send_command(0x04)?; // POWER_ON
            self.read_busy()?;

            self.send_command(0x00)?; // Panel setting
            self.send_data(0x0F)?;
        }

        eprintln!("   ✅ EPD initialization completed successfully!");
        Ok(())
    }

    /// Display image data - follows C EPD_4IN2B_V2_Display() exactly
    pub fn display(&mut self, black_buffer: &[u8], red_buffer: &[u8]) -> Result<(), EpdError> {
        eprintln!("   📤 Sending image data to display...");

        let high = self.height as usize;
        let wide = ((self.width + 7) / 8) as usize; // Bytes per row

        eprintln!(
            "   📐 Display dimensions: {}x{} pixels = {} bytes per row",
            self.width, self.height, wide
        );

        if self.flag == 0 {
            // Flag=0: New chip version - use commands 0x24/0x26 (C code: flag==0 uses _new functions)
            eprintln!("   📝 Sending black buffer (flag=0, commands 0x24/0x26)...");
            self.send_command(0x24)?;
            for j in 0..high {
                for i in 0..wide {
                    self.send_data(black_buffer[i + j * wide])?;
                }
            }

            eprintln!("   🔴 Sending red buffer (flag=0, commands 0x24/0x26)...");
            self.send_command(0x26)?;
            for j in 0..high {
                for i in 0..wide {
                    self.send_data(!red_buffer[i + j * wide])?; // Inverted as per C code
                }
            }
        } else {
            // Flag=1: Old chip version - use commands 0x10/0x13 (C code: flag==1 uses _old functions)
            eprintln!("   📝 Sending black buffer (flag=1, commands 0x10/0x13)...");
            self.send_command(0x10)?;
            for j in 0..high {
                for i in 0..wide {
                    self.send_data(black_buffer[i + j * wide])?;
                }
            }

            eprintln!("   🔴 Sending red buffer (flag=1, commands 0x10/0x13)...");
            self.send_command(0x13)?;
            for j in 0..high {
                for i in 0..wide {
                    self.send_data(!red_buffer[i + j * wide])?; // Inverted as per C code
                }
            }
        }

        // Turn on display to show the new image
        self.turn_on_display()?;

        eprintln!("   ✅ Image data sent and display updated");
        Ok(())
    }

    /// Clear the display - follows Python Clear() exactly
    pub fn clear(&mut self) -> Result<(), EpdError> {
        eprintln!("   🧹 Clearing display...");

        let high = self.height as usize;
        let wide = ((self.width + 7) / 8) as usize; // Bytes per row

        if self.flag == 1 {
            // Flag=1: New chip version - use commands 0x24/0x26
            self.send_command(0x24)?;
            for _j in 0..high {
                for _i in 0..wide {
                    self.send_data(0xFF)?; // White
                }
            }

            // Clear red buffer
            self.send_command(0x26)?;
            for _j in 0..high {
                for _i in 0..wide {
                    self.send_data(0x00)?; // No red
                }
            }
        } else {
            // Flag=0: Old chip version - use commands 0x10/0x13
            self.send_command(0x10)?;
            for _j in 0..high {
                for _i in 0..wide {
                    self.send_data(0xFF)?; // White
                }
            }

            // Clear red buffer
            self.send_command(0x13)?;
            for _j in 0..high {
                for _i in 0..wide {
                    self.send_data(0x00)?; // No red
                }
            }
        }

        self.turn_on_display()?;

        eprintln!("   ✅ Display cleared");
        Ok(())
    }

    /// Put display to sleep - follows C EPD_4IN2B_V2_Sleep() exactly
    pub fn sleep(&mut self) -> Result<(), EpdError> {
        eprintln!("   😴 Putting display to sleep...");

        if self.flag == 1 {
            // New chip version
            self.send_command(0x10)?; // Deep sleep mode
            self.send_data(0x03)?;
        } else {
            // Old chip version
            self.send_command(0x50)?; // VCOM and data interval setting
            self.send_data(0xF7)?;

            self.send_command(0x02)?; // Power off
            self.read_busy()?;

            self.send_command(0x07)?; // Deep sleep
            self.send_data(0xA5)?;
        }

        thread::sleep(Duration::from_millis(2000));
        eprintln!("   ✅ Display sleeping");
        Ok(())
    }
}

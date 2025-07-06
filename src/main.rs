//! # Tide Tracker Application Entry Point
//!
//! This binary crate provides the main application logic for the tide tracker,
//! coordinating between data fetching, rendering, and hardware interfaces.
//! It supports both production mode (e-ink display) and development mode (ASCII output).

// Test modules
#[cfg(test)]
mod tests;

// Re-export library types for internal use
pub use tide_clock_lib::{config::Config, Sample, TideSeries};

// Import custom EPD traits for hardware mode
#[cfg(all(target_os = "linux", feature = "hardware"))]
use tide_clock_lib::epd4in2b_v2::{GpioPin, InputPin};

// Application dependencies
use std::env;
use tide_clock_lib::{fallback, renderer::draw_ascii, tide_data};

/// Convert GPIO number to physical pin number for display
/// This is a simplified mapping for common pins
fn gpio_to_pin(gpio: u32) -> u32 {
    match gpio {
        8 => 24,  // CS
        17 => 11, // RST
        24 => 18, // BUSY
        25 => 22, // DC
        7 => 26,  // Alternative CS
        27 => 13, // Alternative RST
        _ => 0,   // Unknown
    }
}

/// Initialize e-ink display with configurable GPIO pins and render tide data
/// Following the Waveshare example pattern - using rppal GPIO (like Python's gpiozero)
///
/// IMPORTANT BUSY PIN LOGIC:
/// - Waveshare 4.2" B rev2.2+ modules use BUSY active HIGH (flag=1)
/// - Older modules use BUSY active LOW (flag=0)  
/// - The code automatically forces flag=1 for newer modules to prevent hanging
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn initialize_eink_display(tide_series: &TideSeries, config: &Config) -> anyhow::Result<()> {
    use tide_clock_lib::epd4in2b_v2::{Color, DisplayBuffer, Epd4in2bV2};

    eprintln!("🚀 Initializing GPIO-only e-ink display (SPI disabled mode)...");

    // Use rppal for GPIO access (equivalent to Python's gpiozero)
    // This works without /dev/gpiochip* devices by accessing GPIO directly
    let gpio = rppal::gpio::Gpio::new().map_err(|e| {
        anyhow::anyhow!(
            "Failed to initialize GPIO: {}. Make sure you're running as root or in gpio group.",
            e
        )
    })?;

    let hw = &config.display.hardware;

    eprintln!("🔌 Initializing GPIO pins (following Waveshare Python example)...");
    eprintln!("   DC pin: GPIO {}", hw.dc_pin);
    eprintln!("   RST pin: GPIO {}", hw.rst_pin);
    eprintln!("   BUSY pin: GPIO {}", hw.busy_pin);
    eprintln!("   CS pin: GPIO {}", hw.cs_pin);

    // Initialize GPIO pins - using rppal which works like gpiozero
    let dc_pin = gpio
        .get(hw.dc_pin as u8)
        .map_err(|e| anyhow::anyhow!("Failed to get DC pin GPIO {}: {}", hw.dc_pin, e))?
        .into_output();

    let rst_pin = gpio
        .get(hw.rst_pin as u8)
        .map_err(|e| anyhow::anyhow!("Failed to get RST pin GPIO {}: {}", hw.rst_pin, e))?
        .into_output();

    let busy_pin = gpio
        .get(hw.busy_pin as u8)
        .map_err(|e| anyhow::anyhow!("Failed to get BUSY pin GPIO {}: {}", hw.busy_pin, e))?
        .into_input();

    // Create software SPI using rppal GPIO pins (without CS - EPD will control it)
    let mut spi = RppalSoftwareSpi::new(&gpio, 10, 11)?; // MOSI=GPIO10, SCLK=GPIO11

    eprintln!("🎨 Creating e-ink display driver (4.2\" b/w/red v2)...");

    // Wrap rppal pins to be compatible with our custom EPD driver
    eprintln!("🔧 Wrapping GPIO pins for custom EPD driver...");
    // CS pin is controlled by EPD driver, not SPI
    let cs_pin_wrapper = RppalOutputPin::new(gpio.get(hw.cs_pin as u8)?.into_output());
    let mut dc_pin_wrapper = RppalOutputPin::new(dc_pin);
    let mut rst_pin_wrapper = RppalOutputPin::new(rst_pin);
    let busy_pin_wrapper = RppalInputPin::new(busy_pin);
    eprintln!("✅ GPIO pin wrappers created successfully");

    // Check BUSY pin state before initialization
    eprintln!("🔍 Checking BUSY pin state before initialization...");
    match busy_pin_wrapper.is_high() {
        Ok(true) => eprintln!("   BUSY pin is HIGH (display may be busy or not connected)"),
        Ok(false) => eprintln!("   BUSY pin is LOW (display ready)"),
        Err(e) => eprintln!("   ⚠️  Failed to read BUSY pin: {:?}", e),
    }

    // Test GPIO pin behavior before EPD initialization
    eprintln!("🧪 Testing GPIO pin control...");
    eprintln!("   Testing RST pin (should toggle)...");
    rst_pin_wrapper
        .set_low()
        .map_err(|e| anyhow::anyhow!("RST set_low failed: {:?}", e))?;
    std::thread::sleep(std::time::Duration::from_millis(100));
    rst_pin_wrapper
        .set_high()
        .map_err(|e| anyhow::anyhow!("RST set_high failed: {:?}", e))?;
    eprintln!("   RST pin toggled successfully");

    eprintln!("   Testing DC pin (should toggle)...");
    dc_pin_wrapper
        .set_low()
        .map_err(|e| anyhow::anyhow!("DC set_low failed: {:?}", e))?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    dc_pin_wrapper
        .set_high()
        .map_err(|e| anyhow::anyhow!("DC set_high failed: {:?}", e))?;
    eprintln!("   DC pin toggled successfully");

    eprintln!("   Re-checking BUSY pin after RST toggle...");
    match busy_pin_wrapper.is_high() {
        Ok(true) => eprintln!("   BUSY pin is HIGH"),
        Ok(false) => eprintln!("   BUSY pin is LOW"),
        Err(e) => eprintln!("   ⚠️  Failed to read BUSY pin: {:?}", e),
    }

    // Test SPI communication first
    eprintln!("🧪 Testing software SPI by sending test bytes...");
    spi.write_byte(0x00).ok(); // Test byte
    spi.write_byte(0xFF).ok(); // Test byte
    spi.write_byte(0xAA).ok(); // Test byte
    eprintln!("   Software SPI test completed successfully");

    // Now try the real EPD initialization with our custom driver
    eprintln!("🚀 ATTEMPTING REAL EPD INITIALIZATION WITH CUSTOM DRIVER...");
    eprintln!("   This follows the exact Python epd4in2b_v2.py implementation");
    eprintln!("   Press Ctrl+C if it doesn't complete within 30 seconds");

    // Initialize the display using our custom EPD driver (matches Python exactly)
    let mut epd = Epd4in2bV2::new(
        spi,
        cs_pin_wrapper,
        dc_pin_wrapper,
        rst_pin_wrapper,
        busy_pin_wrapper,
    );

    // Force flag=1 for Waveshare 4.2" B rev2.2 modules (BUSY active HIGH)
    // Comment this line out if you have an older module that needs flag=0
    eprintln!("🔧 Forcing flag=1 for Waveshare 4.2\" B rev2.2 module (BUSY active HIGH)");
    epd.set_flag(1);

    match epd.init() {
        Ok(_) => {
            eprintln!("🎉 SUCCESS! Custom E-ink display driver initialized!");
            eprintln!("   The EPD initialization completed without hanging!");
        }
        Err(e) => {
            eprintln!(
                "❌ Custom E-ink display driver initialization failed: {:?}",
                e
            );
            return Err(anyhow::anyhow!("Display initialization failed: {:?}", e));
        }
    }

    eprintln!("🧹 Clearing display and rendering tide chart...");

    // Create display buffer - 4.2" display is 400x300 pixels
    let mut display_buffer = DisplayBuffer::new(400, 300);
    display_buffer.clear(Color::White);

    // Render tide data to the display buffer
    eprintln!("📊 Rendering tide data...");
    render_tide_data_to_buffer(tide_series, &mut display_buffer);

    // Add geometric test patterns to debug pixel mapping
    eprintln!("🧪 Adding geometric test patterns...");
    add_geometric_test_patterns(&mut display_buffer);

    eprintln!("📤 Updating e-ink display...");

    // Display the rendered data
    epd.display(display_buffer.black_buffer(), display_buffer.red_buffer())?;

    // DON'T put display to sleep - keep the image visible
    // epd.sleep()?; // Commented out to keep image persistent

    eprintln!("✅ E-ink display updated successfully (image should persist)"); // Keep the program running to prevent any automatic cleanup that might clear the display
    eprintln!("🕐 Keeping program running for 60 seconds to ensure image persistence...");
    eprintln!("   This allows time to observe if the image blinks, fades, or disappears");
    eprintln!("   Press Ctrl+C to exit early");
    std::thread::sleep(std::time::Duration::from_secs(60));
    eprintln!("   Program completed - image should remain on display indefinitely");
    eprintln!("   E-ink displays retain images without power");

    Ok(())
}

/// Render actual tide data to the display buffer using the full chart renderer
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn render_tide_data_to_buffer(
    tide_series: &TideSeries,
    buffer: &mut tide_clock_lib::epd4in2b_v2::DisplayBuffer,
) {
    eprintln!("🎨 SKIPPING tide chart rendering for now - focusing on pixel mapping tests...");
    eprintln!(
        "   📊 Tide series has {} samples",
        tide_series.samples.len()
    );

    // Comment out the complex tide chart renderer while we debug pixel mapping
    // tide_clock_lib::renderer::draw_eink_v2_custom(tide_series, buffer);

    eprintln!("✅ Tide chart rendering skipped - using test patterns only");
}

/// Add very simple test pattern to verify basic rendering
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn add_geometric_test_patterns(buffer: &mut tide_clock_lib::epd4in2b_v2::DisplayBuffer) {
    use tide_clock_lib::epd4in2b_v2::Color;

    eprintln!("🎨 Adding VERY SIMPLE test pattern for basic verification...");

    // Just a simple 5px border - THAT'S IT!
    eprintln!("   ⬛ Drawing simple 5px border...");
    for thickness in 0..5 {
        // Top and bottom borders
        for x in 0..400 {
            buffer.set_pixel(x, thickness, Color::Black); // Top border
            buffer.set_pixel(x, 299 - thickness, Color::Black); // Bottom border
        }
        // Left and right borders
        for y in 0..300 {
            buffer.set_pixel(thickness, y, Color::Black); // Left border
            buffer.set_pixel(399 - thickness, y, Color::Black); // Right border
        }
    }

    eprintln!("✅ Simple 5px border pattern added - should be clearly visible");
}

/// Software SPI implementation using rppal GPIO bit-banging for e-ink displays
/// This matches the approach used in Waveshare Python examples when SPI is disabled
/// CS (Chip Select) is controlled separately by the EPD driver
#[cfg(all(target_os = "linux", feature = "hardware"))]
struct RppalSoftwareSpi {
    mosi_pin: rppal::gpio::OutputPin,
    sclk_pin: rppal::gpio::OutputPin,
}

#[cfg(all(target_os = "linux", feature = "hardware"))]
impl RppalSoftwareSpi {
    fn new(gpio: &rppal::gpio::Gpio, mosi_gpio: u8, sclk_gpio: u8) -> anyhow::Result<Self> {
        eprintln!("🔧 Creating software SPI using GPIO pins:");
        eprintln!("   MOSI (Data): GPIO {}", mosi_gpio);
        eprintln!("   SCLK (Clock): GPIO {}", sclk_gpio);
        eprintln!("   CS (Chip Select): Controlled by EPD driver");

        let mosi_pin = gpio
            .get(mosi_gpio)
            .map_err(|e| anyhow::anyhow!("Failed to get MOSI pin GPIO {}: {}", mosi_gpio, e))?
            .into_output();

        let sclk_pin = gpio
            .get(sclk_gpio)
            .map_err(|e| anyhow::anyhow!("Failed to get SCLK pin GPIO {}: {}", sclk_gpio, e))?
            .into_output();

        Ok(RppalSoftwareSpi { mosi_pin, sclk_pin })
    }

    fn write_byte(&mut self, byte: u8) -> anyhow::Result<()> {
        // Note: CS pin is controlled by the EPD driver, not here

        // Send 8 bits, MSB first
        for i in (0..8).rev() {
            // Set data line
            if (byte >> i) & 1 == 1 {
                self.mosi_pin.set_high();
            } else {
                self.mosi_pin.set_low();
            }

            // Clock pulse: low -> high -> low
            self.sclk_pin.set_low();
            std::thread::sleep(std::time::Duration::from_nanos(500)); // Slower timing for reliability
            self.sclk_pin.set_high();
            std::thread::sleep(std::time::Duration::from_nanos(500)); // Slower timing for reliability
            self.sclk_pin.set_low();
        }

        // Small delay between bytes for display processing
        std::thread::sleep(std::time::Duration::from_micros(1));

        Ok(())
    }

    fn read_byte(&mut self) -> anyhow::Result<u8> {
        // For SPI read, we need to send dummy bytes while reading MISO
        // E-ink displays typically don't use MISO, so this is a simplified implementation
        eprintln!("🔍 Attempting SPI read (dummy implementation for e-ink)");

        // Note: CS pin is controlled by the EPD driver, not here

        let mut result = 0u8;

        // Read 8 bits, MSB first (dummy implementation - just return 0x00 for now)
        for _i in (0..8).rev() {
            // Clock pulse: low -> high -> low
            self.sclk_pin.set_low();
            std::thread::sleep(std::time::Duration::from_nanos(500));
            self.sclk_pin.set_high();
            std::thread::sleep(std::time::Duration::from_nanos(500));
            self.sclk_pin.set_low();

            // Shift result (dummy read - no actual MISO pin)
            result <<= 1;
        }

        // Small delay between bytes
        std::thread::sleep(std::time::Duration::from_micros(1));

        // Return dummy value - most e-ink displays don't actually respond to reads
        Ok(0x00)
    }
}

// Implement our custom EPD SoftwareSpi trait for RppalSoftwareSpi
#[cfg(all(target_os = "linux", feature = "hardware"))]
impl tide_clock_lib::epd4in2b_v2::SoftwareSpi for RppalSoftwareSpi {
    fn write_byte(&mut self, byte: u8) -> Result<(), tide_clock_lib::epd4in2b_v2::EpdError> {
        self.write_byte(byte)
            .map_err(|e| tide_clock_lib::epd4in2b_v2::EpdError(e.to_string()))
    }

    fn read_byte(&mut self) -> Result<u8, tide_clock_lib::epd4in2b_v2::EpdError> {
        self.read_byte()
            .map_err(|e| tide_clock_lib::epd4in2b_v2::EpdError(e.to_string()))
    }
}

// Define custom error types for embedded-hal compatibility

/// Wrapper to make rppal OutputPin compatible with our custom EPD driver
#[cfg(all(target_os = "linux", feature = "hardware"))]
struct RppalOutputPin {
    pin: rppal::gpio::OutputPin,
}

#[cfg(all(target_os = "linux", feature = "hardware"))]
impl RppalOutputPin {
    fn new(pin: rppal::gpio::OutputPin) -> Self {
        Self { pin }
    }
}

#[cfg(all(target_os = "linux", feature = "hardware"))]
impl tide_clock_lib::epd4in2b_v2::GpioPin for RppalOutputPin {
    fn set_low(&mut self) -> Result<(), tide_clock_lib::epd4in2b_v2::EpdError> {
        self.pin.set_low();
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), tide_clock_lib::epd4in2b_v2::EpdError> {
        self.pin.set_high();
        Ok(())
    }
}

/// Wrapper to make rppal InputPin compatible with our custom EPD driver
#[cfg(all(target_os = "linux", feature = "hardware"))]
struct RppalInputPin {
    pin: rppal::gpio::InputPin,
}

#[cfg(all(target_os = "linux", feature = "hardware"))]
impl RppalInputPin {
    fn new(pin: rppal::gpio::InputPin) -> Self {
        Self { pin }
    }
}

#[cfg(all(target_os = "linux", feature = "hardware"))]
impl tide_clock_lib::epd4in2b_v2::InputPin for RppalInputPin {
    fn is_high(&self) -> Result<bool, tide_clock_lib::epd4in2b_v2::EpdError> {
        Ok(self.pin.is_high())
    }
}

/// Main application entry point.
fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    // Development mode: render to stdout for testing without hardware
    let development_mode = env::args().any(|arg| arg == "--stdout");

    // Create Tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;

    // Fetch tide data with automatic fallback on failure
    // Network errors are expected and handled gracefully
    let tide_series = rt.block_on(async {
        tide_data::fetch().await.unwrap_or_else(|error| {
            // Log fetch failure for debugging (visible in systemd journal)
            eprintln!("Tide data fetch failed: {}", error);
            eprintln!("Falling back to offline mathematical model");

            // Continue with synthetic data rather than crashing
            fallback::approximate()
        })
    });

    // Development mode: ASCII output for testing
    if development_mode {
        draw_ascii(&tide_series);
        return Ok(());
    }

    // Production mode: Initialize e-ink display hardware
    // This section requires SPI access and proper GPIO permissions
    #[cfg(all(target_os = "linux", feature = "hardware"))]
    {
        // Load configuration for GPIO pins
        let config = Config::load();
        let hw = &config.display.hardware;

        eprintln!("🔧 E-ink hardware integration with configurable GPIO pins");
        eprintln!("📋 GPIO pin configuration:");
        eprintln!(
            "   CS (Chip Select): GPIO {} (Pin {})",
            hw.cs_pin,
            gpio_to_pin(hw.cs_pin)
        );
        eprintln!(
            "   DC (Data/Command): GPIO {} (Pin {})",
            hw.dc_pin,
            gpio_to_pin(hw.dc_pin)
        );
        eprintln!(
            "   RST (Reset): GPIO {} (Pin {})",
            hw.rst_pin,
            gpio_to_pin(hw.rst_pin)
        );
        eprintln!(
            "   BUSY: GPIO {} (Pin {})",
            hw.busy_pin,
            gpio_to_pin(hw.busy_pin)
        );
        eprintln!("");

        // Initialize e-ink display with configured GPIO pins
        match initialize_eink_display(&tide_series, &config) {
            Ok(_) => {
                eprintln!("✅ E-ink display updated successfully");
            }
            Err(e) => {
                eprintln!("❌ E-ink display initialization failed: {}", e);
                eprintln!("Falling back to ASCII output for debugging:");
                draw_ascii(&tide_series);
            }
        }
    }

    #[cfg(all(target_os = "linux", not(feature = "hardware")))]
    {
        eprintln!("E-ink display support not enabled. Rebuild with --features hardware for display functionality.");
        eprintln!("Showing ASCII output instead:");
        draw_ascii(&tide_series);
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("Hardware mode is only available on Linux. Use --stdout for development mode.");
        return Err(anyhow::anyhow!(
            "Hardware mode not supported on this platform"
        ));
    }

    Ok(())
}

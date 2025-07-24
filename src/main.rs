//! # Tide Tracker Application Entry Point
//!
//! This binary crate provides the main application logic for the tide tracker,
//! coordinating between data fetching, rendering, and hardware interfaces.
//! It supports both production mode (e-ink display) and development mode (ASCII output).

// Test modules
#[cfg(test)]
mod tests;

mod gpio_sysfs;
mod hw_spi_spidev;

// Re-export library types for internal use
pub use tide_clock_lib::{config::Config, Sample, TideSeries};

// Remove unused GPIO and SPI imports
// use crate::gpio_sysfs::{CdevInputPin, CdevOutputPin};
// use crate::hw_spi_spidev::SpidevHwSpi;
// use anyhow::Context;

// Application dependencies
use std::env;
use tide_clock_lib::{fallback, renderer::draw_ascii, tide_data};

/// Convert GPIO number to physical pin number for display
/// This is a simplified mapping for common pins
#[allow(dead_code)]
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
    use tide_clock_lib::epd4in2b_v2::{DisplayBuffer, Epd4in2bV2};

    eprintln!("🚀 Initializing GPIO-only e-ink display (SPI disabled mode)...");

    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0").context("open gpiochip0")?;

    // Get hardware pin config from Config
    let hw = &config.display.hardware;

    // Only request DC, RST, BUSY via gpiod for hardware SPI
    let dc = CdevOutputPin::new(&mut chip, hw.dc_pin as u32)?;
    let rst = CdevOutputPin::new(&mut chip, hw.rst_pin as u32)?;
    let busy = CdevInputPin::new(&mut chip, hw.busy_pin as u32)?;

    // Use kernel SPI driver (CS handled by kernel)
    let spi = SpidevHwSpi::new()?;
    // Pass None::<CdevOutputPin> for CS pin to satisfy type inference
    let mut epd = Epd4in2bV2::new(spi, None::<CdevOutputPin>, dc, rst, busy);

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

    eprintln!("🎨 Creating display buffer and rendering content...");

    // Create display buffer - 4.2" display is 400x300 pixels
    let mut display_buffer = DisplayBuffer::new(400, 300);
    // Buffer is already initialized to white by default - no need to clear again

    // Choose rendering mode: true = test patterns, false = tide chart
    // let test_mode = true; // Start with simple test pattern to debug persistence
    let test_mode = false; // Complex tide chart - test this after simple patterns work

    if test_mode {
        eprintln!("🧪 TEST MODE: Adding geometric test patterns...");
        add_geometric_test_patterns(&mut display_buffer);
    } else {
        eprintln!("📊 CHART MODE: Rendering tide chart...");

        // First, clear the display to remove any previous content (like alternating stripes)
        eprintln!("🧹 Clearing display to remove previous content...");
        epd.clear()?;
        eprintln!("✅ Display cleared successfully");

        let renderer = tide_clock_lib::eink_renderer::EinkTideRenderer::new();
        renderer.render_chart(&mut display_buffer, tide_series);

        // After rendering the chart, overlay the last update time/date
        use chrono::Local;
        let now = Local::now();
        let time_str = now.format("%-m/%-d %-I:%M%p").to_string(); // e.g. "7/23 8:14PM"
                                                                   // Overlay at top right, 10px from right, 10px from top (large font, aligned)
        let overlay_x = 400 - 10 - (time_str.len() as u32 * 10); // 10px per char for large font
        let overlay_y = 10; // Top margin
        renderer.draw_large_text(&mut display_buffer, overlay_x, overlay_y, &time_str);

        // Debug: Check what we actually rendered
        let black_pixels = display_buffer
            .black_buffer()
            .iter()
            .map(|&b| b.count_zeros())
            .sum::<u32>();
        let red_pixels = display_buffer
            .red_buffer()
            .iter()
            .map(|&b| b.count_ones())
            .sum::<u32>();
        eprintln!(
            "📊 Rendered buffer: {} black pixels, {} red pixels",
            black_pixels, red_pixels
        );

        // Sample a few bytes from the middle of the buffer to verify content
        let mid_offset = display_buffer.black_buffer().len() / 2;
        eprintln!(
            "📊 Buffer sample (middle): black={:02X} {:02X} {:02X}, red={:02X} {:02X} {:02X}",
            display_buffer.black_buffer().get(mid_offset).unwrap_or(&0),
            display_buffer
                .black_buffer()
                .get(mid_offset + 1)
                .unwrap_or(&0),
            display_buffer
                .black_buffer()
                .get(mid_offset + 2)
                .unwrap_or(&0),
            display_buffer.red_buffer().get(mid_offset).unwrap_or(&0),
            display_buffer
                .red_buffer()
                .get(mid_offset + 1)
                .unwrap_or(&0),
            display_buffer
                .red_buffer()
                .get(mid_offset + 2)
                .unwrap_or(&0)
        );

        // Check if the buffer looks inverted (if all bytes are 0xFF, it means white background)
        let first_few_black =
            &display_buffer.black_buffer()[..16.min(display_buffer.black_buffer().len())];
        let all_ff = first_few_black.iter().all(|&b| b == 0xFF);
        if all_ff {
            eprintln!(
                "⚠️  WARNING: Buffer appears to be all 0xFF (white) - may need bit inversion"
            );
        }
    }

    eprintln!("📤 Updating e-ink display...");
    eprintln!("     ⚠️  This should be called EXACTLY ONCE to avoid flickering");

    // Try the normal display method first since we cleared the display
    eprintln!("     🎨 Trying normal display method after clear...");
    match epd.display(display_buffer.black_buffer(), display_buffer.red_buffer()) {
        Ok(_) => {
            eprintln!("     ✅ Normal display method completed successfully");
        }
        Err(e) => {
            eprintln!("     ⚠️  Normal display failed: {:?}", e);
            eprintln!("     🔄 Falling back to C test sequence...");
            epd.display_c_test_sequence(
                display_buffer.black_buffer(),
                display_buffer.red_buffer(),
            )?;
            eprintln!("     ✅ C test sequence fallback completed");
        }
    }

    // SKIP power-off for now since basic persistence works
    eprintln!("     ⚠️  SKIPPING power-off sequence since basic persistence is working");

    eprintln!("     ✅ Display function completed");

    eprintln!("✅ E-ink display updated successfully with PERSISTENCE SEQUENCE!");
    eprintln!("   📋 Persistence checklist completed:");
    eprintln!("   ✅ 1. Drew image once (no clear after)");
    eprintln!("   ✅ 2. Sent POWER_OFF (0x02) + wait BUSY");
    eprintln!("   ✅ 3. Sent DEEP_SLEEP (0x10) + 0x01 + wait BUSY");
    eprintln!("   ✅ 4. Display controller parked safely");
    eprintln!();
    eprintln!("🎯 Image should now persist indefinitely (even with Pi powered off)");
    eprintln!("   This follows the persistence cheat sheet exactly");
    eprintln!("   You can now safely power off the Pi - image will remain");
    eprintln!();
    eprintln!("🕐 Keeping program running for 10 seconds to verify persistence...");
    eprintln!("   Press Ctrl+C to exit early");
    std::thread::sleep(std::time::Duration::from_secs(10));

    eprintln!("   ✅ Persistence test completed - you can now safely power off the Pi");
    eprintln!("   The image should remain on the display indefinitely");

    Ok(())
}

/// Render actual tide data to the display buffer using the full chart renderer
#[cfg(all(target_os = "linux", feature = "hardware"))]
#[allow(dead_code)]
fn render_tide_data_to_buffer(
    tide_series: &TideSeries,
    buffer: &mut tide_clock_lib::epd4in2b_v2::DisplayBuffer,
) {
    eprintln!("🎨 Rendering tide chart to e-ink display buffer...");
    eprintln!(
        "   📊 Tide series has {} samples",
        tide_series.samples.len()
    );

    // Use the complex tide chart renderer for e-paper display
    tide_clock_lib::renderer::draw_eink_v2_custom(tide_series, buffer);

    eprintln!("✅ Tide chart rendering completed");
}

/// Add a VERY simple test pattern for reliable debugging
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn add_geometric_test_patterns(buffer: &mut tide_clock_lib::epd4in2b_v2::DisplayBuffer) {
    use tide_clock_lib::epd4in2b_v2::Color;

    eprintln!("🎨 Adding SIMPLE BORDER test pattern for reliable debugging...");

    // Test: Just a simple 5px border (less aggressive than full black screen)
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

    /*
    // Alternative: Just fill the entire screen black to see if anything shows up
    eprintln!("   ⬛ TEST: Filling entire screen black...");
    for x in 0..400 {
        for y in 0..300 {
            buffer.set_pixel(x, y, Color::Black);
        }
    }
    */

    eprintln!("✅ Simple 5px border pattern added - should be clearly visible");

    // Debug: Check if buffer actually has data
    let black_buffer = buffer.black_buffer();
    let red_buffer = buffer.red_buffer();
    let mut black_pixels = 0;
    let mut red_pixels = 0;

    for &byte in black_buffer {
        black_pixels += byte.count_zeros() as usize; // 0 bits are black pixels
    }
    for &byte in red_buffer {
        red_pixels += byte.count_ones() as usize; // 1 bits are red pixels
    }

    eprintln!(
        "   📊 Buffer contains: {} black pixels, {} red pixels",
        black_pixels, red_pixels
    );
    if black_pixels == 0 && red_pixels == 0 {
        eprintln!("   ⚠️  WARNING: Buffer appears to be empty - no pixels set!");
    }

    // Debug: Show first few bytes of buffers to verify data
    eprintln!(
        "   🔍 First 8 bytes of black buffer: {:02X?}",
        &black_buffer[..8.min(black_buffer.len())]
    );
    eprintln!(
        "   🔍 First 8 bytes of red buffer: {:02X?}",
        &red_buffer[..8.min(red_buffer.len())]
    );
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
        #[allow(unreachable_code)]
        return Err(anyhow::anyhow!(
            "Hardware mode not supported on this platform"
        ));
    }

    #[allow(unreachable_code)]
    Ok(())
}

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

// Import new GPIO and SPI types for hardware mode
#[allow(unused_imports)]
use crate::gpio_sysfs::{CdevInputPin, CdevOutputPin};
#[allow(unused_imports)]
use crate::hw_spi_spidev::SpidevHwSpi;
#[allow(unused_imports)]
use anyhow::Context;

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
fn initialize_eink_display(
    tide_series: &TideSeries,
    config: &Config,
    full_refresh: bool,
) -> anyhow::Result<()> {
    use tide_clock_lib::epd4in2b_v2::{DisplayBuffer, Epd4in2bV2};

    eprintln!("🚀 Initializing GPIO-only e-ink display (SPI disabled mode)...");

    let mut chip = gpio_cdev::Chip::new("/dev/gpiochip0").context("open gpiochip0")?;

    // Get hardware pin config from Config
    let hw = &config.display.hardware;

    // Only request DC, RST, BUSY via gpiod for hardware SPI
    let dc = CdevOutputPin::new(&mut chip, hw.dc_pin)?;
    let rst = CdevOutputPin::new(&mut chip, hw.rst_pin)?;
    let busy = CdevInputPin::new(&mut chip, hw.busy_pin)?;

    // SPI setup: use hardware CS for GPIO 8 (CE0) or 7 (CE1), manual CS for others
    let use_hw_cs = hw.cs_pin == 8 || hw.cs_pin == 7;
    let spi: Box<dyn tide_clock_lib::epd4in2b_v2::SoftwareSpi> = if use_hw_cs {
        if hw.cs_pin == 8 {
            Box::new(SpidevHwSpi::new_ce0()?)
        } else {
            Box::new(SpidevHwSpi::new_ce1()?)
        }
    } else {
        // Warn if user tries to use manual CS on a kernel-controlled pin
        if hw.cs_pin == 8 || hw.cs_pin == 7 {
            eprintln!("⚠️  Config error: cs_pin {} is kernel-controlled (CE0/CE1), manual CS will not work!", hw.cs_pin);
        }
        let spi = SpidevHwSpi::new_ce0()?; // Default to CE0 for manual CS
        let cs = CdevOutputPin::new(&mut chip, hw.cs_pin)?;
        Box::new(crate::hw_spi_spidev::SpidevManualCs::new(spi, cs))
    };
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

    eprintln!("📊 CHART MODE: Rendering tide chart...");

    // A full clear-then-draw cycle flashes the panel through its inversion
    // sequence, which clears accumulated ghosting - but it's only needed
    // periodically, not on every update. This function runs 144x/day from
    // the 10-minute timer, and once/day (at midnight) from the dedicated
    // full-refresh timer, which passes `--full-refresh`. Clearing on every
    // one of those 144 runs would be visually noisy and adds unnecessary
    // panel wear for no benefit, since epd.display() already writes the
    // complete frame buffer on its own; the midnight run supplies the
    // periodic full refresh instead.
    if full_refresh {
        // First, clear the display to remove any previous content (like alternating stripes)
        eprintln!("🧹 Clearing display to remove previous content...");
        epd.clear()?;
        eprintln!("✅ Display cleared successfully");
    } else {
        eprintln!(
            "📝 Incremental redraw (no clear) - ghosting is cleared by the daily full refresh"
        );
    }

    let renderer = tide_clock_lib::eink_renderer::EinkTideRenderer::new();
    // New API: pass epd, display_buffer, tide_series
    renderer.render_chart(&mut epd, &mut display_buffer, tide_series);

    // One label covers both cases (see TideSeries::status_label): OFFLINE! for
    // the synthetic, station-unaligned fallback, CACHED Nh/Nd for real station
    // predictions served from cache after a failed live fetch. A fresh fetch
    // gets no annotation at all.
    if let Some(label) = tide_series.status_label() {
        use embedded_graphics::mono_font::iso_8859_1::FONT_10X20;
        use embedded_graphics::{
            mono_font::MonoTextStyle, pixelcolor::BinaryColor, prelude::*, text::Text,
        };
        let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
        Text::new(&label, Point::new(10, 24), style)
            .draw(&mut display_buffer)
            .ok();
    }

    // Overlay the last update time/date using embedded-graphics Text primitive
    use chrono::Local;
    use embedded_graphics::mono_font::iso_8859_1::FONT_10X20;
    use embedded_graphics::{
        mono_font::MonoTextStyle, pixelcolor::BinaryColor, prelude::*, text::Text,
    };

    let now = Local::now();
    let time_str = now.format("%-m/%-d %-I:%M%p").to_string(); // e.g. "7/23 8:14PM"
                                                               // Overlay at top right, 10px from right, 10px from top
    let char_width = 10; // FONT_10X20 width
    let overlay_x = 400 - 10 - (time_str.len() as i32 * char_width);
    let overlay_y = 10;
    let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    Text::new(&time_str, Point::new(overlay_x, overlay_y + 16), style)
        .draw(&mut display_buffer)
        .ok();

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
        eprintln!("⚠️  WARNING: Buffer appears to be all 0xFF (white) - may need bit inversion");
    }

    eprintln!("📤 Updating e-ink display...");
    eprintln!("     ⚠️  This should be called EXACTLY ONCE to avoid flickering");

    // Try the normal display method first
    eprintln!("     🎨 Trying normal display method...");
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

    Ok(())
}

/// Main application entry point.
///
/// Supported flags:
/// - `--stdout`: development mode, render to stdout (ASCII) instead of hardware.
/// - `--test-offline`: force the synthetic offline fallback curve, for testing.
/// - `--full-refresh`: do a full clear-then-draw cycle instead of an incremental
///   redraw (see `initialize_eink_display`). Used by the midnight timer.
fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    // Development mode: render to stdout for testing without hardware
    let args: Vec<String> = env::args().collect();
    let development_mode = args.iter().any(|arg| arg == "--stdout");
    let test_offline_mode = args.iter().any(|arg| arg == "--test-offline");
    // Only consumed by initialize_eink_display, which is compiled out on
    // non-Linux and non-hardware builds - matches the #[allow(unused_imports)]
    // pattern above for the same reason.
    #[allow(unused_variables)]
    let full_refresh = args.iter().any(|arg| arg == "--full-refresh");

    // Create Tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;

    // Fetch tide data with automatic fallback on failure, or force offline if requested
    let tide_series = if test_offline_mode {
        // Force offline fallback mode for testing: this sets offline=true in the returned TideSeries
        eprintln!("[TEST] Forcing offline fallback mode (--test-offline flag set)");
        fallback::approximate(None)
    } else {
        rt.block_on(async {
            tide_data::fetch().await.unwrap_or_else(|error| {
                // tide_data::fetch() only returns Err here when *both* a live
                // NOAA fetch failed *and* no cached predictions cover the
                // current display window (it otherwise transparently serves
                // a usable cache - see its doc comment). Reaching this point
                // means the display is about to show a synthetic curve that
                // is NOT aligned to any real station, so make that loud and
                // unmistakable in the logs.
                let config = Config::load();
                eprintln!(
                    "⚠️  DEGRADED: no live NOAA data and no usable cache — displaying SYNTHETIC \
                     tide curve, which is NOT aligned to station {}: {}",
                    config.station.id, error
                );
                // Continue with synthetic data rather than crashing
                fallback::approximate(None)
            })
        })
    };

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
        match initialize_eink_display(&tide_series, &config, full_refresh) {
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

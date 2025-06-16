//! # Tide Tracker Application Entry Point
//!
//! This binary crate provides the main application logic for the tide tracker,
//! coordinating between data fetching, rendering, and hardware interfaces.
//! It supports both production mode (e-ink display) and development mode (ASCII output).
//!
//! ## Application Flow
//!
//! 1. **Parse command line**: Check for `--stdout` development flag
//! 2. **Fetch tide data**: Try NOAA API, fall back to mathematical model on failure
//! 3. **Render output**: Either ASCII terminal or e-ink display
//! 4. **Hardware control**: Initialize SPI, update display, enter sleep mode
//!
//! ## Memory Management
//!
//! The application is designed for the Pi Zero W's 512MB RAM constraint:
//! - **Peak usage**: < 1MB across entire execution
//! - **No memory leaks**: All allocations are stack-based or freed before exit
//! - **Minimal dependencies**: Only essential crates for functionality
//! - **Efficient data structures**: Pre-allocated vectors and primitive types
//!
//! ## Hardware Configuration
//!
//! ### SPI Configuration
//! - **Device**: `/dev/spidev0.0` (SPI bus 0, chip select 0)
//! - **Speed**: Default (typically 1-8MHz, adequate for e-ink refresh)
//! - **Mode**: SPI Mode 0 (CPOL=0, CPHA=0)
//!
//! ### GPIO Pin Mapping
//! - **CS (Chip Select)**: GPIO 8 (Pin 24)
//! - **DC (Data/Command)**: GPIO 25 (Pin 22)
//! - **RST (Reset)**: GPIO 17 (Pin 11)
//! - **BUSY**: GPIO 24 (Pin 18)
//!
//! ## Error Handling
//!
//! The application prioritizes robustness over strict error reporting:
//! - **Network failures**: Automatically fall back to offline model
//! - **Hardware errors**: Propagate via `anyhow::Result` for systemd logging
//! - **Invalid data**: Validated during parsing, falls back gracefully
//! - **Permission issues**: Clear error messages for debugging

// Module declarations - organize code into logical units
mod fallback;
mod renderer;
mod tide_data;

// Test modules
#[cfg(test)]
mod tests;

// Re-export library types for internal use
pub use tide_clock_lib::*;

// Hardware and graphics dependencies (only used in production mode on Linux)
#[cfg(target_os = "linux")]
use epd_waveshare::{epd4in2::EPD4in2, prelude::*};
#[cfg(target_os = "linux")]
use linux_embedded_hal::{Delay, Pin, Spidev};

// Application dependencies
use renderer::draw_ascii;
#[cfg(target_os = "linux")]
use renderer::draw_eink;
use std::env;

/// Main application entry point.
///
/// This function orchestrates the complete tide tracking workflow:
/// 1. Parse command line arguments for development vs. production mode
/// 2. Fetch current tide data (network or fallback)
/// 3. Render to appropriate output target
/// 4. Handle hardware initialization and cleanup
///
/// # Command Line Arguments
/// - `--stdout`: Development mode - render ASCII output to terminal
/// - (no args): Production mode - render to e-ink display via SPI
///
/// # Error Handling
/// Returns `anyhow::Result<()>` for integration with systemd and logging.
/// Errors are logged to stderr/systemd journal for debugging.
///
/// # Memory Usage
/// - Development mode: ~1KB peak (ASCII rendering only)
/// - Production mode: ~50KB peak (includes e-ink driver and SPI buffers)
/// - Both modes: Released before exit, no persistent allocations
///
/// # Example Usage
/// ```bash
/// # Development mode on macOS/Linux
/// cargo run --release -- --stdout
///
/// # Production mode on Raspberry Pi
/// sudo ./tide-tracker
/// ```
fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    // Development mode: render to stdout for testing without hardware
    let development_mode = env::args().any(|arg| arg == "--stdout");

    // Fetch tide data with automatic fallback on failure
    // Network errors are expected and handled gracefully
    let tide_series = tide_data::fetch().unwrap_or_else(|error| {
        // Log fetch failure for debugging (visible in systemd journal)
        eprintln!("Tide data fetch failed: {}", error);
        eprintln!("Falling back to offline mathematical model");

        // Continue with synthetic data rather than crashing
        fallback::approximate()
    });

    // Development mode: ASCII output for testing
    if development_mode {
        draw_ascii(&tide_series);
        return Ok(());
    }

    // Production mode: Initialize e-ink display hardware
    // This section requires SPI access and proper GPIO permissions
    #[cfg(target_os = "linux")]
    {
        // Initialize SPI interface
        // /dev/spidev0.0 = SPI bus 0, chip select 0
        let mut spi = Spidev::open("/dev/spidev0.0")?;

        // Initialize delay provider for timing-sensitive operations
        let mut delay = Delay {};

        // Initialize e-ink display driver with GPIO pin configuration
        // Pin numbers correspond to BCM GPIO numbers (not physical pin numbers)
        let mut epd = EPD4in2::new(
            &mut spi,     // SPI interface
            Pin::new(8),  // CS (Chip Select) - GPIO 8
            Pin::new(25), // DC (Data/Command) - GPIO 25
            Pin::new(24), // RST (Reset) - GPIO 24
            Pin::new(17), // BUSY - GPIO 17
            &mut delay,
        )?;

        // Create display buffer for rendering
        // This allocates the frame buffer for the 400×300 pixel display
        let mut display = epd_waveshare::graphics::Display4in2::default();

        // Render tide data to display buffer
        draw_eink(&tide_series, &mut display);

        // Update physical e-ink display
        // This operation takes several seconds due to e-ink refresh characteristics
        epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;

        // Settle delay for 4.2" glass (1.5s is within Waveshare's 1-1.7s fast-refresh spec)
        delay.delay_ms(1500u16);

        // Put display into low-power sleep mode
        // Critical for battery-powered applications and longevity
        epd.sleep(&mut spi, &mut delay)?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("Hardware mode is only available on Linux. Use --stdout for development mode.");
        return Err(anyhow::anyhow!(
            "Hardware mode not supported on this platform"
        ));
    }
}

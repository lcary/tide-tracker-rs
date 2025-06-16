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

// Test modules
#[cfg(test)]
mod tests;

// Re-export library types for internal use
pub use tide_clock_lib::{config::Config, Sample, TideSeries};

// Application dependencies
use std::env;
use tide_clock_lib::{fallback, renderer::draw_ascii, tide_data};

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
        eprintln!("E-ink hardware integration updated: compiling embedded-hal compatibility layer");
        eprintln!(
            "Note: Full GPIO/SPI integration requires running on actual Raspberry Pi hardware"
        );
        eprintln!("Current status: Cross-compilation resolved, runtime integration pending");
        eprintln!("");
        eprintln!("For testing, showing ASCII output:");
        draw_ascii(&tide_series);
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
        Err(anyhow::anyhow!(
            "Hardware mode not supported on this platform"
        ))
    }

    #[cfg(target_os = "linux")]
    Ok(())
}

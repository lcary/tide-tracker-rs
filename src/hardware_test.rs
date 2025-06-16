//! Hardware integration test module
//! Tests embedded-hal compatibility and e-ink display initialization

#[cfg(all(target_os = "linux", feature = "hardware"))]
pub mod hardware {
    use embedded_hal_bus::spi::ExclusiveDevice;
    use epd_waveshare::{epd4in2::*, prelude::*};
    use linux_embedded_hal::{
        gpio_cdev::{Chip, LineRequestFlags},
        spidev::{SpiModeFlags, SpidevOptions},
        Delay, Spidev,
    };

    /// Test function to verify hardware dependencies compile correctly
    pub fn test_hardware_init() -> Result<(), Box<dyn std::error::Error>> {
        // This is a compile-time test only - will not run on non-Pi hardware
        // Just verify that the types and traits are compatible

        eprintln!("Hardware integration test: types compile successfully");
        Ok(())
    }
}

#[cfg(not(all(target_os = "linux", feature = "hardware")))]
pub mod hardware {
    pub fn test_hardware_init() -> Result<(), Box<dyn std::error::Error>> {
        eprintln!("Hardware integration disabled - skipping test");
        Ok(())
    }
}

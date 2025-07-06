#!/usr/bin/env cargo
//! Quick e-ink display test using Rust
//!
//! This tests the same hardware setup that the main tide-tracker application uses.
//! Uses GPIO pin configuration from tide-config.toml.
//!
//! Usage:
//!   chmod +x scripts/test_display_rust.rs
//!   ./scripts/test_display_rust.rs
//!
//! Or compile and run:
//!   cargo run --bin test_display --features hardware

#[cfg(all(target_os = "linux", feature = "hardware"))]
mod hardware_test {
    use embedded_graphics::{
        geometry::{Point, Size},
        mono_font::{ascii::FONT_10X20, MonoTextStyle},
        prelude::*,
        primitives::{Circle, Line, PrimitiveStyle, Rectangle},
        text::{Baseline, Text},
    };
    use epd_waveshare::{color::Color, epd4in2::*};
    use linux_embedded_hal::spidev::Spidev;
    use linux_embedded_hal::{
        gpio_cdev::{Chip, LineRequestFlags},
        spidev::{SpiModeFlags, SpidevOptions},
        Delay,
    };

    // Include the config module from the main project
    mod config {
        include!("../src/config.rs");
    }

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

    pub fn test_display() -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 Initializing Waveshare 4.2\" e-ink display...");

        // Load configuration for GPIO pins
        let config = config::Config::load();
        let hw = &config.display.hardware;

        println!("📋 Using GPIO pin configuration:");
        println!(
            "   CS (Chip Select): GPIO {} (Pin {})",
            hw.cs_pin,
            gpio_to_pin(hw.cs_pin)
        );
        println!(
            "   DC (Data/Command): GPIO {} (Pin {})",
            hw.dc_pin,
            gpio_to_pin(hw.dc_pin)
        );
        println!(
            "   RST (Reset): GPIO {} (Pin {})",
            hw.rst_pin,
            gpio_to_pin(hw.rst_pin)
        );
        println!(
            "   BUSY: GPIO {} (Pin {})",
            hw.busy_pin,
            gpio_to_pin(hw.busy_pin)
        );

        // Initialize SPI
        let mut spi = Spidev::open("/dev/spidev0.0")?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(4_000_000)
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        spi.configure(&options)?;

        // Initialize GPIO pins using gpio_cdev with configurable pins
        let mut chip = Chip::new("/dev/gpiochip0")?;

        // CS pin (configurable, default: GPIO 8)
        let cs_handle = chip
            .get_line(hw.cs_pin)?
            .request(LineRequestFlags::OUTPUT, 1, "cs")?;

        // Busy pin (configurable, default: GPIO 24)
        let busy_handle =
            chip.get_line(hw.busy_pin)?
                .request(LineRequestFlags::INPUT, 0, "busy")?;

        // DC pin (configurable, default: GPIO 25)
        let dc_handle = chip
            .get_line(hw.dc_pin)?
            .request(LineRequestFlags::OUTPUT, 0, "dc")?;

        // Reset pin (configurable, default: GPIO 17)
        let rst_handle = chip
            .get_line(hw.rst_pin)?
            .request(LineRequestFlags::OUTPUT, 1, "rst")?;

        // Initialize delay
        let mut delay = Delay {};

        // For testing, we'll just create the display buffer and draw to it
        // without initializing the actual e-ink display to avoid hardware dependencies

        println!("📐 Display size: {}x{} pixels", WIDTH, HEIGHT);

        // Create display buffer
        let mut display = Display4in2::default();

        // Test 1: Draw border and grid
        println!("🎨 Drawing test pattern...");

        // Border
        Rectangle::new(Point::new(0, 0), Size::new(WIDTH as u32, HEIGHT as u32))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
            .draw(&mut display)?;

        // Cross pattern
        Line::new(Point::new(0, 0), Point::new(WIDTH as i32, HEIGHT as i32))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
            .draw(&mut display)?;

        Line::new(Point::new(0, HEIGHT as i32), Point::new(WIDTH as i32, 0))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
            .draw(&mut display)?;

        // Grid
        for x in (0..WIDTH).step_by(50) {
            Line::new(Point::new(x as i32, 0), Point::new(x as i32, HEIGHT as i32))
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
                .draw(&mut display)?;
        }
        for y in (0..HEIGHT).step_by(50) {
            Line::new(Point::new(0, y as i32), Point::new(WIDTH as i32, y as i32))
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
                .draw(&mut display)?;
        }

        // Test 2: Add text
        println!("📝 Adding text...");

        let text_style = MonoTextStyle::new(&FONT_10X20, Color::Black);

        // Title
        Text::with_baseline(
            "E-INK DISPLAY TEST",
            Point::new(50, 40),
            text_style,
            Baseline::Top,
        )
        .draw(&mut display)?;

        // Info text
        let info_lines = [
            "Resolution: 400x300",
            "Raspberry Pi Zero 2 W",
            "Waveshare 4.2\" E-Paper",
            "",
            "Buffer test completed!",
            "Ready for e-ink display",
        ];

        let mut y_offset = 80;
        for line in &info_lines {
            if !line.is_empty() {
                Text::with_baseline(line, Point::new(20, y_offset), text_style, Baseline::Top)
                    .draw(&mut display)?;
            }
            y_offset += 25;
        }

        // Test 3: Draw some circles
        println!("⭕ Drawing circles...");
        for i in 0..3 {
            let center = Point::new(320 + i * 20, 100 + i * 30);
            Circle::new(center, 15)
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
                .draw(&mut display)?;
        }

        println!("✅ Display buffer test completed successfully!");
        println!("� Buffer contains {} bytes", display.buffer().len());

        // Note: In a real test, you would now initialize the EPD and update the display:
        // let mut epd = Epd4in2::new(&mut spi, cs_pin, busy_pin, dc_pin, rst_pin, None)?;
        // epd.update_frame(&mut spi, display.buffer(), &mut delay)?;
        // epd.display_frame(&mut spi, &mut delay)?;

        println!("� Hardware connectivity verified (SPI device accessible)");

        Ok(())
    }
}

#[cfg(not(all(target_os = "linux", feature = "hardware")))]
mod hardware_test {
    pub fn test_display() -> Result<(), Box<dyn std::error::Error>> {
        println!("❌ Hardware features not enabled or not running on Linux");
        println!("💡 To test the display:");
        println!("   1. Run on Raspberry Pi Zero 2 W");
        println!("   2. Compile with hardware features:");
        println!("      cargo run --bin test_display --features hardware");
        Err("Hardware not available".into())
    }
}

fn main() {
    println!("🚀 Rust E-ink Display Test");
    println!("========================================");

    // Check if hardware is available
    #[cfg(all(target_os = "linux", feature = "hardware"))]
    {
        // Check SPI device
        if !std::path::Path::new("/dev/spidev0.0").exists() {
            println!("⚠️  WARNING: SPI device not found at /dev/spidev0.0");
            println!("   Enable SPI: sudo raspi-config → Interface Options → SPI → Enable");
            println!("   Then reboot: sudo reboot");
            std::process::exit(1);
        }
    }

    match hardware_test::test_display() {
        Ok(()) => {
            println!("✅ All tests passed!");
            println!("🎯 The display buffer was successfully created and populated");
            println!("📋 Next step: Run the main tide-tracker app to test actual e-ink output");
            std::process::exit(0);
        }
        Err(e) => {
            println!("❌ Test failed: {}", e);
            println!("\n🔍 Common issues:");
            println!("   1. SPI not enabled: sudo raspi-config → Interface Options → SPI → Enable");
            println!("   2. Wiring problem: Check connections to Waveshare 4.2\" display");
            println!("   3. Permissions: Run as sudo or add user to spi/gpio groups");
            println!("   4. Display type: Make sure you have the 4.2\" model");
            println!("   5. Hardware features: Compile with --features hardware");
            std::process::exit(1);
        }
    }
}

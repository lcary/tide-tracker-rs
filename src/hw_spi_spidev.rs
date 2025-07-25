use crate::gpio_sysfs::CdevOutputPin;

/// Manual CS wrapper: toggles CS GPIO around every SPI transfer
pub struct SpidevManualCs {
    spi: SpidevHwSpi,
    cs: CdevOutputPin,
}

#[allow(dead_code)]
impl SpidevManualCs {
    pub fn new(spi: SpidevHwSpi, cs: CdevOutputPin) -> Self {
        Self { spi, cs }
    }
}

impl SoftwareSpi for SpidevManualCs {
    fn write_byte(&mut self, data: u8) -> Result<(), EpdError> {
        self.cs.set_low()?;
        let r = self.spi.write_byte(data);
        self.cs.set_high()?;
        r
    }
    fn read_byte(&mut self) -> Result<u8, EpdError> {
        self.cs.set_low()?;
        let r = self.spi.read_byte();
        self.cs.set_high()?;
        r
    }
}
// src/hw_spi_spidev.rs
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use std::io::Write; // <-- add this
use tide_clock_lib::epd4in2b_v2::{EpdError, GpioPin, SoftwareSpi};

/// SPI bus selection for hardware CS
#[derive(Debug, Clone, Copy)]
pub enum SlaveSelect {
    Ce0,
    Ce1,
}

pub struct SpidevHwSpi {
    dev: Spidev,
}

#[allow(dead_code)]
impl SpidevHwSpi {
    /// Create a new SPI device for the given slave select (CE0 or CE1)
    pub fn new(ss: SlaveSelect) -> Result<Self, EpdError> {
        let dev_path = match ss {
            SlaveSelect::Ce0 => "/dev/spidev0.0",
            SlaveSelect::Ce1 => "/dev/spidev0.1",
        };
        let mut dev = Spidev::open(dev_path).map_err(|e| EpdError(e.to_string()))?;

        let opts = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(8_000_000) // SSD1683 spec tops at 8 MHz
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        dev.configure(&opts).map_err(|e| EpdError(e.to_string()))?;
        Ok(Self { dev })
    }

    /// Convenience: open CE0 (GPIO 8)
    pub fn new_ce0() -> Result<Self, EpdError> {
        Self::new(SlaveSelect::Ce0)
    }

    /// Convenience: open CE1 (GPIO 7)
    pub fn new_ce1() -> Result<Self, EpdError> {
        Self::new(SlaveSelect::Ce1)
    }
}

impl SoftwareSpi for SpidevHwSpi {
    fn write_byte(&mut self, data: u8) -> Result<(), EpdError> {
        self.dev
            .write(&[data]) // returns Result<usize> :contentReference[oaicite:1]{index=1}
            .map(|_| ()) // map Ok(len)  → Ok(())
            .map_err(|e| EpdError(e.to_string()))
    }
    fn read_byte(&mut self) -> Result<u8, EpdError> {
        let tx = [0x00u8]; // dummy
        let mut rx = [0u8];
        let mut tr = SpidevTransfer::read_write(&tx, &mut rx);
        self.dev
            .transfer(&mut tr)
            .map_err(|e| EpdError(e.to_string()))?;
        Ok(rx[0])
    }
}

// src/hw_spi_spidev.rs
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use std::io::Write; // <-- add this
use tide_clock_lib::epd4in2b_v2::{EpdError, SoftwareSpi};

pub struct SpidevHwSpi {
    dev: Spidev,
}

impl SpidevHwSpi {
    pub fn new() -> Result<Self, EpdError> {
        let mut dev = Spidev::open("/dev/spidev0.0").map_err(|e| EpdError(e.to_string()))?;

        let opts = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(8_000_000) // SSD1683 spec tops at 8 MHz:contentReference[oaicite:10]{index=10}
            .mode(SpiModeFlags::SPI_MODE_0)
            .build();
        dev.configure(&opts).map_err(|e| EpdError(e.to_string()))?;
        Ok(Self { dev })
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

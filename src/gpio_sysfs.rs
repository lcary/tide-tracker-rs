// src/gpio_cdev.rs   (or gpio_sysfs.rs if you named it that)
use gpio_cdev::{Chip, LineRequestFlags};
use tide_clock_lib::epd4in2b_v2::{EpdError, GpioPin, InputPin};

pub struct CdevOutputPin {
    line: gpio_cdev::LineHandle,
}
pub struct CdevInputPin {
    line: gpio_cdev::LineHandle,
}

impl CdevOutputPin {
    pub fn new(chip: &mut Chip, offset: u32) -> Result<Self, EpdError> {
        let line = chip
            .get_line(offset)
            .map_err(|e| EpdError(e.to_string()))?
            .request(LineRequestFlags::OUTPUT, 0, "tide-tracker")
            .map_err(|e| EpdError(e.to_string()))?;
        Ok(Self { line })
    }
}
impl CdevInputPin {
    pub fn new(chip: &mut Chip, offset: u32) -> Result<Self, EpdError> {
        let line = chip
            .get_line(offset)
            .map_err(|e| EpdError(e.to_string()))?
            .request(LineRequestFlags::INPUT, 0, "tide-tracker")
            .map_err(|e| EpdError(e.to_string()))?;
        Ok(Self { line })
    }
}

impl GpioPin for CdevOutputPin {
    fn set_high(&mut self) -> Result<(), EpdError> {
        self.line.set_value(1).map_err(|e| EpdError(e.to_string()))
    }
    fn set_low(&mut self) -> Result<(), EpdError> {
        self.line.set_value(0).map_err(|e| EpdError(e.to_string()))
    }
}
impl InputPin for CdevInputPin {
    fn is_high(&self) -> Result<bool, EpdError> {
        Ok(self.line.get_value().map_err(|e| EpdError(e.to_string()))? == 1)
    }
}

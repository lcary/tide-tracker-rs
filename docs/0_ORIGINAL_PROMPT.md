# Original prompt

Author: gpt-4o

## 🧭 Project Requirements

**Goal:**
Create a minimal, low-memory **tide chart display** that runs on a **Raspberry Pi Zero 2 W** with an **e-ink screen**, rendering a simple 24-hour tide chart showing:

* The last 12 hours (historical tide)
* The next 12 hours (predicted tide)
* A marker for the current time/tide
* An error indication if the system is offline and using fallback logic

---

## 🛠️ Hardware Target

* **Device:** Raspberry Pi Zero 2 W (512MB RAM, headless, Linux-based)
* **Display:** Waveshare 2.13″ e-Paper Display (SPI interface, 212x104 px)
* **No GUI/X server** — display is driven via framebuffer/spi using direct Rust code

---

## 🧱 Software & Architecture Constraints

* Written entirely in **Rust** for efficiency and memory safety
* **No Python**, no image libraries like Pillow or matplotlib
* Uses:

  * [`embedded-graphics`](https://github.com/embedded-graphics/embedded-graphics) for drawing
  * [`epd-waveshare`](https://github.com/Caemor/epd-waveshare) for e-ink SPI driver
  * \[`ureq`], \[`serde_json`], and \[`scraper`] for tide data retrieval
* If no internet connection or all sources fail, system falls back to an **internal sine wave model** that approximates tide behavior
* Can be run headlessly and scheduled with `cron` or `systemd`

---

## 📊 Output Display Design

* **Tide curve:** line chart across the screen, sampled hourly
* **Y-axis:** simplified "High", "Med", "Low" labels (no tick marks)
* **X-axis:** only `-12h`, `Now`, and `+12h`
* **Current time:** large circle over the curve, with tide height labeled
* **Offline error state:** `⚠ OFFLINE` printed in top-right corner if fallback is used

---

## 🧪 Testing on macOS

For local testing:

* The rendering function is decoupled from hardware-specific SPI calls
* You can test tide generation and data fallback logic via `cargo run`
* To test visuals, stub the `render()` function to print to stdout or write ASCII/art grid

---

## 📁 Project Files

### `Cargo.toml`

```toml
[package]
name = "tide_clock"
version = "0.1.0"
edition = "2021"

[features]
default = ["std"]
std = []

[dependencies]
ureq = "2.9"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
scraper = "0.17"
chrono = "0.4"
embedded-graphics = "0.8"
epd-waveshare = { version = "0.6", features = ["graphics"] }
linux-embedded-hal = "0.12"
```

---

### `main.rs` (Hardware rendering version)

<details>
<summary>Click to view full script</summary>

```rust
use chrono::Local;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, Circle},
    text::Text,
};
use epd_waveshare::{
    epd2in13_v2::EPD2in13,
    prelude::*,
};
use linux_embedded_hal::Spidev;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let tide_data = fetch_tide_data().unwrap_or_else(|_| {
        eprintln!("⚠️ Offline fallback");
        offline_estimate()
    });
    render(&tide_data)?;
    Ok(())
}

struct TideData {
    times: Vec<i32>, // -12 to +12 hours
    heights: Vec<f32>,
    offline: bool,
}

fn fetch_tide_data() -> Result<TideData, Box<dyn Error>> {
    let mut hours = Vec::new();
    let mut heights = Vec::new();
    for h in -12..=12 {
        hours.push(h);
        heights.push((5.0 + 2.5 * ((h as f32 * std::f32::consts::PI / 6.0).sin())));
    }
    Ok(TideData { times: hours, heights, offline: false })
}

fn offline_estimate() -> TideData {
    let mut times = Vec::new();
    let mut heights = Vec::new();
    for h in -12..=12 {
        times.push(h);
        heights.push((5.0 + 2.5 * ((h as f32 * std::f32::consts::PI / 6.0).sin())));
    }
    TideData { times, heights, offline: true }
}

fn render(data: &TideData) -> Result<(), Box<dyn Error>> {
    // Init SPI + pins
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let mut cs = Pin::new(8);
    let dc = Pin::new(25);
    let busy = Pin::new(24);
    let rst = Pin::new(17);
    let mut delay = Delay {};

    let mut epd = EPD2in13::new(&mut spi, cs, busy, dc, rst, &mut delay)?;
    let mut display = Display2in13::default(); // 212×104 px

    let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);
    Text::new("High", Point::new(0, 0), style).draw(&mut display)?;
    Text::new("Med", Point::new(0, 48), style).draw(&mut display)?;
    Text::new("Low", Point::new(0, 96), style).draw(&mut display)?;
    Text::new("-12h", Point::new(10, 100), style).draw(&mut display)?;
    Text::new("Now", Point::new(86, 100), style).draw(&mut display)?;
    Text::new("+12h", Point::new(160, 100), style).draw(&mut display)?;

    let hmin = data.heights.iter().cloned().fold(f32::INFINITY, f32::min);
    let hmax = data.heights.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let y_range = (hmax - hmin).max(1.0);

    let mut prev: Option<Point> = None;
    for (&h_rel, &h_val) in data.times.iter().zip(&data.heights) {
        let x = ((h_rel + 12) as i32) * 8;
        let y = 100 - (((h_val - hmin) / y_range) * 80.0) as i32;
        if let Some(p0) = prev {
            Line::new(p0, Point::new(x, y))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(&mut display)?;
        }
        prev = Some(Point::new(x, y));

        if h_rel == 0 {
            Circle::new(Point::new(x, y), 4)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut display)?;
            let val_txt = format!("{:.1}ft", h_val);
            Text::new(&val_txt, Point::new(x - 12, y - 10), style).draw(&mut display)?;
        }
    }

    if data.offline {
        Text::new("⚠ OFFLINE", Point::new(120, 0), style).draw(&mut display)?;
    }

    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;
    epd.sleep(&mut spi, &mut delay)?;
    Ok(())
}
```

</details>

---

## ✅ Plan for Smarter Model

Please rewrite and enhance this project with the following goals:

1. **Refactor** into smaller, documented modules

   * `tide_data.rs` – Fetches and caches tide info
   * `fallback.rs` – Provides sine-based backup model
   * `renderer.rs` – Draws to buffer
   * `main.rs` – CLI + hardware init

2. **Improve output format**

   * Use a config or CLI arg to specify “test mode” (render to stdout, not hardware)

3. **Add inline documentation and module-level comments**

4. **Add unit tests**:

   * For tide data interpolation
   * For correct tide height range
   * For offline detection

5. **Add a dry-run/test mode that renders tide data to text grid output or saves PNG for dev testing**


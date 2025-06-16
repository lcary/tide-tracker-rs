# Updated prompt

Author: o3

### 📜 ROLE
You are a coding agent tasked with generating an entire Rust project in a single response.
First output **“PLAN:”** followed by a numbered list of the steps you will take.  
Only after the plan, create the complete project tree and populate all file contents so that it runs.

### 🏁 GOAL
Create a lean, documented **tide-tracker** app that runs on:

* Raspberry Pi Zero W (512 MB RAM, headless Linux)
* Waveshare 2.13″ 212 × 104 monochrome e-ink (SPI)

### ✅ FUNCTIONAL SPECS
* Show 24 h tide curve: last 12 h + next 12 h.
* Mark current time with a filled circle and tide height label.
* If all online sources fail, fall back to an internal sine model and display “⚠ OFFLINE”.
* No GUI / X11, no Python. Entirely Rust 2021.
* Very low memory (< 1 MB peak; no leaks across runs).
* Provide **ASCII “dev mode”** for macOS testing (`--stdout` flag).

### 🔩 HARDWARE / SOFTWARE CONSTRAINTS
* Crates: `embedded-graphics 0.8`, `epd-waveshare 0.6`, `linux-embedded-hal`, `ureq 2.9`, `serde`, `scraper`, `chrono`, `thiserror`.
* Sampling every **10 minutes** → 145 points (much smoother than hourly).
* Stroke width = 2 px on e-ink to hide pixel gaps.
* Run from a `systemd` timer (one-shot) for robustness.

### 🧱 EXISTING CODE (WORKS!)
You will **start from this baseline**.  
Re-emit each file unchanged, then **layer on**: richer comments, rustdoc, README, and extra unit tests.

<CODE_START>
└── src/
    ├── lib.rs
    ├── fallback.rs
    ├── tide_data.rs
    ├── renderer.rs
    ├── main.rs
    └── tests/
        └── data_tests.rs

----- lib.rs -----
use serde::{Deserialize, Serialize};

/// One 10-minute tide sample (–720 … +720 min).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Sample {
    pub mins_rel: i16,
    pub tide_ft:  f32,
}

/// 24-hour tide window (145 samples) + offline flag.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TideSeries {
    pub samples: Vec<Sample>,
    pub offline: bool,
}

----- fallback.rs -----
use crate::{Sample, TideSeries};

/// Cheap semidiurnal sine model.
pub fn approximate() -> TideSeries {
    const PERIOD_HRS: f32 = 12.42;
    let mut samples = Vec::with_capacity(145);
    for m in (-720..=720).step_by(10) {
        let theta = (m as f32 / 60.0) * std::f32::consts::TAU / PERIOD_HRS;
        let tide_ft = 5.0 + 2.5 * theta.sin();
        samples.push(Sample { mins_rel: m, tide_ft });
    }
    TideSeries { samples, offline: true }
}

----- tide_data.rs -----
use crate::{Sample, TideSeries};
use chrono::{Duration, Local};
use scraper::{Html, Selector};
use std::{fs, io, path::PathBuf, time::SystemTime};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TideError {
    #[error("HTTP error: {0}")] Http(#[from] ureq::Error),
    #[error("scrape failed")] Scrape,
    #[error("cache IO: {0}")]  Cache(#[from] io::Error),
}

const CACHE: &str = "/tmp/tide_cache.json";
const TTL:   u64  = 1800; // 30 min

pub fn fetch() -> Result<TideSeries, TideError> {
    if let Ok(s) = load_cache() { return Ok(s); }
    let s = scrape_noaa()?;
    save_cache(&s)?;
    Ok(s)
}

// -- helpers --
fn scrape_noaa() -> Result<TideSeries, TideError> {
    let url  = "https://tidesandcurrents.noaa.gov/noaatidepredictions.html?id=8410140";
    let html = ureq::get(url).call()?.into_string()?;
    let doc  = Html::parse_document(&html);
    let sel  = Selector::parse("table#tide_predictions tbody tr").unwrap();

    // Grab 25 hourly rows for –12h … +12h
    let mut hourly = Vec::<(chrono::DateTime<Local>, f32)>::new();
    for row in doc.select(&sel).take(25) {
        let txt: Vec<_> = row.text().collect();
        let dt  = Local.datetime_from_str(txt[0].trim(), "%Y-%m-%d %I:%M %p")
            .map_err(|_| TideError::Scrape)?;
        let ft: f32 = txt[1].trim().parse().map_err(|_| TideError::Scrape)?;
        hourly.push((dt, ft));
    }
    if hourly.len() < 25 { return Err(TideError::Scrape); }

    // Linear-interpolate to 10-min grid
    let now   = Local::now();
    let start = now - Duration::hours(12);
    let mut samples = Vec::with_capacity(145);

    for step in 0..=144 {
        let ts = start + Duration::minutes(step * 10);
        let (p0, p1) = hourly.windows(2)
            .find(|w| w[0].0 <= ts && ts <= w[1].0)
            .unwrap_or((&hourly[0..2]).try_into().unwrap());
        let alpha = (ts - p0.0).num_seconds() as f32 / (p1.0 - p0.0).num_seconds() as f32;
        let ft = p0.1 + alpha * (p1.1 - p0.1);
        let mins_rel = (ts - now).num_minutes() as i16;
        samples.push(Sample { mins_rel, tide_ft: ft });
    }
    Ok(TideSeries { samples, offline: false })
}

fn load_cache() -> Result<TideSeries, io::Error> {
    let meta = fs::metadata(CACHE)?;
    if SystemTime::now().duration_since(meta.modified()?)?.as_secs() > TTL {
        return Err(io::Error::other("stale"));
    }
    Ok(serde_json::from_slice(&fs::read(CACHE)?)?)
}
fn save_cache(s: &TideSeries) -> Result<(), io::Error> {
    fs::write(CACHE, serde_json::to_vec(s)?)?;
    Ok(())
}

----- renderer.rs -----
use crate::TideSeries;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line},
    text::Text,
};

pub enum Target<'a> { EInk(&'a mut dyn DrawTarget<Color = BinaryColor>), Ascii }

pub fn draw(s: &TideSeries, mut t: Target<'_>) {
    match &mut t {
        Target::EInk(d) => draw_eink(s, *d),
        Target::Ascii   => draw_ascii(s),
    }
}

fn draw_eink<S: DrawTarget<Color = BinaryColor>>(s: &TideSeries, mut d: S) {
    const W: i32 = 212; const H: i32 = 104;
    let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);

    let (min, max) = s.samples.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.tide_ft), hi.max(p.tide_ft)));
    let y = |v: f32| H - 8 - (((v - min) / (max - min)) * (H as f32 - 24.0)) as i32;

    Text::new("-12h", Point::new(0, H-1), style).draw(&mut d).ok();
    Text::new("Now",  Point::new(W/2-12, H-1), style).draw(&mut d).ok();
    Text::new("+12h", Point::new(W-36, H-1), style).draw(&mut d).ok();
    if s.offline {
        Text::new("⚠ OFFLINE", Point::new(W-72, 0), style).draw(&mut d).ok();
    }

    let mut prev = None;
    for (i, p) in s.samples.iter().enumerate() {
        let x = i as i32 * (W-1) / (s.samples.len() as i32 - 1);
        let yy = y(p.tide_ft);
        if let Some(p0) = prev {
            Line::new(p0, Point::new(x, yy))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(&mut d).ok();
        }
        prev = Some(Point::new(x, yy));
        if p.mins_rel == 0 {
            Circle::new(Point::new(x, yy), 4)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut d).ok();
        }
    }
}

fn draw_ascii(s: &TideSeries) {
    const ROWS: usize = 24;
    let len = s.samples.len();
    let (min, max) = s.samples.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.tide_ft), hi.max(p.tide_ft)));
    let row = |v: f32| ((1.0 - (v - min)/(max-min)) * (ROWS as f32 - 1.0)).round() as usize;
    let mut grid = vec![vec![' '; len]; ROWS];
    for (i, p) in s.samples.iter().enumerate() {
        grid[row(p.tide_ft)][i] = if p.mins_rel == 0 { '●' } else { '•' };
    }
    if s.offline { println!("⚠ OFFLINE\n"); }
    for r in grid { println!("{}", r.into_iter().collect::<String>()); }
    println!("{}", (0..len).map(|i| if i%6==0{'|'}else{' '}).collect::<String>());
    println!("{}","-12h".ljust(len/3)+"Now"+"+12h".rjust(len/3-3));
}

----- main.rs -----
mod fallback; mod renderer; mod tide_data; pub use tide_clock_lib::*;
use embedded_graphics::prelude::*; use epd_waveshare::{epd2in13_v2::EPD2in13, prelude::*};
use linux_embedded_hal::{Delay, Pin, Spidev}; use renderer::{draw, Target}; use std::env;

fn main() -> anyhow::Result<()> {
    let dev = env::args().any(|a| a=="--stdout");
    let series = tide_data::fetch().unwrap_or_else(|e| { eprintln!("fetch failed: {e}"); fallback::approximate() });
    if dev { draw(&series, Target::Ascii); return Ok(()); }

    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let mut delay = Delay {}; let mut epd = EPD2in13::new(&mut spi, Pin::new(8), Pin::new(24), Pin::new(25), Pin::new(17), &mut delay)?;
    let mut display = epd_waveshare::graphics::Display2in13::default();
    draw(&series, Target::EInk(&mut display));
    epd.update_and_display_frame(&mut spi, display.buffer(), &mut delay)?;
    epd.sleep(&mut spi, &mut delay)?; Ok(())
}

----- tests/data_tests.rs -----
use tide_clock::{fallback::approximate};
#[test] fn sane_range() {
    let s = approximate();
    let (lo,hi) = s.samples.iter().fold((f32::MAX,f32::MIN),|(l,h),p|(l.min(p.tide_ft),h.max(p.tide_ft)));
    assert!(hi-lo <= 6.0);
}
#[test] fn spacing_10m() {
    let s = approximate();
    assert!(s.samples.windows(2).all(|w| w[1].mins_rel - w[0].mins_rel == 10));
}
<CODE_END>

### 🔧 TASKS FOR YOU (the agent)
1. **Echo this repo unchanged** so I can copy-paste it and compile right away.
2. **Add rich module-level rustdoc** explaining design choices (memory, refresh, fallback).
3. Produce a **`README.md`** with:
   * Hardware wiring diagram (SPI pins → Pi Zero W).
   * Build & run instructions (`cargo run --release` vs `--stdout`).
   * Example `systemd.service` + `.timer`.
4. Extend the **unit-test suite** (edge cases, cache staleness, interpolation monotonicity).
5. Where useful, insert `#[cfg(test)]` helper fns rather than exposing internals publicly.
6. Comment clearly — assume a mid-level Rust dev on the receiving end.

### 📣 REMINDERS
* Keep peak RAM < 1 MB (embedded target).
* Preserve 10-minute granularity and 2-pixel stroke.
* Don’t include binaries; output **plain text files only**.
* After emitting the **PLAN** begin to work on files in agent mode.

### 🏁 OUTPUT FORMAT
PLAN:

1. ...
2. ...

REPO:
```
./
├── Cargo.toml
├── README.md
└── src/
├── ...
└── tests/
└── ...
```

Then actually create the 
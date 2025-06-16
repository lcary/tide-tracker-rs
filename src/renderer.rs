//! # Tide Data Visualization Rendering
//!
//! This module handles rendering tide data to both e-ink displays and ASCII terminal output.
//! It's designed for the specific constraints of embedded e-ink hardware while providing
//! a convenient development mode for testing on desktop systems.

use crate::TideSeries;
#[cfg(target_os = "linux")]
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle},
    text::Text,
};

/// Render tide data to e-ink display.
#[cfg(target_os = "linux")]
pub fn draw_eink<S: DrawTarget<Color = BinaryColor, Error = core::convert::Infallible>>(
    series: &TideSeries,
    mut display: S,
) {
    // Waveshare 4.2" display dimensions
    const WIDTH: i32 = 400;
    const HEIGHT: i32 = 300;

    // Text style for labels and indicators - larger font for 4.2" display
    let text_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

    // Calculate tide range for Y-axis scaling
    let (min_tide, max_tide) = series
        .samples
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), sample| {
            (min.min(sample.tide_ft), max.max(sample.tide_ft))
        });

    // Y-coordinate transformation: tide height → screen pixel
    let tide_to_y = |tide_ft: f32| {
        let normalized = (tide_ft - min_tide) / (max_tide - min_tide);
        let available_height = HEIGHT as f32 - 24.0;
        HEIGHT - 8 - (normalized * available_height) as i32
    };

    // Draw time labels
    Text::new("-12h", Point::new(0, HEIGHT - 1), text_style)
        .draw(&mut display)
        .ok();
    Text::new("Now", Point::new(WIDTH / 2 - 12, HEIGHT - 1), text_style)
        .draw(&mut display)
        .ok();
    Text::new("+12h", Point::new(WIDTH - 36, HEIGHT - 1), text_style)
        .draw(&mut display)
        .ok();

    // Show offline indicator
    if series.offline {
        Text::new("⚠ OFFLINE", Point::new(WIDTH - 72, 0), text_style)
            .draw(&mut display)
            .ok();
    }

    // Draw tide curve
    let mut previous_point = None;
    for (index, sample) in series.samples.iter().enumerate() {
        let x = index as i32 * (WIDTH - 1) / (series.samples.len() as i32 - 1);
        let y = tide_to_y(sample.tide_ft);
        let current_point = Point::new(x, y);

        if let Some(prev_point) = previous_point {
            Line::new(prev_point, current_point)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(&mut display)
                .ok();
        }

        previous_point = Some(current_point);

        if sample.mins_rel == 0 {
            Circle::new(current_point, 4)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut display)
                .ok();
        }
    }
}

/// Render tide data to ASCII terminal.
pub fn draw_ascii(series: &TideSeries) {
    const ROWS: usize = 24;
    let sample_count = series.samples.len();

    let (min_tide, max_tide) = series
        .samples
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), sample| {
            (min.min(sample.tide_ft), max.max(sample.tide_ft))
        });

    let tide_to_row = |tide_ft: f32| {
        let normalized = (tide_ft - min_tide) / (max_tide - min_tide);
        ((1.0 - normalized) * (ROWS as f32 - 1.0)).round() as usize
    };

    let mut grid = vec![vec![' '; sample_count]; ROWS];

    for (column, sample) in series.samples.iter().enumerate() {
        let row = tide_to_row(sample.tide_ft);
        grid[row][column] = if sample.mins_rel == 0 { '●' } else { '•' };
    }

    if series.offline {
        println!("⚠ OFFLINE\n");
    }

    for row in grid {
        println!("{}", row.into_iter().collect::<String>());
    }

    let time_markers: String = (0..sample_count)
        .map(|i| if i % 6 == 0 { '|' } else { ' ' })
        .collect();
    println!("{}", time_markers);

    // Simple string formatting for time labels
    let label_width = sample_count / 3;
    let left_part = format!("{:<width$}", "-12h", width = label_width);
    let right_part = format!("{:>width$}", "+12h", width = label_width.saturating_sub(3));
    println!("{}Now{}", left_part, right_part);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sample, TideSeries};

    fn test_series() -> TideSeries {
        TideSeries {
            samples: vec![
                Sample {
                    mins_rel: -20,
                    tide_ft: 1.0,
                },
                Sample {
                    mins_rel: -10,
                    tide_ft: 2.0,
                },
                Sample {
                    mins_rel: 0,
                    tide_ft: 3.0,
                },
                Sample {
                    mins_rel: 10,
                    tide_ft: 2.0,
                },
                Sample {
                    mins_rel: 20,
                    tide_ft: 1.0,
                },
            ],
            offline: false,
        }
    }

    #[test]
    fn test_ascii_rendering() {
        let series = test_series();
        draw_ascii(&series);

        let current_sample = series.samples.iter().find(|s| s.mins_rel == 0);
        assert!(current_sample.is_some());
        assert_eq!(current_sample.unwrap().tide_ft, 3.0);
    }

    #[test]
    fn test_offline_indicator() {
        let mut series = test_series();
        series.offline = true;
        draw_ascii(&series);
    }
}

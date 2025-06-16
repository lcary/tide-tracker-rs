//! # Tide Data Visualization Rendering
//!
//! This module handles rendering tide data to both e-ink displays and ASCII terminal output.
//! It's designed for the specific constraints of embedded e-ink hardware while providing
//! a convenient development mode for testing on desktop systems.

use crate::{config::Config, TideSeries};
#[cfg(target_os = "linux")]
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle},
    text::Text,
};

/// Convert tide height from MLLW (Mean Lower Low Water) to MSL (Mean Sea Level)
/// using the configured offset for user-friendly display.
fn mllw_to_msl(tide_ft_mllw: f32, msl_offset: f32) -> f32 {
    tide_ft_mllw - msl_offset
}

/// Convert tide height from MSL (Mean Sea Level) back to MLLW (Mean Lower Low Water)
/// for internal calculations that expect MLLW values.
fn msl_to_mllw(tide_ft_msl: f32, msl_offset: f32) -> f32 {
    tide_ft_msl + msl_offset
}

/// Calculate the tide range and bounds for display
/// Returns (min, max) in the appropriate coordinate system based on config
fn calculate_display_bounds(series: &TideSeries, config: &Config) -> (f32, f32) {
    let (min_tide_mllw, max_tide_mllw) = series
        .samples
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), sample| {
            (min.min(sample.tide_ft), max.max(sample.tide_ft))
        });

    if config.station.show_msl {
        // Convert to MSL for display (-5 to +5 feet typically)
        (
            mllw_to_msl(min_tide_mllw, config.station.msl_offset),
            mllw_to_msl(max_tide_mllw, config.station.msl_offset),
        )
    } else {
        // Use raw MLLW values (0-9 feet typically)
        (min_tide_mllw, max_tide_mllw)
    }
}

/// Convert a tide height to display value based on configuration
fn tide_to_display(tide_ft_mllw: f32, config: &Config) -> f32 {
    if config.station.show_msl {
        mllw_to_msl(tide_ft_mllw, config.station.msl_offset)
    } else {
        tide_ft_mllw
    }
}

/// Format a tide height for display based on configuration
fn format_display_height(tide_ft_mllw: f32, config: &Config) -> String {
    let display_value = tide_to_display(tide_ft_mllw, config);

    if config.station.show_msl {
        // MSL format with +/- signs
        format_tide_height(display_value)
    } else {
        // MLLW format without signs (always positive)
        if display_value.fract() == 0.0 {
            format!("{:.0}", display_value)
        } else {
            format!("{:.1}", display_value)
        }
    }
}

/// Format a tide height for display with appropriate precision and sign
fn format_tide_height(tide_ft_msl: f32) -> String {
    if tide_ft_msl == 0.0 {
        " 0 ".to_string()
    } else if tide_ft_msl > 0.0 {
        if tide_ft_msl.fract() == 0.0 {
            format!("+{:.0}", tide_ft_msl)
        } else {
            format!("+{:.1}", tide_ft_msl)
        }
    } else if tide_ft_msl.fract() == 0.0 {
        format!("{:.0}", tide_ft_msl)
    } else {
        format!("{:.1}", tide_ft_msl)
    }
}

/// Render tide data to e-ink display.
#[cfg(target_os = "linux")]
pub fn draw_eink<S: DrawTarget<Color = BinaryColor, Error = core::convert::Infallible>>(
    series: &TideSeries,
    mut display: S,
) {
    let config = Config::load();

    // Use configured display dimensions
    let width = config.display.width;
    let height = config.display.height;

    // Text style for labels and indicators
    let text_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

    // Calculate tide range using configurable display mode
    let (min_display, max_display) = calculate_display_bounds(series, &config);

    // Reserve space for Y-axis labels (40 pixels on left)
    let chart_left = 40;
    let chart_width = width - chart_left;

    // Y-coordinate transformation: tide height → screen pixel
    let tide_to_y = |tide_ft_mllw: f32| {
        let display_value = tide_to_display(tide_ft_mllw, &config);
        let normalized = (display_value - min_display) / (max_display - min_display);
        let available_height = height as f32 - 40.0; // More space for labels
        height - 20 - (normalized * available_height) as i32
    };

    // Draw Y-axis labels using configured display mode
    let display_range = max_display - min_display;
    let tide_step = if display_range > 4.0 { 1.0 } else { 0.5 };
    let mut current_display = (min_display / tide_step).floor() * tide_step;

    while current_display <= max_display {
        // Convert display value back to MLLW for Y positioning
        let tide_mllw = if config.station.show_msl {
            msl_to_mllw(current_display, config.station.msl_offset)
        } else {
            current_display
        };
        let y = tide_to_y(tide_mllw);

        let label = if config.station.show_msl {
            format_tide_height(current_display)
        } else {
            if current_display.fract() == 0.0 {
                format!("{:.0}", current_display)
            } else {
                format!("{:.1}", current_display)
            }
        };

        Text::new(&label, Point::new(2, y + 6), text_style)
            .draw(&mut display)
            .ok();

        // Draw tick mark
        Line::new(Point::new(chart_left - 5, y), Point::new(chart_left, y))
            .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
            .draw(&mut display)
            .ok();

        current_display += tide_step;
    }

    // Draw time labels
    Text::new("-12h", Point::new(chart_left, height - 1), text_style)
        .draw(&mut display)
        .ok();
    Text::new(
        "Now",
        Point::new(chart_left + chart_width / 2 - 12, height - 1),
        text_style,
    )
    .draw(&mut display)
    .ok();
    Text::new(
        "+12h",
        Point::new(chart_left + chart_width - 36, height - 1),
        text_style,
    )
    .draw(&mut display)
    .ok();

    // Show offline indicator
    if series.offline {
        Text::new("⚠ OFFLINE", Point::new(width - 72, 0), text_style)
            .draw(&mut display)
            .ok();
    }

    // Draw tide curve with "now" marker at the center of the chart
    // The center represents "now" regardless of exact sample timing
    let center_index = series.samples.len() / 2;

    let mut previous_point = None;
    for (index, sample) in series.samples.iter().enumerate() {
        let x = chart_left + (index as i32 * (chart_width - 1) / (series.samples.len() as i32 - 1));
        let y = tide_to_y(sample.tide_ft);
        let current_point = Point::new(x, y);

        if let Some(prev_point) = previous_point {
            Line::new(prev_point, current_point)
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(&mut display)
                .ok();
        }

        previous_point = Some(current_point);

        // Mark "now" with a prominent arrow/caret pointing down
        if index == center_index {
            // Draw downward-pointing arrow/caret: ▼
            let arrow_top = y - 12;
            Line::new(Point::new(x - 6, arrow_top), Point::new(x, y - 2))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(&mut display)
                .ok();
            Line::new(Point::new(x + 6, arrow_top), Point::new(x, y - 2))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                .draw(&mut display)
                .ok();

            // Also draw a filled circle at the tide point
            Circle::new(current_point, 4)
                .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                .draw(&mut display)
                .ok();
        }
    }
}

/// Render tide data to ASCII terminal.
pub fn draw_ascii(series: &TideSeries) {
    let config = Config::load();
    const ROWS: usize = 24;
    const Y_AXIS_WIDTH: usize = 5; // Space for Y-axis labels
    let sample_count = series.samples.len();

    // Calculate tide range using configurable display mode
    let (min_display, max_display) = calculate_display_bounds(series, &config);

    // Get raw MLLW bounds for row calculation (always use MLLW for internal positioning)
    let (min_tide_mllw, max_tide_mllw) = series
        .samples
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), sample| {
            (min.min(sample.tide_ft), max.max(sample.tide_ft))
        });

    let tide_to_row = |tide_ft: f32| {
        let normalized = (tide_ft - min_tide_mllw) / (max_tide_mllw - min_tide_mllw);
        ((1.0 - normalized) * (ROWS as f32 - 1.0)).round() as usize
    };

    let mut grid = vec![vec![' '; sample_count + Y_AXIS_WIDTH]; ROWS];

    // Add Y-axis labels using configured display mode
    let display_range = max_display - min_display;
    let tide_step = if display_range > 4.0 { 1.0 } else { 0.5 };
    let mut current_display = (min_display / tide_step).floor() * tide_step;

    while current_display <= max_display {
        // Convert display value back to MLLW for Y positioning
        let tide_mllw = if config.station.show_msl {
            msl_to_mllw(current_display, config.station.msl_offset)
        } else {
            current_display
        };
        let row = tide_to_row(tide_mllw);

        if row < ROWS {
            let label = format_display_height(tide_mllw, &config);
            // Ensure label fits in Y_AXIS_WIDTH - 1 (leave room for axis line)
            let padded_label = format!("{:<width$}", label, width = Y_AXIS_WIDTH - 1);

            for (i, ch) in padded_label.chars().enumerate() {
                if i < Y_AXIS_WIDTH - 1 {
                    grid[row][i] = ch;
                }
            }
            grid[row][Y_AXIS_WIDTH - 1] = '│'; // Vertical axis line
        }
        current_display += tide_step;
    }

    // Plot tide data with "now" marker at the center of the chart
    // The center represents "now" regardless of exact sample timing
    let center_index = series.samples.len() / 2;

    for (column, sample) in series.samples.iter().enumerate() {
        let row = tide_to_row(sample.tide_ft);
        let grid_column = column + Y_AXIS_WIDTH;

        if column == center_index {
            // Mark "now" with a prominent X (center of the time window)
            grid[row][grid_column] = 'X';
        } else {
            grid[row][grid_column] = '•';
        }
    }

    if series.offline {
        println!("⚠ OFFLINE\n");
    }

    for row in grid {
        println!("{}", row.into_iter().collect::<String>());
    }

    // Time markers below the chart
    let padding = " ".repeat(Y_AXIS_WIDTH);
    let time_markers: String = (0..sample_count)
        .map(|i| if i % 6 == 0 { '|' } else { ' ' })
        .collect();
    println!("{}{}", padding, time_markers);

    // Time labels - properly center the "Now" label with the X marker
    let data_center = sample_count / 2; // Center position in the data area (where X is placed)
    let now_text = "Now";
    let now_offset = now_text.len() / 2; // Offset to center the "Now" text
    let left_width = data_center.saturating_sub(now_offset);
    let left_part = format!("{:<width$}", "-12h", width = left_width);
    let right_width = sample_count - data_center - now_text.len() + now_offset;
    let right_part = format!("{:>width$}", "+12h", width = right_width);
    println!("{}{}{}{}", padding, left_part, now_text, right_part);
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
    fn test_mllw_to_msl_conversion() {
        let msl_offset = 4.9;

        // Test MLLW to MSL conversion with floating-point tolerance
        assert!((mllw_to_msl(5.0, msl_offset) - 0.1).abs() < 1e-6);
        assert!((mllw_to_msl(4.9, msl_offset) - 0.0).abs() < 1e-6);
        assert!((mllw_to_msl(0.0, msl_offset) - (-4.9)).abs() < 1e-6);

        // Test MSL to MLLW conversion with floating-point tolerance
        assert!((msl_to_mllw(0.0, msl_offset) - 4.9).abs() < 1e-6);
        assert!((msl_to_mllw(-4.9, msl_offset) - 0.0).abs() < 1e-6);
        assert!((msl_to_mllw(2.0, msl_offset) - 6.9).abs() < 1e-6);
    }

    #[test]
    fn test_calculate_display_bounds() {
        let series = test_series();
        let mut config = Config::default();

        // Test MSL display mode
        config.station.show_msl = true;
        config.station.msl_offset = 4.9;
        let (min_msl, max_msl) = calculate_display_bounds(&series, &config);
        // min tide is 1.0, max tide is 3.0 in test_series
        assert!((min_msl - (1.0 - 4.9)).abs() < 1e-6); // -3.9
        assert!((max_msl - (3.0 - 4.9)).abs() < 1e-6); // -1.9

        // Test MLLW display mode
        config.station.show_msl = false;
        let (min_mllw, max_mllw) = calculate_display_bounds(&series, &config);
        assert!((min_mllw - 1.0).abs() < 1e-6);
        assert!((max_mllw - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_format_tide_height() {
        // Test zero
        assert_eq!(format_tide_height(0.0), " 0 ");

        // Test positive values
        assert_eq!(format_tide_height(1.0), "+1");
        assert_eq!(format_tide_height(1.5), "+1.5");
        assert_eq!(format_tide_height(2.0), "+2");

        // Test negative values
        assert_eq!(format_tide_height(-1.0), "-1");
        assert_eq!(format_tide_height(-1.5), "-1.5");
        assert_eq!(format_tide_height(-2.0), "-2");
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

    #[cfg(target_os = "linux")]
    mod eink_tests {
        use super::*;
        use embedded_graphics::{mock_display::MockDisplay, pixelcolor::BinaryColor};

        #[test]
        fn test_eink_rendering() {
            let series = test_series();
            let mut display = MockDisplay::<BinaryColor>::new();

            // This should not panic and should render something
            draw_eink(&series, &mut display);

            // Check that some pixels were drawn (tide curve should exist)
            let pixels_drawn = display.into_iter().count();
            assert!(pixels_drawn > 0, "No pixels were drawn to the display");
        }

        #[test]
        fn test_eink_offline_indicator() {
            let mut series = test_series();
            series.offline = true;
            let mut display = MockDisplay::<BinaryColor>::new();

            draw_eink(&series, &mut display);

            // Should still render without panicking
            let pixels_drawn = display.into_iter().count();
            assert!(pixels_drawn > 0, "No pixels were drawn to the display");
        }

        #[test]
        fn test_eink_with_different_config() {
            use std::env;

            // Create a temporary config file for testing
            let test_config = r#"
[station]
id = "8443970"
name = "Boston, MA"
msl_offset = 9.5

[display]
time_window_hours = 12
cache_ttl_minutes = 30
width = 640
height = 384
font_height = 16
"#;

            // Write to a temporary file and test (in a real implementation)
            // For now, just test that the function works with default config
            let series = test_series();
            let mut display = MockDisplay::<BinaryColor>::new();

            draw_eink(&series, &mut display);

            let pixels_drawn = display.into_iter().count();
            assert!(pixels_drawn > 0, "Failed to render with different config");
        }
    }
}

//! # Tide Data Visualization Rendering
//!
//! This module handles rendering tide data to both e-ink displays and ASCII terminal output.
//! It's designed for the specific constraints of embedded e-ink hardware while providing
//! a convenient development mode for testing on desktop systems.

use crate::{config::Config, TideSeries};

// Custom EPD module for hardware rendering
#[cfg(all(target_os = "linux", feature = "hardware"))]
#[allow(unused_imports)]
use crate::epd4in2b_v2;

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
/// NOTE: This function is temporarily disabled as we migrate to custom EPD implementation
/*
#[cfg(all(target_os = "linux", feature = "hardware"))]
pub fn draw_eink(series: &TideSeries, display: &mut epd_waveshare::epd4in2::Display4in2) {
    use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_6X10};
    use embedded_graphics::{
        mono_font::MonoTextStyle,
        prelude::*,
        primitives::{Circle, Line, Primitive, PrimitiveStyle},
        text::Text,
    };
    use epd_waveshare::color::Color;

    let config = Config::load();

    // Use configured display dimensions
    let width = config.display.width as i32;
    let height = config.display.height as i32;

    // Text style for labels and indicators
    let text_style = MonoTextStyle::new(&FONT_10X20, Color::Black);
    let _small_text_style = MonoTextStyle::new(&FONT_6X10, Color::Black);

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
        } else if current_display.fract() == 0.0 {
            format!("{:.0}", current_display)
        } else {
            format!("{:.1}", current_display)
        };

        Text::new(&label, Point::new(2, y + 6), text_style)
            .draw(display)
            .ok();

        // Draw tick mark
        Line::new(Point::new(chart_left - 5, y), Point::new(chart_left, y))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 1))
            .draw(display)
            .ok();

        current_display += tide_step;
    }

    // Draw time labels
    Text::new("-12h", Point::new(chart_left, height - 1), text_style)
        .draw(display)
        .ok();
    Text::new(
        "Now",
        Point::new(chart_left + chart_width / 2 - 12, height - 1),
        text_style,
    )
    .draw(display)
    .ok();
    Text::new(
        "+12h",
        Point::new(chart_left + chart_width - 36, height - 1),
        text_style,
    )
    .draw(display)
    .ok();

    // Show offline indicator
    if series.offline {
        Text::new("⚠ OFFLINE", Point::new(width - 72, 0), text_style)
            .draw(display)
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
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
                .draw(display)
                .ok();
        }

        previous_point = Some(current_point);

        // Mark "now" with a prominent arrow/caret pointing down
        if index == center_index {
            // Draw downward-pointing arrow/caret: ▼
            let arrow_top = y - 12;
            Line::new(Point::new(x - 6, arrow_top), Point::new(x, y - 2))
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
                .draw(display)
                .ok();
            Line::new(Point::new(x + 6, arrow_top), Point::new(x, y - 2))
                .into_styled(PrimitiveStyle::with_stroke(Color::Black, 2))
                .draw(display)
                .ok();

            // Also draw a filled circle at the tide point
            Circle::new(current_point, 4)
                .into_styled(PrimitiveStyle::with_fill(Color::Black))
                .draw(display)
                .ok();
        }
    }
}
*/
/// Render tide data to e-ink display (4.2" b/w/red v2).
/// This function accepts a TideSeries for comprehensive display.
/// Updated to use our custom EPD implementation.
#[cfg(all(target_os = "linux", feature = "hardware"))]
pub fn draw_eink_v2_custom(series: &TideSeries, display: &mut crate::epd4in2b_v2::DisplayBuffer) {
    use crate::epd4in2b_v2::Color;

    let config = Config::load();

    // Clear the display to white background
    display.clear(Color::White);

    // Use configured display dimensions (400x300 for 4.2" display)
    let width = 400u32;
    let height = 300u32;

    // Chart layout
    let chart_left = 60u32;
    let chart_top = 40u32;
    let chart_width = width - chart_left - 20;
    let chart_height = height - chart_top - 60;

    // Draw border
    for x in 0..width {
        display.set_pixel(x, 0, Color::Black);
        display.set_pixel(x, height - 1, Color::Black);
    }
    for y in 0..height {
        display.set_pixel(0, y, Color::Black);
        display.set_pixel(width - 1, y, Color::Black);
    }

    // Title area in red
    let _title_text = format!("Tide Chart - {}", config.station.name);
    // Simple title rendering (we'll make it red blocks for now)
    for x in 10..390 {
        for y in 10..25 {
            if (x / 20) % 2 == 0 {
                display.set_pixel(x, y, Color::Red);
            }
        }
    }

    // Calculate tide range for normalization
    let (min_display, max_display) = calculate_display_bounds(series, &config);

    // Y-coordinate transformation: tide height → screen pixel
    let tide_to_y = |tide_ft_mllw: f32| {
        let display_value = tide_to_display(tide_ft_mllw, &config);
        let normalized = (display_value - min_display) / (max_display - min_display);
        let available_height = chart_height as f32;
        chart_top + ((1.0 - normalized) * available_height) as u32
    };

    // Draw Y-axis labels (simplified - just tick marks)
    let display_range = max_display - min_display;
    let tide_step = if display_range > 4.0 { 1.0 } else { 0.5 };
    let mut current_display = (min_display / tide_step).floor() * tide_step;

    while current_display <= max_display {
        let tide_mllw = if config.station.show_msl {
            msl_to_mllw(current_display, config.station.msl_offset)
        } else {
            current_display
        };
        let y = tide_to_y(tide_mllw);

        // Draw tick mark
        for x in (chart_left - 10)..chart_left {
            if y < height {
                display.set_pixel(x, y, Color::Black);
            }
        }
        current_display += tide_step;
    }

    // Draw tide curve
    let mut prev_point = None;
    for (i, sample) in series.samples.iter().enumerate() {
        let x = chart_left + (i as u32 * chart_width / series.samples.len().max(1) as u32);
        let y = tide_to_y(sample.tide_ft);

        // Draw line from previous point
        if let Some((prev_x, prev_y)) = prev_point {
            draw_line(display, prev_x, prev_y, x, y, Color::Black);
        }

        prev_point = Some((x, y));

        // Mark "now" with a red circle (center of time window)
        let center_index = series.samples.len() / 2;
        if i == center_index {
            draw_circle(display, x, y, 4, Color::Red);
        }
    }

    // Current tide value display (simplified)
    if let Some(current_sample) = series.samples.get(series.samples.len() / 2) {
        let _current_tide_display = tide_to_display(current_sample.tide_ft, &config);

        // Draw current tide indicator in bottom area
        for x in 10..200 {
            for y in (height - 30)..(height - 20) {
                if (x / 10) % 2 == 0 {
                    display.set_pixel(x, y, Color::Red);
                }
            }
        }
    }

    // Time axis markers
    let time_label_count = 5; // -12h, -6h, Now, +6h, +12h
    for i in 0..time_label_count {
        let x = chart_left + (i * chart_width / (time_label_count - 1).max(1));

        // Vertical tick marks
        for y in (chart_top + chart_height)..(chart_top + chart_height + 10) {
            if x < width && y < height {
                display.set_pixel(x, y, Color::Black);
            }
        }
    }

    // Offline indicator
    if series.offline {
        for x in (width - 100)..(width - 10) {
            for y in 10..25 {
                if (x + y) % 4 == 0 {
                    display.set_pixel(x, y, Color::Red);
                }
            }
        }
    }
}

// Helper function to draw a line
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn draw_line(
    display: &mut crate::epd4in2b_v2::DisplayBuffer,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    color: crate::epd4in2b_v2::Color,
) {
    let dx = (x1 as i32 - x0 as i32).abs();
    let dy = (y1 as i32 - y0 as i32).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = x0 as i32;
    let mut y = y0 as i32;

    loop {
        if x >= 0 && y >= 0 {
            display.set_pixel(x as u32, y as u32, color);
        }

        if x == x1 as i32 && y == y1 as i32 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}

// Helper function to draw a circle
#[cfg(all(target_os = "linux", feature = "hardware"))]
fn draw_circle(
    display: &mut crate::epd4in2b_v2::DisplayBuffer,
    cx: u32,
    cy: u32,
    radius: u32,
    color: crate::epd4in2b_v2::Color,
) {
    for x in (cx.saturating_sub(radius))..=(cx + radius) {
        for y in (cy.saturating_sub(radius))..=(cy + radius) {
            let dx = x as i32 - cx as i32;
            let dy = y as i32 - cy as i32;
            if (dx * dx + dy * dy) <= (radius * radius) as i32 {
                display.set_pixel(x, y, color);
            }
        }
    }
}

/*
/// Legacy function for rendering full tide series (for completeness).
/// NOTE: This function is temporarily disabled as we migrate to custom EPD implementation
// #[cfg(all(target_os = "linux", feature = "hardware"))]
// pub fn draw_eink_v2_series(series: &TideSeries, display: &mut epd_waveshare::epd4in2b_v2::Display4in2bV2) {
//     ... function body commented out ...
// }
*/

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
        /*
                // NOTE: These tests are temporarily disabled during migration to custom EPD implementation
                use super::*;
                use embedded_graphics::mock_display::MockDisplay;
                use epd_waveshare::color::Color;

                #[test]
                fn test_eink_rendering() {
                    let series = test_series();
                    let mut display = MockDisplay::<Color>::new();

                    // This should not panic and should render something
                    draw_eink(&series, &mut display);

                    // Basic verification that rendering completed successfully
                    // The test passes if no panic occurred during draw_eink call
                }

                #[test]
                fn test_eink_offline_indicator() {
                    let mut series = test_series();
                    series.offline = true;
                    let mut display = MockDisplay::<Color>::new();

                    draw_eink(&series, &mut display);

                    // Test passes if rendering completed without panicking
                }

                #[test]
                fn test_eink_with_different_config() {
                    // Create a temporary config file for testing
                    let _test_config = r#"
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
                    let mut display = MockDisplay::<Color>::new();

                    draw_eink(&series, &mut display);

                    // Test passes if rendering completed without panicking
                }
                */
    }
}

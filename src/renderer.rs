use crate::{config::Config, TideSeries};

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
            min_tide_mllw - config.station.msl_offset,
            max_tide_mllw - config.station.msl_offset,
        )
    } else {
        // Use raw MLLW values (0-9 feet typically)
        (min_tide_mllw, max_tide_mllw)
    }
}

/// Convert a tide height to display value based on configuration
fn tide_to_display(tide_ft_mllw: f32, config: &Config) -> f32 {
    if config.station.show_msl {
        tide_ft_mllw - config.station.msl_offset
    } else {
        tide_ft_mllw
    }
}

/// Convert tide height from MSL (Mean Sea Level) back to MLLW (Mean Lower Low Water)
fn msl_to_mllw(tide_ft_msl: f32, msl_offset: f32) -> f32 {
    tide_ft_msl + msl_offset
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

//! E-ink specific tide chart renderer
//!
//! This module provides a clean, simple tide chart renderer optimized for
//! the 4.2" B/W/Red e-ink display. It follows the drawing patterns from
//! the Waveshare C examples for maximum reliability.

use crate::epd4in2b_v2::{Color, DisplayBuffer};
use crate::TideSeries;
use chrono::Local;

/// E-ink tide chart renderer
pub struct EinkTideRenderer {
    width: u32,
    height: u32,
    margin: u32,
}

impl EinkTideRenderer {
    pub fn new() -> Self {
        Self {
            width: 400,
            height: 300,
            margin: 20,
        }
    }

    /// Render a complete tide chart to the display buffer
    pub fn render_chart(&self, buffer: &mut DisplayBuffer, tide_series: &TideSeries) {
        eprintln!("🎨 Rendering SIMPLIFIED tide chart to e-ink display...");
        eprintln!(
            "   📊 Tide series has {} samples",
            tide_series.samples.len()
        );

        // Chart area (with margins)
        let chart_x = self.margin;
        let chart_y = self.margin;
        let chart_width = self.width - (2 * self.margin);
        let chart_height = self.height - (2 * self.margin);

        eprintln!(
            "   📐 Chart area: {}x{} at ({}, {})",
            chart_width, chart_height, chart_x, chart_y
        );

        // 1. Draw chart border - this should always work
        eprintln!("   🔲 Drawing border...");
        self.draw_border(buffer, chart_x, chart_y, chart_width, chart_height);

        // 2. Draw basic axes - this should always work
        eprintln!("   📏 Drawing axes...");
        self.draw_simple_axes(buffer, chart_x, chart_y, chart_width, chart_height);

        // 3. Draw a test sine wave regardless of data - ensures we can see rendering
        eprintln!("   📈 Drawing test wave pattern...");
        self.draw_test_wave(buffer, chart_x, chart_y, chart_width, chart_height);

        // 4. Draw current time marker (center line)
        eprintln!("   🕐 Drawing center time marker...");
        self.draw_center_marker(buffer, chart_x, chart_y, chart_width, chart_height);

        // 5. Try to plot real data if available
        if !tide_series.samples.is_empty() {
            eprintln!("   📊 Attempting to plot real tide data...");
            self.plot_tide_data_simple(
                buffer,
                tide_series,
                chart_x,
                chart_y,
                chart_width,
                chart_height,
            );
        } else {
            eprintln!("   ⚠️  No tide data available - showing test pattern only");
        }

        eprintln!("✅ Simplified tide chart rendering complete");
    }

    /// Draw a simple border around the chart area
    fn draw_border(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, width: u32, height: u32) {
        eprintln!("   🔲 Drawing chart border...");

        // Draw rectangle border (2px thick for visibility)
        for thickness in 0..2 {
            // Top and bottom
            for px in 0..width {
                buffer.set_pixel(x + px, y + thickness, Color::Black);
                buffer.set_pixel(x + px, y + height - 1 - thickness, Color::Black);
            }
            // Left and right
            for py in 0..height {
                buffer.set_pixel(x + thickness, y + py, Color::Black);
                buffer.set_pixel(x + width - 1 - thickness, y + py, Color::Black);
            }
        }
    }

    /// Draw simple axes without complex tick marks
    fn draw_simple_axes(
        &self,
        buffer: &mut DisplayBuffer,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!("   📏 Drawing simple axes...");

        // X-axis (bottom) - thick line
        for thickness in 0..3 {
            for px in 0..width {
                buffer.set_pixel(x + px, y + height - 15 + thickness, Color::Black);
            }
        }

        // Y-axis (left) - thick line
        for thickness in 0..3 {
            for py in 0..height - 15 {
                buffer.set_pixel(x + 15 + thickness, y + py, Color::Black);
            }
        }
    }

    /// Draw a test sine wave pattern to verify coordinates work
    fn draw_test_wave(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, width: u32, height: u32) {
        eprintln!("   🌊 Drawing test sine wave...");

        let plot_x = x + 20;
        let plot_width = width - 40;
        let plot_y = y + 20;
        let plot_height = height - 40;

        // Draw sine wave across the plot area
        for i in 0..plot_width {
            let angle = (i as f64 / plot_width as f64) * 4.0 * std::f64::consts::PI; // 2 full cycles
            let sine_value = angle.sin();

            // Convert to screen Y coordinate
            let wave_y =
                plot_y + plot_height / 2 - ((sine_value * (plot_height as f64 / 4.0)) as u32);

            // Draw 3x3 pixel dot for visibility
            for dx in 0..3 {
                for dy in 0..3 {
                    if plot_x + i + dx < x + width && wave_y + dy < y + height {
                        buffer.set_pixel(plot_x + i + dx, wave_y + dy, Color::Black);
                    }
                }
            }
        }
    }

    /// Draw center time marker
    fn draw_center_marker(
        &self,
        buffer: &mut DisplayBuffer,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!("   🕐 Drawing center marker...");

        let center_x = x + width / 2;

        // Draw thick red vertical line
        for py in 20..(height - 20) {
            for thickness in 0..4 {
                buffer.set_pixel(center_x + thickness, y + py, Color::Red);
            }
        }
    }

    /// Simple tide data plotting with error handling
    fn plot_tide_data_simple(
        &self,
        buffer: &mut DisplayBuffer,
        tide_series: &TideSeries,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!("   📊 Simple tide data plotting...");

        let samples = &tide_series.samples;
        if samples.len() < 2 {
            eprintln!("   ⚠️  Need at least 2 samples for plotting");
            return;
        }

        // Use all available samples for now
        let min_height = samples
            .iter()
            .map(|s| s.tide_ft)
            .fold(f32::INFINITY, f32::min);
        let max_height = samples
            .iter()
            .map(|s| s.tide_ft)
            .fold(f32::NEG_INFINITY, f32::max);
        let height_range = max_height - min_height;

        eprintln!(
            "   📊 Using {} samples, height range: {:.1} to {:.1} ft",
            samples.len(),
            min_height,
            max_height
        );

        if height_range <= 0.0 {
            eprintln!("   ⚠️  Invalid height range");
            return;
        }

        let plot_x = x + 20;
        let plot_width = width - 40;
        let plot_y = y + 20;
        let plot_height = height - 40;

        // Plot first 100 samples or all if fewer (avoid overplotting)
        let sample_count = samples.len().min(100);

        for i in 0..sample_count {
            let sample = &samples[i];

            // X coordinate: spread samples across plot width
            let screen_x = plot_x + (i as u32 * plot_width) / sample_count as u32;

            // Y coordinate: map height to screen position (flip Y axis)
            let height_progress = (sample.tide_ft - min_height) / height_range;
            let screen_y = plot_y + plot_height - (height_progress * plot_height as f32) as u32;

            // Draw bigger dots for visibility
            for dx in 0..2 {
                for dy in 0..2 {
                    if screen_x + dx < x + width && screen_y + dy < y + height {
                        buffer.set_pixel(screen_x + dx, screen_y + dy, Color::Red);
                    }
                }
            }
        }
    }

    /// Plot the actual tide data as a continuous line
    fn plot_tide_data(
        &self,
        buffer: &mut DisplayBuffer,
        tide_series: &TideSeries,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!(
            "   📈 Plotting tide data ({} samples)...",
            tide_series.samples.len()
        );

        if tide_series.samples.is_empty() {
            return;
        }

        // Find time range: 12 hours before to 12 hours after current time
        let _now = Local::now();
        // Filter samples - we already have 24 hours of data with mins_rel from -720 to +720
        let filtered_samples: Vec<_> = tide_series
            .samples
            .iter()
            .filter(|sample| {
                sample.mins_rel >= -720 && sample.mins_rel <= 720 // 12 hours = 720 minutes each way
            })
            .collect();

        if filtered_samples.is_empty() {
            eprintln!("   ⚠️  No samples in 24-hour window");
            return;
        }

        // Find min/max heights for scaling
        let min_height = filtered_samples
            .iter()
            .map(|s| s.tide_ft as f64)
            .fold(f64::INFINITY, f64::min);
        let max_height = filtered_samples
            .iter()
            .map(|s| s.tide_ft as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let height_range = max_height - min_height;

        eprintln!(
            "   📊 Height range: {:.2} to {:.2} feet",
            min_height, max_height
        );

        // Plot area (inside axes)
        let plot_x = x + 15;
        let plot_y = y + 15;
        let plot_width = width - 30;
        let plot_height = height - 30;

        // Plot each sample as a point and connect with lines
        let mut prev_screen_x = None;
        let mut prev_screen_y = None;

        for sample in filtered_samples {
            // Convert time to X coordinate (mins_rel goes from -720 to +720)
            let time_progress = (sample.mins_rel + 720) as f64 / 1440.0; // 1440 = 24 hours in minutes
            let screen_x = plot_x + (time_progress * plot_width as f64) as u32;

            // Convert height to Y coordinate (flip Y axis - higher values at top)
            let height_progress = (sample.tide_ft as f64 - min_height) / height_range;
            let screen_y = plot_y + plot_height - (height_progress * plot_height as f64) as u32;

            // Draw point (2x2 pixel)
            for dx in 0..2 {
                for dy in 0..2 {
                    if screen_x + dx < x + width && screen_y + dy < y + height {
                        buffer.set_pixel(screen_x + dx, screen_y + dy, Color::Black);
                    }
                }
            }

            // Draw line from previous point
            if let (Some(prev_x), Some(prev_y)) = (prev_screen_x, prev_screen_y) {
                self.draw_line(buffer, prev_x, prev_y, screen_x, screen_y);
            }

            prev_screen_x = Some(screen_x);
            prev_screen_y = Some(screen_y);
        }
    }

    /// Draw current time marker (red vertical line)
    fn draw_current_time_marker(
        &self,
        buffer: &mut DisplayBuffer,
        _tide_series: &TideSeries,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!("   🕐 Drawing current time marker...");

        // Current time (mins_rel = 0) is at 50% of the timeline (12 hours into 24-hour window)
        let plot_x = x + 15;
        let plot_width = width - 30;
        let marker_x = plot_x + plot_width / 2;

        // Draw red vertical line for "NOW"
        for py in 15..(height - 15) {
            buffer.set_pixel(marker_x, y + py, Color::Red);
            // Make it 3px wide for visibility
            buffer.set_pixel(marker_x + 1, y + py, Color::Red);
            buffer.set_pixel(marker_x + 2, y + py, Color::Red);
        }
    }

    /// Add simple text labels
    fn draw_labels(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, width: u32, height: u32) {
        eprintln!("   🏷️  Drawing labels...");

        // For now, just draw simple markers where text would go
        // We can add proper text rendering later

        // "NOW" marker at center
        let center_x = x + width / 2;

        // Draw small squares where labels would be
        for dx in 0..10 {
            for dy in 0..5 {
                buffer.set_pixel(center_x - 5 + dx, y + height - 8 + dy, Color::Red);
            }
        }
    }

    /// Simple line drawing using Bresenham's algorithm
    fn draw_line(&self, buffer: &mut DisplayBuffer, x0: u32, y0: u32, x1: u32, y1: u32) {
        let mut x0 = x0 as i32;
        let mut y0 = y0 as i32;
        let x1 = x1 as i32;
        let y1 = y1 as i32;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            if x0 >= 0 && y0 >= 0 && x0 < 400 && y0 < 300 {
                buffer.set_pixel(x0 as u32, y0 as u32, Color::Black);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }
}

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
            margin: 20, // Increased to 20 to give more space for text labels
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
        self.draw_simple_axes(
            buffer,
            chart_x,
            chart_y,
            chart_width,
            chart_height,
            tide_series,
        );

        // 3. Draw current time marker (center line)
        eprintln!("   🕐 Drawing center time marker...");
        self.draw_center_marker(buffer, chart_x, chart_y, chart_width, chart_height);

        // 4. Plot real tide data with time-based coordinates
        if !tide_series.samples.is_empty() {
            eprintln!("   📊 Plotting real tide data with TIME-BASED coordinates...");
            self.plot_tide_data_simple(
                buffer,
                tide_series,
                chart_x,
                chart_y,
                chart_width,
                chart_height,
            );
        } else {
            eprintln!("   ⚠️  No tide data available - drawing test wave...");
            self.draw_test_wave(buffer, chart_x, chart_y, chart_width, chart_height);
        }

        eprintln!("✅ Simplified tide chart rendering complete");
    }

    /// Draw a simple border around the chart area
    fn draw_border(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, width: u32, height: u32) {
        eprintln!("   🔲 Drawing chart border with proper spacing for labels...");

        // Draw border at the edge of chart area, leaving margin space for text
        // No inset needed - the margin already provides space for labels
        let border_x = x;
        let border_y = y;
        let border_width = width;
        let border_height = height;

        // Draw rectangle border (2px thick for visibility)
        for thickness in 0..2 {
            // Top and bottom
            for px in 0..border_width {
                buffer.set_pixel(border_x + px, border_y + thickness, Color::Black);
                buffer.set_pixel(
                    border_x + px,
                    border_y + border_height - 1 - thickness,
                    Color::Black,
                );
            }
            // Left and right
            for py in 0..border_height {
                buffer.set_pixel(border_x + thickness, border_y + py, Color::Black);
                buffer.set_pixel(
                    border_x + border_width - 1 - thickness,
                    border_y + py,
                    Color::Black,
                );
            }
        }
    }

    /// Draw simple axes with time labels
    fn draw_simple_axes(
        &self,
        buffer: &mut DisplayBuffer,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tide_series: &TideSeries,
    ) {
        eprintln!("   📏 Drawing axes with CORRECTED positioning...");
        eprintln!(
            "   📐 Chart coordinates: x={}, y={}, width={}, height={}",
            x, y, width, height
        );

        // Get tide height range for Y-axis labels
        let samples = &tide_series.samples;
        let (min_height, max_height) = if !samples.is_empty() {
            let min = samples
                .iter()
                .map(|s| s.tide_ft)
                .fold(f32::INFINITY, f32::min);
            let max = samples
                .iter()
                .map(|s| s.tide_ft)
                .fold(f32::NEG_INFINITY, f32::max);
            (min, max)
        } else {
            (0.0, 10.0) // Default range
        };
        let height_range = max_height - min_height;
        eprintln!(
            "   📊 Tide range: {:.1} to {:.1} ft",
            min_height, max_height
        );

        // Define proper chart plotting area (inside the border, with space for axes)
        let plot_margin = 15; // Space for axes within the chart area
        let plot_x = x + plot_margin;
        let plot_y = y + plot_margin;
        let plot_width = width - (2 * plot_margin);
        let plot_height = height - (2 * plot_margin);

        eprintln!(
            "   📐 Plot area: x={}, y={}, width={}, height={}",
            plot_x, plot_y, plot_width, plot_height
        );

        // X-axis: horizontal line at BOTTOM of plot area
        let x_axis_y = plot_y + plot_height;
        eprintln!("   📏 Drawing X-axis at y={}", x_axis_y);
        for thickness in 0..2 {
            for px in plot_x..(plot_x + plot_width) {
                if px < self.width && (x_axis_y + thickness) < self.height {
                    buffer.set_pixel(px, x_axis_y + thickness, Color::Black);
                }
            }
        }

        // Y-axis: vertical line at LEFT of plot area
        let y_axis_x = plot_x;
        eprintln!("   📏 Drawing Y-axis at x={}", y_axis_x);
        for thickness in 0..2 {
            for py in plot_y..(plot_y + plot_height) {
                if (y_axis_x + thickness) < self.width && py < self.height {
                    buffer.set_pixel(y_axis_x + thickness, py, Color::Black);
                }
            }
        }

        // Add Y-axis tick marks for tide heights
        eprintln!("   📏 Adding Y-axis tick marks...");
        let num_ticks = 4; // Show 5 tick marks (0-4)
        for i in 0..=num_ticks {
            let tick_y = plot_y + (i * plot_height / num_ticks);
            // Draw tick mark extending left from Y-axis
            for thickness in 0..2 {
                for tick_x in (y_axis_x - 5)..y_axis_x {
                    if tick_x < self.width && (tick_y + thickness) < self.height {
                        buffer.set_pixel(tick_x, tick_y + thickness, Color::Black);
                    }
                }
            }

            // Calculate the tide height for this tick (flip because screen Y increases downward)
            let tick_height = max_height - (i as f32 / num_ticks as f32) * height_range;

            // Draw simple height label to the left of Y-axis
            if y_axis_x >= 12 {
                let label_text = format!("{:.0}", tick_height);
                self.draw_simple_text(buffer, y_axis_x - 12, tick_y.saturating_sub(3), &label_text);
            }
        }

        // Time labels: BELOW the X-axis, well outside the plot area
        let label_y = x_axis_y + 10; // 10 pixels below X-axis for clearance
        eprintln!("   📝 Drawing LARGE time labels at y={}", label_y);

        // Check if label position is valid (need space for 12px tall text)
        if label_y + 12 < self.height {
            // "-12h" at left edge of plot area - LARGE TEXT
            self.draw_large_text(buffer, plot_x, label_y, "-12h");

            // "Now" at center of plot area - LARGE TEXT (centered)
            let center_x = plot_x + plot_width / 2;
            self.draw_large_text(buffer, center_x - 15, label_y, "Now");

            // "+12h" at right edge of plot area - LARGE TEXT (right-aligned)
            self.draw_large_text(buffer, plot_x + plot_width - 40, label_y, "+12h");
        } else {
            eprintln!(
                "   ⚠️  Skipping time labels - not enough space at y={}",
                label_y
            );
        }

        // Add simplified Y-axis labels for better readability
        self.draw_y_axis_labels(buffer, x, y, y_axis_x, height);

        eprintln!("   ✅ Axes drawn successfully");
    }

    /// Draw simple text using pixel patterns (basic but readable)
    fn draw_simple_text(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, text: &str) {
        // Simple text rendering - 5x7 pixel characters with spacing
        for (i, ch) in text.chars().enumerate() {
            let char_x = x + (i as u32 * 6); // 6 pixels per character (5 + 1 spacing)

            // Draw character based on simple patterns
            match ch {
                '-' => {
                    // Draw horizontal line in middle
                    for dx in 0..4 {
                        if char_x + dx < self.width && y + 3 < self.height {
                            buffer.set_pixel(char_x + dx, y + 3, Color::Black);
                        }
                    }
                }
                '1' => {
                    // Draw vertical line
                    for dy in 0..7 {
                        if char_x + 2 < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x + 2, y + dy, Color::Black);
                        }
                    }
                }
                '2' => {
                    // Draw a simple "2" pattern
                    for dx in 0..4 {
                        // Top line
                        if char_x + dx < self.width && y < self.height {
                            buffer.set_pixel(char_x + dx, y, Color::Black);
                        }
                        // Bottom line
                        if char_x + dx < self.width && y + 6 < self.height {
                            buffer.set_pixel(char_x + dx, y + 6, Color::Black);
                        }
                    }
                    // Middle diagonal and edges
                    if char_x + 3 < self.width && y + 3 < self.height {
                        buffer.set_pixel(char_x + 3, y + 3, Color::Black);
                    }
                }
                'h' => {
                    // Draw vertical line and horizontal connector
                    for dy in 0..7 {
                        if char_x < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x, y + dy, Color::Black);
                        }
                    }
                    for dx in 0..4 {
                        if char_x + dx < self.width && y + 3 < self.height {
                            buffer.set_pixel(char_x + dx, y + 3, Color::Black);
                        }
                    }
                }
                'N' | 'n' => {
                    // Draw "N" pattern
                    for dy in 0..7 {
                        if char_x < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x, y + dy, Color::Black);
                        }
                        if char_x + 3 < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x + 3, y + dy, Color::Black);
                        }
                    }
                    // Diagonal
                    for i in 0..4 {
                        if char_x + i < self.width && y + i < self.height {
                            buffer.set_pixel(char_x + i, y + i, Color::Black);
                        }
                    }
                }
                'o' => {
                    // Draw "o" pattern - simple rectangle
                    for dx in 1..4 {
                        if char_x + dx < self.width && y + 2 < self.height {
                            buffer.set_pixel(char_x + dx, y + 2, Color::Black);
                        }
                        if char_x + dx < self.width && y + 5 < self.height {
                            buffer.set_pixel(char_x + dx, y + 5, Color::Black);
                        }
                    }
                    for dy in 2..6 {
                        if char_x < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x, y + dy, Color::Black);
                        }
                        if char_x + 4 < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x + 4, y + dy, Color::Black);
                        }
                    }
                }
                'w' => {
                    // Draw "w" pattern
                    for dy in 0..7 {
                        if char_x < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x, y + dy, Color::Black);
                        }
                        if char_x + 4 < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x + 4, y + dy, Color::Black);
                        }
                    }
                    if char_x + 2 < self.width && y + 5 < self.height {
                        buffer.set_pixel(char_x + 2, y + 5, Color::Black);
                        buffer.set_pixel(char_x + 2, y + 6, Color::Black);
                    }
                }
                '+' => {
                    // Draw plus sign
                    for dx in 1..4 {
                        if char_x + dx < self.width && y + 3 < self.height {
                            buffer.set_pixel(char_x + dx, y + 3, Color::Black);
                        }
                    }
                    for dy in 1..6 {
                        if char_x + 2 < self.width && y + dy < self.height {
                            buffer.set_pixel(char_x + 2, y + dy, Color::Black);
                        }
                    }
                }
                _ => {
                    // Default: draw a small rectangle for unknown characters
                    for dx in 0..3 {
                        for dy in 0..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw large, bold text for better readability on e-ink display
    fn draw_large_text(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, text: &str) {
        // Large text rendering - 8x12 pixel characters for better readability
        for (i, ch) in text.chars().enumerate() {
            let char_x = x + (i as u32 * 10); // 10 pixels per character (8 + 2 spacing)

            // Draw character with thick strokes for high contrast
            match ch {
                '-' => {
                    // Draw thick horizontal line in middle
                    for dy in 5..7 {
                        for dx in 1..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                '1' => {
                    // Draw thick vertical line with top serif
                    for dy in 0..12 {
                        for dx in 3..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Top left serif
                    for dx in 1..4 {
                        if char_x + dx < self.width && y + 1 < self.height {
                            buffer.set_pixel(char_x + dx, y + 1, Color::Black);
                        }
                    }
                }
                '2' => {
                    // Draw thick "2" pattern
                    for dx in 1..7 {
                        // Top line (thick)
                        for dy in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        // Bottom line (thick)
                        for dy in 10..12 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Middle diagonal and edges (thick)
                    for dy in 5..7 {
                        for dx in 1..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Right edge (thick)
                    for dy in 2..6 {
                        for dx in 5..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'h' => {
                    // Draw thick "h" pattern
                    for dy in 0..12 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Horizontal bar (thick)
                    for dy in 5..7 {
                        for dx in 0..6 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Right vertical (thick)
                    for dy in 7..12 {
                        for dx in 4..6 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'N' | 'n' => {
                    // Draw thick "N" pattern
                    for dy in 0..12 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dx in 5..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Diagonal (thick)
                    for i in 0..6 {
                        for thickness in 0..2 {
                            if char_x + i + thickness < self.width
                                && y + (i * 2) + thickness < self.height
                            {
                                buffer.set_pixel(
                                    char_x + i + thickness,
                                    y + (i * 2) + thickness,
                                    Color::Black,
                                );
                            }
                        }
                    }
                }
                'o' => {
                    // Draw thick "o" pattern
                    for dx in 1..6 {
                        // Top and bottom (thick)
                        for dy in 3..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dy in 8..10 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Left and right sides (thick)
                    for dy in 3..10 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dx in 5..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'w' => {
                    // Draw thick "w" pattern
                    for dy in 3..12 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dx in 6..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Middle strokes (thick)
                    for dy in 8..12 {
                        for dx in 2..4 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dx in 4..6 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                '+' => {
                    // Draw thick plus sign
                    for dx in 2..6 {
                        for dy in 5..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    for dy in 2..10 {
                        for dx in 3..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                _ => {
                    // Default: draw a thick rectangle for unknown characters
                    for dx in 0..6 {
                        for dy in 0..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw bold text with enhanced thickness and spacing for maximum readability on e-ink
    fn draw_bold_text(&self, buffer: &mut DisplayBuffer, x: u32, y: u32, text: &str) {
        // Bold text rendering - 8x10 pixel characters with thick strokes and extra spacing
        for (i, ch) in text.chars().enumerate() {
            let char_x = x + (i as u32 * 10); // 10 pixels per character (8 + 2 spacing)

            // Draw character with very thick strokes for maximum contrast
            match ch {
                'H' => {
                    // Draw bold "H" - thick verticals with thick horizontal bar
                    for dy in 0..10 {
                        // Left vertical (3px thick)
                        for dx in 0..3 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        // Right vertical (3px thick)
                        for dx in 5..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Horizontal bar (3px thick)
                    for dy in 4..7 {
                        for dx in 0..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'i' => {
                    // Draw bold "i" - thick dot and thick vertical line
                    // Dot at top (3x3 pixels)
                    for dy in 0..3 {
                        for dx in 2..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Vertical line (3px thick)
                    for dy in 4..10 {
                        for dx in 2..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'M' => {
                    // Draw bold "M" - thick verticals with thick angled top
                    for dy in 0..10 {
                        // Left vertical (3px thick)
                        for dx in 0..3 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        // Right vertical (3px thick)
                        for dx in 5..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Top peak (thick)
                    for dy in 0..4 {
                        for dx in 2..6 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'd' => {
                    // Draw bold "d" - thick circle with thick right vertical
                    // Right vertical (full height, 3px thick)
                    for dy in 0..10 {
                        for dx in 5..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Circle part (thick strokes)
                    for dx in 1..5 {
                        // Top and bottom of circle (2px thick)
                        for dy in 3..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dy in 7..9 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Left side of circle (2px thick)
                    for dy in 5..7 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'L' => {
                    // Draw bold "L" - thick vertical with thick bottom horizontal
                    for dy in 0..10 {
                        // Left vertical (3px thick)
                        for dx in 0..3 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Bottom horizontal (3px thick)
                    for dy in 7..10 {
                        for dx in 0..7 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                'o' => {
                    // Draw bold "o" - thick oval
                    for dx in 1..7 {
                        // Top and bottom (2px thick)
                        for dy in 3..5 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dy in 7..9 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                    // Left and right sides (2px thick)
                    for dy in 3..9 {
                        for dx in 0..2 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                        for dx in 6..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
                _ => {
                    // Default: draw a thick block for unknown characters
                    for dx in 0..6 {
                        for dy in 0..8 {
                            if char_x + dx < self.width && y + dy < self.height {
                                buffer.set_pixel(char_x + dx, y + dy, Color::Black);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Draw Y-axis labels with improved positioning and enhanced readability
    fn draw_y_axis_labels(
        &self,
        buffer: &mut DisplayBuffer,
        _chart_x: u32,
        chart_y: u32,
        y_axis_x: u32,
        chart_height: u32,
    ) {
        eprintln!("   📏 Drawing enhanced Y-axis labels with better contrast...");

        // Simplified Y-axis labels - just show "Hi", "Mid", "Lo" for better readability
        // Position them well away from the Y-axis line and border
        let label_positions = [
            (chart_y + 30, "Hi"),                    // Near top
            (chart_y + chart_height / 2 - 6, "Mid"), // Center
            (chart_y + chart_height - 50, "Lo"),     // Near bottom
        ];

        for (y_pos, label) in label_positions {
            // Position labels to the LEFT of chart area, with extra space
            let label_x = if y_axis_x >= 40 { y_axis_x - 40 } else { 5 };
            eprintln!("   📝 Drawing \"{}\" at ({}, {})", label, label_x, y_pos);
            self.draw_bold_text(buffer, label_x, y_pos, label);
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

    /// Draw center time marker at the "now" position (where mins_rel = 0)
    fn draw_center_marker(
        &self,
        buffer: &mut DisplayBuffer,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        eprintln!("   🕐 Drawing \"now\" marker with DOTTED vertical line...");

        // Use same plot area calculation as axes
        let plot_margin = 15;
        let plot_x = x + plot_margin;
        let plot_y = y + plot_margin;
        let plot_width = width - (2 * plot_margin);
        let plot_height = height - (2 * plot_margin);

        // Center marker in the middle of the plot area
        let center_x = plot_x + plot_width / 2;

        eprintln!(
            "   📍 Drawing dotted \"now\" line at x={} (plot center)",
            center_x
        );

        // Draw dotted vertical line for "now" - within the plot area only
        let marker_start_y = plot_y; // Start at top of plot area
        let marker_end_y = plot_y + plot_height; // End at bottom of plot area (at X-axis)

        // Create dotted pattern: 4 pixels on, 4 pixels off
        for py in marker_start_y..marker_end_y {
            // Check if this pixel should be part of the dot pattern
            if (py - marker_start_y) % 8 < 4 {
                // 4 on, 4 off = 8 pixel cycle
                // Draw thicker dots (2px wide) for better visibility
                for thickness in 0..2 {
                    if center_x + thickness < self.width && py < self.height {
                        buffer.set_pixel(center_x + thickness, py, Color::Black);
                    }
                }
            }
        }

        eprintln!(
            "   ✅ Dotted \"now\" line drawn at x={} from y={} to y={}",
            center_x, marker_start_y, marker_end_y
        );
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
        eprintln!("   📊 Simple tide data plotting with TIME-BASED coordinates...");

        let samples = &tide_series.samples;
        if samples.len() < 2 {
            eprintln!("   ⚠️  Need at least 2 samples for plotting");
            return;
        }

        // Find tide height range (same as ASCII renderer)
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

        // Find time range (should be -720 to +720 minutes, i.e., -12h to +12h)
        let min_time = samples.iter().map(|s| s.mins_rel).min().unwrap_or(-720);
        let max_time = samples.iter().map(|s| s.mins_rel).max().unwrap_or(720);
        let time_range = (max_time - min_time) as f32;

        eprintln!(
            "   🕐 Time range: {} to {} minutes ({:.1}h to {:.1}h)",
            min_time,
            max_time,
            min_time as f32 / 60.0,
            max_time as f32 / 60.0
        );

        // Define plot area that matches the axes coordinate system
        let plot_margin = 15; // Same as in draw_simple_axes
        let plot_x = x + plot_margin;
        let plot_width = width - (2 * plot_margin);
        let plot_y = y + plot_margin;
        let plot_height = height - (2 * plot_margin);

        eprintln!(
            "   📐 Plot area: {}x{} at ({}, {}) - matches axes system",
            plot_width, plot_height, plot_x, plot_y
        );

        // Plot each sample using TIME-BASED X coordinates (like ASCII renderer)
        for sample in samples {
            // X coordinate: map time to screen position (0 = left, plot_width = right)
            let time_progress = (sample.mins_rel - min_time) as f32 / time_range;
            let screen_x = plot_x + (time_progress * plot_width as f32) as u32;

            // Y coordinate: map height to screen position (flip Y axis for screen coordinates)
            let height_progress = (sample.tide_ft - min_height) / height_range;
            let screen_y = plot_y + plot_height - (height_progress * plot_height as f32) as u32;

            // Choose color and size based on proximity to "now"
            let is_now = sample.mins_rel.abs() <= 5; // Within 5 minutes of "now"
            let color = if is_now { Color::Red } else { Color::Black };
            let dot_size = if is_now { 5 } else { 2 }; // Much larger dot for "now"

            // Draw dots with variable size
            for dx in 0..dot_size {
                for dy in 0..dot_size {
                    if screen_x + dx < self.width && screen_y + dy < self.height {
                        buffer.set_pixel(screen_x + dx, screen_y + dy, color);
                    }
                }
            }

            // For "now" sample, draw a prominent X marker
            if is_now {
                eprintln!(
                    "   ❌ Drawing prominent \"NOW\" marker at tide curve position ({}, {})",
                    screen_x, screen_y
                );

                // Draw X pattern with thick lines
                let x_size = 8;
                for i in 0..x_size {
                    // Diagonal line from top-left to bottom-right
                    let x1 = screen_x.saturating_sub(x_size / 2) + i;
                    let y1 = screen_y.saturating_sub(x_size / 2) + i;
                    if x1 < self.width && y1 < self.height {
                        buffer.set_pixel(x1, y1, Color::Red);
                        // Make it thicker
                        if x1 + 1 < self.width {
                            buffer.set_pixel(x1 + 1, y1, Color::Red);
                        }
                        if y1 + 1 < self.height {
                            buffer.set_pixel(x1, y1 + 1, Color::Red);
                        }
                    }

                    // Diagonal line from top-right to bottom-left
                    let x2 = screen_x + x_size / 2 - i;
                    let y2 = screen_y.saturating_sub(x_size / 2) + i;
                    if x2 < self.width && y2 < self.height {
                        buffer.set_pixel(x2, y2, Color::Red);
                        // Make it thicker
                        if x2 + 1 < self.width {
                            buffer.set_pixel(x2 + 1, y2, Color::Red);
                        }
                        if y2 + 1 < self.height {
                            buffer.set_pixel(x2, y2 + 1, Color::Red);
                        }
                    }
                }
            }
        }

        eprintln!(
            "   ✅ Plotted {} tide samples with time-based coordinates",
            samples.len()
        );
    }

    /// Plot the actual tide data as a continuous line
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

impl Default for EinkTideRenderer {
    fn default() -> Self {
        Self::new()
    }
}

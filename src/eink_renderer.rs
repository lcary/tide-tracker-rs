//! E-ink tide chart renderer using embedded-graphics and epd-waveshare
//!
//! This module provides a clean, simple tide chart renderer optimized for
//! the 4.2" B/W/Red e-ink display. It follows the drawing patterns from
//! the Waveshare C examples for maximum reliability.

use crate::epd4in2b_v2::Epd4in2bV2;
use crate::TideSeries;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::*,
    text::Text,
};

/// Chart renderer for the Waveshare 4.2" e-ink display using embedded-graphics
pub struct EinkTideRenderer {
    pub width: u32,
    pub height: u32,
    pub margin: u32,
}

impl EinkTideRenderer {
    /// Create a new renderer for 400x300 e-ink panel
    pub fn new() -> Self {
        Self {
            width: 400,
            height: 300,
            margin: 20,
        }
    }

    /// Render a complete tide chart to the e-ink display
    pub fn render_chart<SPI, CS, DC, RST, BUSY, DT>(
        &self,
        epd: &mut Epd4in2bV2<SPI, CS, DC, RST, BUSY>,
        draw_target: &mut DT,
        tide: &TideSeries,
    ) where
        DT: DrawTarget<Color = BinaryColor>,
    {
        let chart_x = self.margin;
        let chart_y = self.margin;
        let chart_width = self.width - 2 * self.margin;
        let chart_height = self.height - 2 * self.margin;
        let plot_margin = 15;
        let plot_x = chart_x + plot_margin;
        let plot_y = chart_y + plot_margin;
        let plot_width = chart_width - 2 * plot_margin;
        let plot_height = chart_height - 2 * plot_margin;

        // Draw axes
        let axis_style = PrimitiveStyle::with_stroke(BinaryColor::On, 2);
        let x_axis = Line::new(
            Point::new(plot_x as i32, (plot_y + plot_height) as i32),
            Point::new((plot_x + plot_width) as i32, (plot_y + plot_height) as i32),
        );
        let y_axis = Line::new(
            Point::new(plot_x as i32, plot_y as i32),
            Point::new(plot_x as i32, (plot_y + plot_height) as i32),
        );
        x_axis.into_styled(axis_style).draw(draw_target).ok();
        y_axis.into_styled(axis_style).draw(draw_target).ok();

        // Draw Y-axis ticks and labels
        let num_ticks = 4;
        let samples = &tide.samples;
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
            (0.0, 10.0)
        };
        let height_range = max_height - min_height;
        let label_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
        for i in 0..=num_ticks {
            let tick_y = plot_y + (i * plot_height / num_ticks);
            let tick = Line::new(
                Point::new((plot_x - 5) as i32, tick_y as i32),
                Point::new(plot_x as i32, tick_y as i32),
            );
            tick.into_styled(axis_style).draw(draw_target).ok();
            let tick_height = max_height - (i as f32 / num_ticks as f32) * height_range;
            let label = format!("{:.0}", tick_height);
            Text::new(
                &label,
                Point::new((plot_x - 40) as i32, (tick_y - 6) as i32),
                label_style,
            )
            .draw(draw_target)
            .ok();
        }
        // Y-axis labels "Hi" and "Lo"
        Text::new(
            "Hi",
            Point::new((plot_x - 40) as i32, (plot_y + 30) as i32),
            label_style,
        )
        .draw(draw_target)
        .ok();
        Text::new(
            "Lo",
            Point::new((plot_x - 40) as i32, (plot_y + plot_height - 50) as i32),
            label_style,
        )
        .draw(draw_target)
        .ok();

        // Draw X-axis time labels
        let label_y = plot_y + plot_height + 10;
        if label_y + 12 < self.height {
            Text::new(
                "-12h",
                Point::new(plot_x as i32, label_y as i32),
                label_style,
            )
            .draw(draw_target)
            .ok();
            Text::new(
                "Now",
                Point::new((plot_x + plot_width / 2 - 15) as i32, label_y as i32),
                label_style,
            )
            .draw(draw_target)
            .ok();
            Text::new(
                "+12h",
                Point::new((plot_x + plot_width - 40) as i32, label_y as i32),
                label_style,
            )
            .draw(draw_target)
            .ok();
        }

        // Draw 'now' marker (dotted vertical line)
        let center_x = plot_x + plot_width / 2;
        let marker_style = PrimitiveStyle::with_stroke(BinaryColor::On, 2);
        let mut y = plot_y;
        while y < plot_y + plot_height {
            let end = (y + 4).min(plot_y + plot_height);
            Line::new(
                Point::new(center_x as i32, y as i32),
                Point::new(center_x as i32, end as i32),
            )
            .into_styled(marker_style)
            .draw(draw_target)
            .ok();
            y += 8;
        }

        // Draw tide data as polyline and highlight 'now' point
        if samples.len() >= 2 {
            let min_time = samples.iter().map(|s| s.mins_rel).min().unwrap_or(-720);
            let max_time = samples.iter().map(|s| s.mins_rel).max().unwrap_or(720);
            let time_range = (max_time - min_time) as f32;
            let mut prev: Option<Point> = None;
            for sample in samples {
                let time_progress = (sample.mins_rel - min_time) as f32 / time_range;
                let screen_x = plot_x + (time_progress * plot_width as f32) as u32;
                let height_progress = (sample.tide_ft - min_height) / height_range;
                let screen_y = plot_y + plot_height - (height_progress * plot_height as f32) as u32;
                let pt = Point::new(screen_x as i32, screen_y as i32);
                // Draw polyline
                if let Some(prev_pt) = prev {
                    Line::new(prev_pt, pt)
                        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                        .draw(draw_target)
                        .ok();
                }
                prev = Some(pt);
                // Draw 'now' marker as red circle
                if sample.mins_rel.abs() <= 5 {
                    Circle::new(pt, 8)
                        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 2))
                        .draw(draw_target)
                        .ok();
                } else {
                    Circle::new(pt, 3)
                        .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                        .draw(draw_target)
                        .ok();
                }
            }
        }
    }
}

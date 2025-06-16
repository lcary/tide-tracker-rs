//! # Fallback Tide Model
//!
//! This module provides a mathematical fallback when network-based tide data is unavailable.
//! It implements a simplified semidiurnal (twice-daily) tide model using a sine wave
//! approximation, which is sufficient for basic tide visualization when real data fails.
//!
//! ## Model Characteristics
//!
//! ### Semidiurnal Pattern
//! Most coastal areas experience semidiurnal tides (two high and two low tides per lunar day).
//! The mathematical model uses:
//! - **Period**: 12.42 hours (half a lunar day)
//! - **Amplitude**: 2.5 feet (reasonable for many coastal areas)
//! - **Mean level**: 5.0 feet (typical above chart datum)
//!
//! ### Memory Efficiency
//! - **Pre-computed constants**: All calculations use compile-time constants
//! - **Simple arithmetic**: Only sine function and basic multiplication
//! - **Fixed allocation**: `Vec::with_capacity(145)` prevents reallocation
//! - **No external dependencies**: Pure mathematical model
//!
//! ### Accuracy Trade-offs
//! This model sacrifices accuracy for simplicity and reliability:
//! - ✅ **Correct period**: Matches real semidiurnal tidal cycle
//! - ✅ **Reasonable range**: 2.5 to 7.5 feet covers typical coastal tides
//! - ❌ **No asymmetry**: Real tides have unequal high/low water heights
//! - ❌ **No phase**: Not synchronized with actual moon phase or location
//! - ❌ **No meteorological effects**: Ignores weather-driven tide variations
//!
//! The offline indicator ensures users understand they're seeing an approximation.

use crate::{Sample, TideSeries};

/// Generate an approximate tide series using a semidiurnal sine wave model.
///
/// This function creates a mathematically-derived tide prediction when real NOAA data
/// is unavailable due to network issues, server problems, or parsing failures.
///
/// The model assumes typical semidiurnal tides with:
/// - **12.42-hour period**: Based on lunar semidiurnal tide (M2 constituent)
/// - **2.5-foot amplitude**: Reasonable for mid-latitude coastal areas
/// - **5.0-foot mean**: Typical height above chart datum
///
/// # Memory Usage
/// - Pre-allocates exactly 145 samples (no reallocation)
/// - Uses stack-allocated constants and simple arithmetic
/// - Peak memory: ~900 bytes for the returned `TideSeries`
///
/// # Returns
/// A `TideSeries` with `offline = true` and 145 synthetic samples covering
/// 24 hours in 10-minute increments.
///
/// # Example
/// ```
/// use tide_clock_lib::fallback::approximate;
///
/// let series = approximate();
/// assert_eq!(series.samples.len(), 145);
/// assert!(series.offline);
///
/// // Check that we have reasonable tide range
/// let heights: Vec<f32> = series.samples.iter().map(|s| s.tide_ft).collect();
/// let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
/// let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
/// assert!(max_height - min_height > 4.0); // Significant tidal range
/// ```
pub fn approximate() -> TideSeries {
    // Semidiurnal tide period: 12 hours 25 minutes (12.42 hours)
    // This matches the M2 tidal constituent, the dominant tide component globally
    const PERIOD_HRS: f32 = 12.42;

    // Pre-allocate vector to exact size to prevent reallocation during construction
    let mut samples = Vec::with_capacity(145);

    // Generate samples from -12 hours to +12 hours in 10-minute increments
    for m in (-720..=720).step_by(10) {
        // Convert minutes to angular position in the sine wave
        // TAU (2π) represents one complete cycle
        let theta = (m as f32 / 60.0) * std::f32::consts::TAU / PERIOD_HRS;

        // Simple sine wave: mean_level + amplitude * sin(theta)
        // - Mean level: 5.0 feet (reasonable for many coastal areas)
        // - Amplitude: 2.5 feet (giving 2.5 to 7.5 foot range)
        let tide_ft = 5.0 + 2.5 * theta.sin();

        samples.push(Sample {
            mins_rel: m,
            tide_ft,
        });
    }

    // Mark as offline so display shows warning indicator
    TideSeries {
        samples,
        offline: true,
    }
}

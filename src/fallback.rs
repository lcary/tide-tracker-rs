//! # Fallback Tide Model
//!
//! This module provides a lunar-phase-aware mathematical fallback when network-based tide data is unavailable.
//! It implements a semidiurnal (twice-daily) tide model using a sine wave approximation, but now:
//!
//! - The 24-hour window is always centered on the *current* time, and the tide curve advances with real time
//! - The phase of the tide is tied to the real-time clock (not moon age)
//! - The amplitude of the tide curve is modulated by the current moon phase using the Schaefer Moon algorithm, peaking at new/full moon
//! - Spring–neap cycles are reflected in the amplitude envelope (higher tides at new/full, lower at quarters)
//!
//! ## Model Characteristics
//!
//! ### Semidiurnal Pattern
//! Most coastal areas experience semidiurnal tides (two high and two low tides per lunar day).
//! The model uses:
//! - **Period**: 12.42 hours (half a lunar day)
//! - **Mean level**: 5.0 feet (typical above chart datum)
//! - **Amplitude**: 4.5 feet (Portland M2) modulated by solar S2
//!
//! ### Lunar Phase & Amplitude
//! - The curve's phase is tied to the real-time clock (advances as time passes)
//! - The amplitude is modulated by a cosine envelope, peaking at new/full moon (cos(2φ))
//! - The sample at `mins_rel = 0` always reflects the current tide state for the current time
//!
//! ### Memory Efficiency
//! - **Pre-computed constants**: All calculations use stack scalars
//! - **Simple arithmetic**: Only sine/cosine and basic multiplication
//! - **Fixed allocation**: `Vec::with_capacity(145)` prevents reallocation
//! - **No external dependencies**: Pure mathematical model, except for chrono and lunar
//!
//! ### Accuracy Trade-offs
//! This model sacrifices accuracy for simplicity and reliability:
//! - ✅ **Correct period**: Matches real semidiurnal tidal cycle
//! - ✅ **Spring–neap envelope**: Amplitude modulated by moon phase
//! - ✅ **Phase alignment**: Window is centered on *now* and advances with real time
//! - ❌ **No asymmetry**: Real tides have unequal high/low water heights
//! - ❌ **No meteorological effects**: Ignores weather-driven tide variations
//! - ❌ **±1 day accuracy**: Not synchronized to local station, but tracks moon
//!
//! The offline indicator ensures users understand they're seeing an approximation.

use crate::{Sample, TideSeries};
use chrono::{DateTime, Datelike, Timelike, Utc};

/// Generate an approximate tide series for the next 24 h.
/// If `now` is `None`, fall back to `Utc::now()`.
///
/// The returned series is always centered on the current instant, and modulates
/// phase and amplitude using the Schaefer Moon algorithm.
pub fn approximate(now: Option<DateTime<Utc>>) -> TideSeries {
    // 1. Current instant
    let now = now.unwrap_or_else(Utc::now);

    let y = now.year();
    let m = now.month();
    let d = now.day() as f64
        + (now.hour() as f64 + now.minute() as f64 / 60.0 + now.second() as f64 / 3600.0) / 24.0;

    // 2. Moon ephemeris
    let eph = crate::lunar::schaefer_moon(y, m, d);
    let tau: f32 = std::f32::consts::TAU;

    // ---- 3. Two-constituent equilibrium tide -----------------------------

    // Lunar M2 amplitude for Portland, ME (NOAA harmonics)
    const A_M2: f32 = 4.51; // ft
    const P_M2_HRS: f32 = 12.42;

    // Solar S2 amplitude for Portland, ME
    const A_S2: f32 = 0.68; // ft
    const P_S2_HRS: f32 = 12.00;

    // High-water interval (Moon transit → local HW) ≈ 3 h 35 m
    const LUNITIDAL_OFFSET_HRS: f32 = 3.59; // hrs

    // Real-time phase of each constituent
    let daily_phase_m2 = ((now.timestamp() + (LUNITIDAL_OFFSET_HRS * 3600.0) as i64)
        .rem_euclid((P_M2_HRS * 3600.0) as i64) as f32)
        / (P_M2_HRS * 3600.0)
        * tau;

    let moon_phase_angle = (eph.age_days / 29.530_588_2) as f32 * tau;
    let daily_phase_s2 = daily_phase_m2 + 2.0 * moon_phase_angle;

    const MEAN_LEVEL_FT: f32 = 5.0; // chart datum offset
    let mut samples = Vec::with_capacity(145);
    for m in (-720..=720).step_by(10) {
        let theta_m2 = daily_phase_m2 + (m as f32 / 60.0) * tau / P_M2_HRS;
        let theta_s2 = daily_phase_s2 + (m as f32 / 60.0) * tau / P_S2_HRS;
        let tide_ft = MEAN_LEVEL_FT + A_M2 * theta_m2.sin() + A_S2 * theta_s2.sin();
        samples.push(Sample {
            mins_rel: m,
            tide_ft,
        });
    }

    TideSeries {
        samples,
        offline: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_approximate_changes_with_time() {
        // Midnight UTC
        let t0 = Utc.ymd(2025, 7, 24).and_hms(0, 0, 0);
        let series0 = approximate(Some(t0));
        let now_sample0 = series0.samples.iter().find(|s| s.mins_rel == 0).unwrap();

        // 6 hours later
        let t1 = t0 + chrono::Duration::hours(6);
        let series1 = approximate(Some(t1));
        let now_sample1 = series1.samples.iter().find(|s| s.mins_rel == 0).unwrap();

        // 12 hours later (should be different, but not necessarily opposite)
        let t2 = t0 + chrono::Duration::hours(12);
        let series2 = approximate(Some(t2));
        let now_sample2 = series2.samples.iter().find(|s| s.mins_rel == 0).unwrap();

        // The tide at now should change as time advances
        assert_ne!(
            now_sample0.tide_ft, now_sample1.tide_ft,
            "Tide at now should change with time"
        );
        assert_ne!(
            now_sample0.tide_ft, now_sample2.tide_ft,
            "Tide at now should change with time"
        );
        assert_ne!(
            now_sample1.tide_ft, now_sample2.tide_ft,
            "Tide at now should change with time"
        );

        // 12 hours apart should be significantly different (Portland: expect >0.25)
        let diff = (now_sample0.tide_ft - now_sample2.tide_ft).abs();
        assert!(
            diff > 0.25,
            "Tide at now should change after 12h (diff: {diff})"
        );
    }

    #[test]
    fn test_approximate_known_high_and_low() {
        // Pick a time and check if we can get a high or low at mins_rel=0
        let t0 = Utc.ymd(2025, 7, 24).and_hms(0, 0, 0);
        let series = approximate(Some(t0));
        let now_sample = series.samples.iter().find(|s| s.mins_rel == 0).unwrap();
        // Should be within the expected range for Portland, ME
        assert!(
            (-0.5..=9.5).contains(&now_sample.tide_ft),
            "Tide at now is in expected range"
        );

        // Try a time 6h12m later (half a tide period)
        let t1 = t0 + chrono::Duration::minutes((12.42 * 60.0 / 2.0) as i64);
        let series1 = approximate(Some(t1));
        let now_sample1 = series1.samples.iter().find(|s| s.mins_rel == 0).unwrap();
        // Should be significantly different
        let diff = (now_sample.tide_ft - now_sample1.tide_ft).abs();
        assert!(
            diff > 0.3,
            "Tide at now should change after half a period (diff: {diff})"
        );
    }
}

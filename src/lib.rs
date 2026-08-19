// # Tide Tracker Core Library
//
// This library provides the foundational data structures and types for the tide tracker
// application. It's designed for extreme memory efficiency on embedded systems like
// the Raspberry Pi Zero 2 W (512 MB RAM total).
//
// ## Design Philosophy
//
// ### Memory Efficiency
// - **Fixed-size data structures**: All containers use `Vec::with_capacity(145)` to
//   pre-allocate exactly the needed memory for 24 hours of 10-minute samples
// - **Minimal allocations**: Uses primitive types (`i16`, `f32`) to minimize memory overhead
// - **Serialization-friendly**: Structures implement `Serialize`/`Deserialize` for efficient
//   binary caching without additional heap allocations
//
// ### Temporal Resolution
// The application samples tide data every 10 minutes for 24 hours:
// - **145 samples total**: -720 to +720 minutes (24 hours) in 10-minute increments
// - **Smooth visualization**: 10-minute granularity provides much smoother curves than
//   traditional hourly sampling, critical for accurate tide prediction display
// - **Current time marker**: Sample with `mins_rel == 0` represents "now"
//
// ### Data Flow
// 1. **Live**: Fetch NOAA hilo predictions → interpolate to 10-minute grid → cache → display
// 2. **Cached**: Live fetch fails, but a station-matched cache still covers the display
//    window → interpolate from the cached predictions → mark with `stale_hours` → display.
//    This is still real, astronomically correct station data - just not freshly fetched.
// 3. **Synthetic**: No live data *and* no usable cache → mathematical fallback model →
//    mark `offline` → display. This is the only path that can show the wrong tide.
// 4. **Memory**: Peak usage < 1MB across entire data pipeline
//
// ## Core Types
//
// The library exports two primary types optimized for the embedded target:
// - [`Sample`]: A single tide measurement at a specific time
// - [`TideSeries`]: Complete 24-hour dataset with offline status indicator

use serde::{Deserialize, Serialize};

// Module declarations
pub mod config;
pub mod eink_renderer;
pub mod epd4in2b_v2;
pub mod fallback;
pub mod lunar;
pub mod renderer;
pub mod tide_data;

/// A single tide measurement at a specific time relative to "now".
///
/// This structure is carefully sized for memory efficiency:
/// - `i16` for time (±720 minutes = ±12 hours fits in 16 bits)
/// - `f32` for height (sufficient precision for tide measurements in feet)
/// - Total size: 6 bytes per sample
///
/// Time is stored as minutes relative to the current time:
/// - Negative values: past (e.g., -60 = 1 hour ago)
/// - Zero: current time (marked with filled circle on display)
/// - Positive values: future (e.g., +120 = 2 hours from now)
///
/// # Example
/// ```
/// use tide_clock_lib::Sample;
///
/// // Current tide height
/// let now_sample = Sample { mins_rel: 0, tide_ft: 3.2 };
///
/// // Tide 2 hours ago
/// let past_sample = Sample { mins_rel: -120, tide_ft: 1.8 };
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Sample {
    /// Minutes relative to current time (-720 to +720)
    pub mins_rel: i16,
    /// Tide height in feet
    pub tide_ft: f32,
}

/// Complete 24-hour tide dataset with metadata.
///
/// Contains exactly 145 samples covering 24 hours at 10-minute intervals,
/// plus two independent flags describing how trustworthy the data is:
/// `offline` (synthetic vs. real) and `stale_hours` (real, but not freshly
/// fetched). See each field's doc comment for the distinction - it matters
/// because they call for different UI treatment and, more importantly,
/// because only one of them means the tide curve might be *wrong*.
///
/// Memory layout:
/// - `Vec<Sample>`: 145 samples × 6 bytes = 870 bytes
/// - `bool`: 1 byte
/// - `Option<f32>`: 8 bytes (tag + payload)
/// - Vec overhead: ~24 bytes
/// - **Total**: ~900 bytes per series
///
/// # Offline Behavior
/// When `offline = true`, the data comes from a mathematical fallback model
/// rather than real NOAA predictions. The display shows "⚠ OFFLINE" to
/// inform users of reduced accuracy.
///
/// # Example
/// ```
/// use tide_clock_lib::{Sample, TideSeries};
///
/// let series = TideSeries {
///     samples: vec![
///         Sample { mins_rel: -10, tide_ft: 2.1 },
///         Sample { mins_rel: 0, tide_ft: 2.3 },
///         Sample { mins_rel: 10, tide_ft: 2.5 },
///     ],
///     offline: false,
///     stale_hours: None,
/// };
///
/// assert_eq!(series.samples.len(), 3);
/// assert!(!series.offline);
/// assert!(series.stale_hours.is_none());
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TideSeries {
    /// Exactly 145 tide samples spanning 24 hours
    pub samples: Vec<Sample>,
    /// True only for the synthetic sine-wave fallback model
    /// ([`fallback::approximate`]), which is not aligned to any station and
    /// should be treated as a rough approximation, not real data. This is
    /// orthogonal to `stale_hours`: synthetic data has no real "age" to
    /// report, so `stale_hours` is always `None` when `offline` is `true`.
    pub offline: bool,
    /// Age in hours of the cached predictions backing this series, or `None`
    /// when the data is live. Unlike `offline`, `Some` still means the curve
    /// is correct - predictions are astronomy, so a cached one covering the
    /// window is as accurate as a fresh fetch.
    #[serde(default)]
    pub stale_hours: Option<f32>,
}

impl TideSeries {
    /// Short label describing the data source, or `None` when it is live and
    /// fresh and therefore needs no annotation.
    ///
    /// Shared by both renderers so the wording and the staleness thresholds
    /// cannot drift apart. A sub-hour cache hit is routine, so it stays quiet.
    pub fn status_label(&self) -> Option<String> {
        if self.offline {
            return Some("OFFLINE!".to_string());
        }
        match self.stale_hours {
            Some(h) if h >= 48.0 => Some(format!("CACHED {}d", (h / 24.0).floor() as i64)),
            Some(h) if h >= 1.0 => Some(format!("CACHED {}h", h.round() as i64)),
            _ => None,
        }
    }
}

// Custom EPD module for hardware rendering (already declared above)

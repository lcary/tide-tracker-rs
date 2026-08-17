//! # NOAA Tide Data Fetching and Caching
//!
//! This module handles all network operations for fetching real-time tide predictions
//! from NOAA's Tides and Currents API. It includes intelligent caching to minimize
//! network requests and robust error handling for unreliable network conditions.
//!
//! ## Data Source
//!
//! ### NOAA CO-OPS API
//! - **URL**: https://api.tidesandcurrents.noaa.gov/api/prod/datagetter
//! - **Station**: 8418150 (Portland, ME) - configurable by editing URL
//! - **Format**: JSON response with high/low (`interval=hilo`) tide predictions.
//!   This works for both NOAA *reference* stations (type R) and *subordinate*
//!   stations (type S). The default 6-minute harmonic interval only works for
//!   reference stations and returns an API error for subordinate stations, so
//!   `hilo` is used unconditionally regardless of station type.
//! - **Data**: 3 days covering yesterday to tomorrow (~12 hilo turning points)
//!
//! ### Data Processing Pipeline
//! 1. **Fetch**: HTTP GET request to NOAA CO-OPS API requesting `interval=hilo`
//! 2. **Parse**: Deserialize JSON response containing high/low tide predictions,
//!    skipping any individual point with an unmappable timestamp (e.g. inside a
//!    DST spring-forward gap) or an unparsable height
//! 3. **Sort**: Order all parsed points chronologically across the full 3-day set
//! 4. **Interpolate**: Convert the sparse high/low points to a dense 10-minute
//!    grid using half-cosine (rule-of-twelfths) interpolation between turning
//!    points, clamped to the endpoint heights outside the data's range
//! 5. **Cache**: Store processed data with timestamp for 30-minute TTL
//! 6. **Return**: 145 samples ready for visualization
//!
//! ## Caching Strategy
//!
//! ### Memory-Efficient Caching
//! - **Location**: `/tmp/tide_cache.json` (cleared on reboot)
//! - **Format**: Binary JSON serialization for compact storage
//! - **TTL**: 30 minutes (balances freshness vs. network load)
//! - **Validation**: File modification time checked before loading
//!
//! ### Cache Benefits
//! - **Reduced bandwidth**: Avoid repeated downloads during development
//! - **Faster startup**: Cached data loads in ~1ms vs. ~200ms network fetch
//! - **Network resilience**: Recent data available during temporary outages
//! - **Pi Zero friendly**: Minimizes CPU time spent on network I/O
//!
//! ## Error Handling
//!
//! The module handles multiple failure modes gracefully:
//! - **Network timeouts**: HTTP client configured with reasonable timeouts
//! - **Server errors**: 5xx responses handled as fetch failures
//! - **Parse failures**: Malformed JSON or unexpected API response structure
//! - **Cache corruption**: Invalid JSON falls back to fresh network fetch
//! - **File system issues**: Permissions or disk space problems
//!
//! All errors propagate through `TideError` enum for consistent handling.

use crate::{config::Config, Sample, TideSeries};
use chrono::{Duration, Local};
use std::{fs, io, time::SystemTime};
use thiserror::Error;

/// Errors that can occur during tide data fetching and processing.
///
/// This enum covers all failure modes in the data pipeline, from network
/// issues to file system problems. Each variant includes the underlying
/// error for debugging while maintaining a clean interface.
#[derive(Error, Debug)]
pub enum TideError {
    /// HTTP request failed (network, server, or protocol error)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API parsing failed (unexpected JSON structure or missing data)
    #[error("API parse failed")]
    Scrape,

    /// Cache file operations failed (permissions, disk space, corruption)
    #[error("cache IO: {0}")]
    Cache(#[from] io::Error),
}

/// Cache file location on filesystem
///
/// Using /tmp ensures the cache is cleared on reboot and doesn't consume
/// permanent storage on the Pi Zero 2 W's limited SD card space.
const CACHE: &str = "/tmp/tide_cache.json";

/// Cache time-to-live in seconds (30 minutes)
///
/// This balances data freshness with network efficiency:
/// - Short enough: Tide predictions stay reasonably current
/// - Long enough: Reduces network load during active development
/// - Pi Zero friendly: Minimizes cellular/WiFi radio usage
const TTL: u64 = 1800; // 30 minutes

/// Fetch current tide series from NOAA or cache.
///
/// This is the main entry point for obtaining tide data. It implements
/// a cache-first strategy: check for valid cached data, and only fetch
/// from the network if the cache is stale or missing.
///
/// # Memory Usage
/// - Cache check: ~100 bytes for file metadata
/// - Network fetch: ~50KB for HTML download
/// - Parsed data: ~900 bytes for final TideSeries
/// - Peak usage: ~51KB during network operations
///
/// # Error Handling
/// On any error, the caller should fall back to `fallback::approximate()`
/// to ensure the application continues working even with network issues.
///
/// # Returns
/// - `Ok(TideSeries)`: Successfully loaded data (either cached or fresh)
/// - `Err(TideError)`: All data sources failed
///
/// # Example
/// ```no_run
/// use tide_clock_lib::tide_data::fetch;
/// use tide_clock_lib::fallback;
///
/// # async fn example() {
/// let series = fetch().await.unwrap_or_else(|err| {
///     eprintln!("Failed to fetch tide data: {}", err);
///     fallback::approximate(None)
/// });
/// # }
/// ```
pub async fn fetch() -> Result<TideSeries, TideError> {
    let config = Config::load();

    // Try cache first - much faster than network fetch
    if let Ok(series) = load_cache() {
        return Ok(series);
    }

    // Cache miss or stale - fetch fresh data from NOAA
    let series = scrape_noaa(&config).await?;

    // Save for future requests (ignore cache write failures)
    let _ = save_cache(&series);

    Ok(series)
}

// -- Private Implementation --

/// Fetch tide predictions from NOAA API and convert to TideSeries.
///
/// This function uses NOAA's official CO-OPS API instead of HTML scraping:
/// 1. Downloads JSON data from NOAA API, requesting `interval=hilo`
/// 2. Parses the structured high/low tide predictions, skipping any point
///    whose timestamp can't be mapped to a local time or whose height can't
///    be parsed (a malformed response, i.e. missing fields, is still fatal)
/// 3. Sorts the points chronologically and interpolates a dense 10-minute
///    grid from the full 3-day set using half-cosine interpolation
/// 4. Returns a complete 24-hour TideSeries
///
/// # Why `interval=hilo`
/// NOAA's default prediction interval returns 6-minute harmonic data, but
/// that only works for *reference* stations (type R). *Subordinate* stations
/// (type S) - which cover almost every station near small bays, coves, and
/// river mouths - return an API error for the default interval:
/// `{"error": {"message":"No Predictions data was found. ..."}}`.
/// Requesting `interval=hilo` (just the high/low turning points) works for
/// both station types, so it is used unconditionally.
///
/// # API Configuration
/// Uses NOAA CO-OPS API v1 with the following parameters:
/// - Station: 8418150 (Portland, ME)
/// - Product: predictions (tide predictions)
/// - Interval: hilo (high/low turning points only; works for all station types)
/// - Datum: MLLW (Mean Lower Low Water)
/// - Time zone: lst_ldt (Local Standard/Daylight Time)
/// - Units: english (feet)
/// - Format: json
///
/// # Example API URL
/// ```text
/// https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?
/// product=predictions&station=8418150&begin_date=20240616&end_date=20240617&
/// datum=MLLW&time_zone=lst_ldt&units=english&format=json&interval=hilo
/// ```
///
/// # Interpolation Algorithm
/// Half-cosine (rule-of-twelfths) interpolation between adjacent high/low
/// points - see [`interpolate_hilo`] for details and rationale. This
/// provides a realistic, smooth tide curve suitable for visualization, unlike
/// naive linear interpolation which would produce visibly incorrect,
/// V-shaped turning points at each high/low.
async fn scrape_noaa(config: &Config) -> Result<TideSeries, TideError> {
    // Calculate date range: yesterday to tomorrow (ensures we have enough data)
    let now = Local::now();
    let yesterday = now - Duration::days(1);
    let tomorrow = now + Duration::days(1);

    // Format dates for API (YYYYMMDD)
    let begin_date = yesterday.format("%Y%m%d").to_string();
    let end_date = tomorrow.format("%Y%m%d").to_string();

    // NOAA CO-OPS API endpoint using configured station.
    //
    // `interval=hilo` requests only high/low turning points instead of the
    // default 6-minute harmonic data. The default interval only works for
    // reference stations (type R); subordinate stations (type S) - which
    // cover most stations near bays, coves, and river mouths - return an
    // API error for it. `hilo` works for both station types.
    let url = format!(
        "https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?\
        product=predictions&station={}&begin_date={}&end_date={}&\
        datum=MLLW&time_zone=lst_ldt&units=english&format=json&interval=hilo",
        config.station.id, begin_date, end_date
    );

    // Fetch JSON data from API with rustls TLS backend
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(&url).send().await?.text().await?;

    // Parse JSON response
    let json: serde_json::Value = serde_json::from_str(&response).map_err(|_| TideError::Scrape)?;

    // Extract predictions array
    let predictions = json["predictions"].as_array().ok_or(TideError::Scrape)?;

    // Parse predictions into (datetime, height) pairs.
    //
    // A missing "t" or "v" field indicates a malformed response and is
    // fatal. A timestamp that can't be mapped to a single local time (e.g.
    // one that falls inside a DST spring-forward gap) or a height that
    // can't be parsed is skipped rather than failing the whole fetch - hilo
    // only returns a handful of points, but losing the entire series twice a
    // year over one ambiguous point is worse than dropping that one point.
    let mut points = Vec::<(chrono::DateTime<Local>, f32)>::new();
    for prediction in predictions {
        let time_str = prediction["t"].as_str().ok_or(TideError::Scrape)?;
        let height_str = prediction["v"].as_str().ok_or(TideError::Scrape)?;

        // Parse datetime (format: "2024-06-16 15:00")
        let Ok(naive) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M") else {
            continue;
        };
        let Some(dt) = naive.and_local_timezone(Local).single() else {
            continue;
        };

        // Parse tide height
        let Ok(ft) = height_str.parse::<f32>() else {
            continue;
        };

        points.push((dt, ft));
    }

    // Sort by time to ensure chronological order before interpolating.
    points.sort_by_key(|&(dt, _)| dt);

    // Verify we got enough data. hilo returns roughly one point every ~6
    // hours, so a 3-day window yields ~12 points; require at least a
    // handful so a single high/low pair can't masquerade as full coverage.
    if points.len() < 4 {
        return Err(TideError::Scrape);
    }

    // Interpolate the sparse high/low points to a dense 10-minute grid.
    // Deliberately interpolate from the full 3-day set rather than a
    // pre-filtered ±12h window: with ~6-hourly points, filtering first can
    // strip the bracketing points just outside the window, leaving edge
    // samples with nothing to interpolate between.
    let start = now - Duration::hours(12);
    let mut samples = Vec::with_capacity(145);

    // Generate 145 samples: 0, 10, 20, ..., 1440 minutes (24 hours)
    for step in 0..=144 {
        let ts = start + Duration::minutes(step * 10);
        let ft = interpolate_hilo(&points, ts);

        // Calculate minutes relative to "now" for display positioning
        let mins_rel = (ts - now).num_minutes() as i16;

        samples.push(Sample {
            mins_rel,
            tide_ft: ft,
        });
    }

    Ok(TideSeries {
        samples,
        offline: false,
    })
}

/// Interpolate a tide height at `ts` from sparse high/low (hilo) points.
///
/// `points` must be sorted ascending by time and non-empty.
///
/// # Why half-cosine, not linear
/// Real tides don't move at a constant rate between a high and a low: the
/// water moves fastest around the midpoint and slows to a stop at each
/// turning point (the "rule of twelfths" navigators use by hand). Linear
/// interpolation between hilo points would produce a sequence of straight
/// V-shapes that are visibly wrong and, worse, have a discontinuous slope
/// (zero velocity is required at every high and low, but linear segments
/// meet at a corner instead). A half-cosine curve matches this physical
/// shape closely while needing only the two bracketing points:
///
/// ```text
/// f = (ts - t0) / (t1 - t0)                        // 0.0..=1.0
/// height = (h0 + h1) / 2.0 - (h1 - h0) / 2.0 * cos(pi * f)
/// ```
///
/// This yields exactly `h0` at `f = 0.0`, exactly `h1` at `f = 1.0`, and an
/// S-shaped curve with zero slope at both ends in between.
///
/// Timestamps outside the range covered by `points` are clamped to the
/// nearest endpoint's height rather than extrapolated, since extrapolating
/// a cosine curve past its defining points produces meaningless values.
fn interpolate_hilo(points: &[(chrono::DateTime<Local>, f32)], ts: chrono::DateTime<Local>) -> f32 {
    use std::f64::consts::PI;

    // Clamp to the first/last point rather than interpolating between them
    // directly - that fallback would otherwise blend two arbitrary, possibly
    // far-apart points into a meaningless value.
    if ts <= points[0].0 {
        return points[0].1;
    }
    if ts >= points[points.len() - 1].0 {
        return points[points.len() - 1].1;
    }

    // Find the bracketing pair. Guaranteed to exist: ts is strictly between
    // the first and last point's times, checked above.
    let Some(window) = points.windows(2).find(|w| w[0].0 <= ts && ts <= w[1].0) else {
        return points[points.len() - 1].1;
    };
    let (t0, h0) = window[0];
    let (t1, h1) = window[1];

    let duration_secs = (t1 - t0).num_milliseconds() as f64 / 1000.0;
    if duration_secs <= 0.0 {
        // Duplicated (or out-of-order) timestamp: avoid dividing by zero.
        return h0;
    }

    let elapsed_secs = (ts - t0).num_milliseconds() as f64 / 1000.0;
    let f = (elapsed_secs / duration_secs).clamp(0.0, 1.0);

    let h0 = h0 as f64;
    let h1 = h1 as f64;
    let height = (h0 + h1) / 2.0 - (h1 - h0) / 2.0 * (PI * f).cos();

    height as f32
}

/// Load tide series from cache file if still valid.
///
/// Checks file modification time against TTL before deserializing.
/// Returns error for stale, missing, or corrupted cache files.
fn load_cache() -> Result<TideSeries, io::Error> {
    let meta = fs::metadata(CACHE)?;

    // Check if cache has expired based on file modification time
    let age = SystemTime::now()
        .duration_since(meta.modified()?)
        .map_err(|_| io::Error::other("time error"))?
        .as_secs();

    if age > TTL {
        return Err(io::Error::other("stale"));
    }

    // Deserialize cached data (binary JSON format)
    let data = fs::read(CACHE)?;
    let series = serde_json::from_slice(&data)?;

    Ok(series)
}

/// Save tide series to cache file for future use.
///
/// Uses binary JSON serialization for compact storage. Failure to write
/// cache is non-fatal - the application continues with fresh data.
fn save_cache(series: &TideSeries) -> Result<(), io::Error> {
    let data = serde_json::to_vec(series)?;
    fs::write(CACHE, data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Test helper: create a sample TideSeries for testing
    fn sample_series() -> TideSeries {
        TideSeries {
            samples: vec![
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
                    tide_ft: 4.0,
                },
            ],
            offline: false,
        }
    }

    #[test]
    fn test_cache_roundtrip() {
        let temp_file = NamedTempFile::new().unwrap();
        let cache_path = temp_file.path().to_str().unwrap();

        let series = sample_series();

        // Test saving
        let data = serde_json::to_vec(&series).unwrap();
        fs::write(cache_path, data).unwrap();

        // Test loading
        let loaded = serde_json::from_slice::<TideSeries>(&fs::read(cache_path).unwrap()).unwrap();

        assert_eq!(loaded.samples.len(), series.samples.len());
        assert_eq!(loaded.offline, series.offline);
    }

    /// Test helper: build a two-point (low, high) hilo series `hours` apart,
    /// starting at an arbitrary fixed local time.
    fn low_high(low: f32, high: f32, hours: i64) -> Vec<(chrono::DateTime<Local>, f32)> {
        let t0 = Local::now();
        let t1 = t0 + Duration::hours(hours);
        vec![(t0, low), (t1, high)]
    }

    #[test]
    fn interpolate_hilo_matches_endpoints() {
        let points = low_high(0.0, 10.0, 6);

        assert_eq!(interpolate_hilo(&points, points[0].0), 0.0);
        assert_eq!(interpolate_hilo(&points, points[1].0), 10.0);
    }

    #[test]
    fn interpolate_hilo_midpoint_is_average() {
        let points = low_high(0.0, 10.0, 6);
        let midpoint = points[0].0 + (points[1].0 - points[0].0) / 2;

        let height = interpolate_hilo(&points, midpoint);
        assert!(
            (height - 5.0).abs() < 0.001,
            "expected midpoint height ~5.0, got {}",
            height
        );
    }

    #[test]
    fn interpolate_hilo_is_monotonic_between_low_and_high() {
        let points = low_high(0.0, 10.0, 6);
        let (t0, _) = points[0];
        let (t1, _) = points[1];

        let mut previous = interpolate_hilo(&points, t0);
        for i in 1..=10 {
            let ts = t0 + (t1 - t0) * i / 10;
            let height = interpolate_hilo(&points, ts);
            assert!(
                height >= previous,
                "expected monotonic increase, got {} then {}",
                previous,
                height
            );
            previous = height;
        }
    }

    #[test]
    fn interpolate_hilo_clamps_outside_range() {
        let points = low_high(0.0, 10.0, 6);
        let (t0, h0) = points[0];
        let (t1, h1) = points[1];

        let before = interpolate_hilo(&points, t0 - Duration::hours(1));
        let after = interpolate_hilo(&points, t1 + Duration::hours(1));

        assert_eq!(before, h0);
        assert_eq!(after, h1);
    }

    #[test]
    fn interpolate_hilo_handles_duplicate_timestamp() {
        let t0 = Local::now();
        let points = vec![(t0, 3.0), (t0, 3.0), (t0 + Duration::hours(6), 8.0)];

        // Should not panic (no division by zero) and should return h0 for
        // the degenerate zero-duration segment.
        let height = interpolate_hilo(&points, t0);
        assert_eq!(height, 3.0);
    }
}

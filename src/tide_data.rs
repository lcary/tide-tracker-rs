//! # NOAA Tide Data Fetching and Caching
//!
//! This module handles all network operations for fetching real-time tide predictions
//! from NOAA's Tides and Currents API. It includes a durable, station-aware cache so
//! that a multi-day network outage still shows *correct* tide predictions instead of
//! falling back to a synthetic, station-unaware curve.
//!
//! ## Data Source
//!
//! ### NOAA CO-OPS API
//! - **URL**: https://api.tidesandcurrents.noaa.gov/api/prod/datagetter
//! - **Station**: configurable via `tide-config.toml` (`config.station.id`)
//! - **Format**: JSON response with high/low (`interval=hilo`) tide predictions.
//!   This works for both NOAA *reference* stations (type R) and *subordinate*
//!   stations (type S). The default 6-minute harmonic interval only works for
//!   reference stations and returns an API error for subordinate stations, so
//!   `hilo` is used unconditionally regardless of station type.
//! - **Data**: yesterday through 7 days from now (~32 hilo turning points, a few KB)
//!
//! ### Data Processing Pipeline
//! 1. **Fetch**: HTTP GET request to NOAA CO-OPS API requesting `interval=hilo`
//!    ([`fetch_points`])
//! 2. **Parse**: Deserialize JSON response containing high/low tide predictions,
//!    skipping any individual point with an unmappable timestamp (e.g. inside a
//!    DST spring-forward gap) or an unparsable height
//! 3. **Sort**: Order all parsed points chronologically across the full ~8-day set
//! 4. **Cache**: Store the raw, sorted points with absolute (unix-time) timestamps,
//!    keyed by station ID, so they can be reused for any later "now" within the
//!    window - see "Caching Strategy" below
//! 5. **Interpolate**: Convert the sparse high/low points to a dense 10-minute
//!    grid using half-cosine (rule-of-twelfths) interpolation between turning
//!    points, clamped to the endpoint heights outside the data's range
//!    ([`series_from_points`])
//! 6. **Return**: 145 samples ready for visualization
//!
//! ## Caching Strategy
//!
//! ### Why the old cache never worked
//! The previous design cached the *derived* 10-minute grid (whose `mins_rel` values
//! are relative to fetch time, and therefore stale the instant "now" moves on) in
//! `/tmp/tide_cache.json` with a 30-minute TTL. But the systemd unit runs the binary
//! as a `Type=oneshot` service with `PrivateTmp=true`, so every single invocation gets
//! its own private, empty `/tmp` - the cache file from the previous run is simply gone
//! before the TTL could ever matter. The cache also wasn't keyed by station, so
//! switching stations would have silently kept serving the old station's data. Net
//! effect: the cache never hit, every run re-fetched, and a network blip fell straight
//! through to a synthetic sine curve with no real relationship to the actual station.
//!
//! ### Why this is fixable: tide predictions don't go stale
//! Unlike weather, tide predictions are deterministic astronomy computed years in
//! advance from harmonic constituents. A prediction fetched today for six days from
//! now is exactly as correct as one fetched six days from now for the same moment.
//! That means a cache doesn't need a short TTL for *correctness* - it only needs to
//! (a) cover the time window currently being displayed and (b) match the currently
//! configured station.
//!
//! ### This implementation
//! - **Location**: see [`cache_path`] - `$TIDE_CACHE_DIR`, then `$STATE_DIRECTORY`
//!   (systemd's `StateDirectory=` mechanism), then `/var/lib/tide-tracker`
//!   as a last resort. All of these persist across `PrivateTmp` oneshot invocations,
//!   which is exactly what the old `/tmp` cache did not.
//! - **Format**: raw hilo points with absolute unix-second timestamps (not the derived
//!   grid), plus the station ID they belong to and when they were fetched - see
//!   [`CachedPredictions`]. Storing absolute time means the same cache file remains
//!   usable for "now" hours or days after it was written.
//! - **Station-aware**: a cache is only usable if `station_id` matches the configured
//!   station, so switching stations correctly forces a fresh fetch instead of quietly
//!   reusing the old station's predictions.
//! - **Window coverage over TTL**: a cache is usable if its points bracket the ±12h
//!   display window (see [`cache_is_usable`]), not based on a fixed expiry. It is
//!   still refreshed opportunistically every [`REFRESH_AFTER_HOURS`] when the network
//!   is available, but that's just housekeeping, not a correctness requirement.
//! - **Outage tolerance**: requesting predictions through 7 days out means a cache
//!   saved right before an outage begins still brackets "now" for roughly 6 more days
//!   (7 days of lookahead minus the 12h the window needs to extend past "now").
//!
//! ### Cache Benefits
//! - **Correctness during outages**: a multi-day network outage still displays real,
//!   station-matched predictions instead of a synthetic curve
//! - **Reduced bandwidth**: avoid repeated downloads when the cache already covers
//!   the window
//! - **Pi Zero friendly**: minimizes CPU time and radio usage spent on network I/O
//!
//! ## Error Handling
//!
//! The module handles multiple failure modes gracefully:
//! - **Network timeouts**: HTTP client configured with reasonable timeouts
//! - **Server errors**: 5xx responses handled as fetch failures
//! - **Parse failures**: Malformed JSON or unexpected API response structure
//! - **Cache corruption**: Invalid JSON falls back to a fresh network fetch attempt
//! - **File system issues**: Permissions or disk space problems fall through to the
//!   next candidate cache directory (see [`cache_path`])
//!
//! All errors propagate through `TideError` enum for consistent handling. Only when
//! *both* a live fetch fails *and* no usable cache exists does the error propagate to
//! the caller, which then falls back to [`crate::fallback::approximate`] - the
//! synthetic, station-unaware model of last resort.

use crate::{config::Config, Sample, TideSeries};
use chrono::{DateTime, Duration, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::{env, fs, io, path::PathBuf};
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

/// On-disk cache format: raw hilo predictions with absolute timestamps.
///
/// Deliberately stores `(unix seconds, feet)` pairs rather than serialized
/// `DateTime<Local>` values. Serializing timezone-aware datetimes directly is
/// a well-known pitfall (the local UTC offset can differ between when the
/// cache was written and when it's read back, e.g. across a DST transition),
/// so we store the unambiguous unix timestamp and reconstruct `DateTime<Local>`
/// on load with [`Local.timestamp_opt`](chrono::TimeZone::timestamp_opt).
#[derive(Serialize, Deserialize)]
struct CachedPredictions {
    /// Station these predictions belong to. A cache is only usable if this
    /// matches the currently configured station - see [`cache_is_usable`].
    station_id: String,
    /// When these predictions were fetched from NOAA (unix seconds).
    fetched_at: i64,
    /// Raw, chronologically sorted hilo points as (unix seconds, feet).
    points: Vec<(i64, f32)>,
}

/// Cache filename within whichever directory [`cache_path`] resolves to.
const CACHE_FILENAME: &str = "predictions.json";

/// Opportunistic refresh interval, in hours.
///
/// This is *not* a correctness TTL - a cache that still covers the display
/// window is astronomically correct no matter how old it is (see the module
/// docs). It just bounds how long we go without checking NOAA again while
/// online, so the cached window keeps sliding forward and stays well clear of
/// its edges, and so any actual NOAA data correction is picked up reasonably
/// promptly.
const REFRESH_AFTER_HOURS: i64 = 6;

/// Resolve the writable directory the prediction cache should live in.
///
/// Tries each candidate in priority order and returns the first one that can
/// be created (which, on the systems this runs on, implies it's writable):
/// 1. `$TIDE_CACHE_DIR` - overrides everything else; used by tests and for
///    manual runs where writing to `/var/lib` isn't desired or possible.
/// 2. `$STATE_DIRECTORY` - exported by systemd when the unit sets
///    `StateDirectory=tide-tracker` (see `scripts/setup-tide-tracker.sh`).
///    systemd creates this directory and makes it persist across
///    `PrivateTmp=true` oneshot invocations, which is exactly what the old
///    `/tmp`-based cache lacked.
/// 3. `/var/lib/tide-tracker` - same path systemd would use, so behavior is
///    consistent even when run manually outside the unit.
fn cache_dir_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::with_capacity(3);
    if let Ok(dir) = env::var("TIDE_CACHE_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    if let Ok(dir) = env::var("STATE_DIRECTORY") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(PathBuf::from("/var/lib/tide-tracker"));
    candidates
}

/// Resolve the full cache file path, falling through candidate directories
/// (see [`cache_dir_candidates`]) until one can be created. Returns `None`
/// only if no candidate directory is writable.
fn cache_path() -> Option<PathBuf> {
    cache_dir_candidates()
        .into_iter()
        .find(|dir| fs::create_dir_all(dir).is_ok())
        .map(|dir| dir.join(CACHE_FILENAME))
}

/// Fetch current tide series from NOAA or a durable, station-aware cache.
///
/// This is the main entry point for obtaining tide data. Policy, in order:
///
/// 1. Load the cache, if any. It's only *usable* if its station matches the
///    configured station and its points bracket the ±12h display window
///    (see [`cache_is_usable`]) - bracketing matters because
///    [`interpolate_hilo`] clamps outside its data range, which would draw a
///    flat line at one or both ends of the chart instead of a real curve.
/// 2. If the cache is usable *and* younger than [`REFRESH_AFTER_HOURS`],
///    return a series built from it directly - no network request at all.
/// 3. Otherwise, attempt a live fetch:
///    - On success, save the new points to the cache and return a fresh
///      series with `stale_hours: None`.
///    - On failure, if the (old) cache is still usable, return a series
///      built from it with `stale_hours: Some(age)` - this is the outage
///      path that keeps the display *correct* instead of falling back to a
///      synthetic curve. If the cache isn't usable either, propagate the
///      error so the caller can fall back to [`crate::fallback::approximate`].
///
/// Every branch logs loudly (to stderr, visible in the systemd journal) so
/// it's obvious after the fact whether a given run hit the network, served
/// from cache, or degraded.
///
/// # Memory Usage
/// - Cache check: a few KB for the cached points (vs. ~50 bytes/point × ~32
///   points for the wider 8-day window)
/// - Network fetch: similarly small - `hilo` returns only turning points
/// - Parsed data: ~900 bytes for the final `TideSeries`
///
/// # Error Handling
/// On any error, the caller should fall back to `fallback::approximate()`
/// to ensure the application continues working even with network issues.
///
/// # Returns
/// - `Ok(TideSeries)`: Successfully loaded data (fresh, or a usable cache)
/// - `Err(TideError)`: Live fetch failed and no usable cache exists
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
    let now = Local::now();

    // Load whatever is on disk (if anything) and check whether it's usable
    // for *this* station and *this* display window. `usable` borrows from
    // `cached`, but `Option<&T>` is `Copy`, so it can be matched on below
    // without giving up the ability to use it again in the failure branch.
    let cached = load_cache();
    let usable = cached
        .as_ref()
        .filter(|c| cache_is_usable(c, &config.station.id, now));

    if let Some(cache) = usable {
        let age_hours = cache_age_hours(cache, now);
        if age_hours < REFRESH_AFTER_HOURS as f32 {
            eprintln!(
                "Tide cache hit for station {} (age {:.1}h, refresh threshold {}h) - \
                 serving cached predictions with no network request",
                config.station.id, age_hours, REFRESH_AFTER_HOURS
            );
            return Ok(series_from_points(&decode_points(&cache.points), now, None));
        }
        eprintln!(
            "Tide cache for station {} still covers the display window but is {:.1}h old \
             (>= {}h refresh threshold) - refreshing from NOAA",
            config.station.id, age_hours, REFRESH_AFTER_HOURS
        );
    } else {
        eprintln!(
            "No usable tide cache for station {} - fetching from NOAA",
            config.station.id
        );
    }

    match fetch_points(&config).await {
        Ok(points) => {
            eprintln!(
                "Fetched {} fresh tide prediction points for station {} from NOAA",
                points.len(),
                config.station.id
            );
            if let Err(e) = save_cache(&config.station.id, now, &points) {
                // Non-fatal: we still have fresh data to display this run,
                // we just won't be able to survive the *next* outage.
                eprintln!("Warning: failed to write tide prediction cache: {}", e);
            }
            Ok(series_from_points(&points, now, None))
        }
        Err(err) => {
            if let Some(cache) = usable {
                let age_hours = cache_age_hours(cache, now);
                eprintln!(
                    "⚠️  Live NOAA fetch failed ({}), but the cached predictions for station {} \
                     (age {:.1}h) still cover the display window - serving cached-but-correct \
                     data instead of a synthetic curve",
                    err, config.station.id, age_hours
                );
                Ok(series_from_points(
                    &decode_points(&cache.points),
                    now,
                    Some(age_hours),
                ))
            } else {
                eprintln!(
                    "❌ Live NOAA fetch failed ({}) and no usable cache exists for station {} - \
                     propagating error for synthetic fallback",
                    err, config.station.id
                );
                Err(err)
            }
        }
    }
}

// -- Private Implementation --

/// Whether `cache` can stand in for a live fetch, right now, for the given
/// `station_id`.
///
/// Two conditions must both hold:
/// - **Station match**: a cache for a different station would silently
///   serve the wrong station's tides, which is exactly the bug this cache
///   redesign is meant to close.
/// - **Window coverage**: the cache's earliest point must be at or before
///   `now - 12h` and its latest point at or after `now + 12h`. This matters
///   because [`interpolate_hilo`] clamps to the nearest endpoint's height
///   outside the range its points cover, rather than extrapolating - a
///   cache that doesn't reach both edges of the ±12h display window would
///   render a flat line at one or both ends instead of a real curve.
///
/// Assumes `cache.points` is sorted chronologically ascending, which holds
/// for every cache this module writes (see [`fetch_points`]).
fn cache_is_usable(cache: &CachedPredictions, station_id: &str, now: DateTime<Local>) -> bool {
    if cache.station_id != station_id {
        return false;
    }

    let (Some(&(earliest, _)), Some(&(latest, _))) = (cache.points.first(), cache.points.last())
    else {
        return false;
    };

    let now_secs = now.timestamp();
    let half_window_secs = Duration::hours(12).num_seconds();

    earliest <= now_secs - half_window_secs && latest >= now_secs + half_window_secs
}

/// Age of `cache` in hours, relative to `now`.
fn cache_age_hours(cache: &CachedPredictions, now: DateTime<Local>) -> f32 {
    (now.timestamp() - cache.fetched_at) as f32 / 3600.0
}

/// Convert cached `(unix seconds, feet)` pairs back into `(DateTime<Local>,
/// feet)` pairs, skipping any timestamp that fails to convert.
///
/// A unix timestamp can fail to map to a single local time in the same way
/// a parsed NOAA timestamp can (e.g. a DST spring-forward gap) - see the
/// equivalent skip in [`fetch_points`]. That should be exceedingly rare for
/// timestamps we generated ourselves, but skipping rather than panicking
/// keeps a single bad point from taking down the whole cached series.
fn decode_points(points: &[(i64, f32)]) -> Vec<(DateTime<Local>, f32)> {
    points
        .iter()
        .filter_map(|&(secs, ft)| Local.timestamp_opt(secs, 0).single().map(|dt| (dt, ft)))
        .collect()
}

/// Convert `(DateTime<Local>, feet)` pairs into the cache's absolute-time
/// `(unix seconds, feet)` representation.
fn encode_points(points: &[(DateTime<Local>, f32)]) -> Vec<(i64, f32)> {
    points
        .iter()
        .map(|&(dt, ft)| (dt.timestamp(), ft))
        .collect()
}

/// Load the cached predictions from disk, if a cache file exists at any
/// candidate location and parses successfully. Missing or corrupt caches
/// simply return `None`, which sends [`fetch`] straight to a live fetch.
fn load_cache() -> Option<CachedPredictions> {
    let path = cache_path()?;
    let data = fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}

/// Save freshly fetched points to the cache, keyed by station and fetch
/// time. Failure is reported to the caller (who logs and continues) rather
/// than propagated as a hard error - losing the ability to survive the
/// *next* outage shouldn't fail the *current* successful fetch.
fn save_cache(
    station_id: &str,
    fetched_at: DateTime<Local>,
    points: &[(DateTime<Local>, f32)],
) -> Result<(), io::Error> {
    let path = cache_path().ok_or_else(|| io::Error::other("no writable cache directory found"))?;
    let cache = CachedPredictions {
        station_id: station_id.to_string(),
        fetched_at: fetched_at.timestamp(),
        points: encode_points(points),
    };
    let data = serde_json::to_vec(&cache)?;
    fs::write(path, data)?;
    Ok(())
}

/// Fetch raw hilo tide points from NOAA for `config`'s station.
///
/// This is deliberately decoupled from grid-building ([`series_from_points`]):
/// the raw points, tagged with absolute timestamps, are what gets cached and
/// can be reused for any later "now" within the window, whereas the derived
/// 10-minute grid is only ever valid for the instant it was generated for.
///
/// 1. Downloads JSON data from NOAA's CO-OPS API, requesting `interval=hilo`
/// 2. Parses the structured high/low tide predictions, skipping any point
///    whose timestamp can't be mapped to a local time or whose height can't
///    be parsed (a malformed response, i.e. missing fields, is still fatal)
/// 3. Sorts the points chronologically across the full window
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
/// # Why yesterday through +7 days
/// The display only needs ±12h, but requesting a much wider window means the
/// *cache* built from these points keeps bracketing "now" for days after it
/// was fetched (see the module docs) - a cache saved right before an outage
/// begins tolerates roughly 6 days of outage before it stops covering the
/// window. `hilo` only returns turning points (~4/day), so widening the
/// window to 8 days is still just ~32 points / a few KB, not a meaningfully
/// larger request.
///
/// # API Configuration
/// Uses NOAA CO-OPS API v1 with the following parameters:
/// - Station: from `config.station.id`
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
/// product=predictions&station=8418150&begin_date=20240615&end_date=20240624&
/// datum=MLLW&time_zone=lst_ldt&units=english&format=json&interval=hilo
/// ```
async fn fetch_points(config: &Config) -> Result<Vec<(DateTime<Local>, f32)>, TideError> {
    // Calculate date range: yesterday through 7 days from now. See the
    // "Why yesterday through +7 days" doc section above for the rationale.
    let now = Local::now();
    let begin_date_time = now - Duration::days(1);
    let end_date_time = now + Duration::days(7);

    // Format dates for API (YYYYMMDD)
    let begin_date = begin_date_time.format("%Y%m%d").to_string();
    let end_date = end_date_time.format("%Y%m%d").to_string();

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
    let mut points = Vec::<(DateTime<Local>, f32)>::new();
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

    // Sort by time to ensure chronological order. Callers (cache_is_usable,
    // series_from_points, and the on-disk cache itself) all rely on this.
    points.sort_by_key(|&(dt, _)| dt);

    // Verify we got enough data. hilo returns roughly one point every ~6
    // hours, so the ~8-day window yields ~32 points; require at least a
    // handful so a single high/low pair can't masquerade as full coverage.
    if points.len() < 4 {
        return Err(TideError::Scrape);
    }

    Ok(points)
}

/// Build a dense 145-sample, 10-minute grid centered on `now` from sparse
/// hilo `points`, using the same half-cosine interpolation ([`interpolate_hilo`])
/// as a live fetch.
///
/// `points` may come from a fresh network fetch or from a decoded cache -
/// the grid-building logic doesn't care which, which is exactly the point:
/// cached predictions produce an identical-quality series to a live fetch,
/// because tide predictions don't go stale (see the module docs).
///
/// `stale_hours` is threaded straight through to the returned
/// [`TideSeries::stale_hours`] - pass `None` for a live fetch, or
/// `Some(age)` when serving a cache after a live fetch failed.
fn series_from_points(
    points: &[(DateTime<Local>, f32)],
    now: DateTime<Local>,
    stale_hours: Option<f32>,
) -> TideSeries {
    // Deliberately interpolate from the full point set rather than a
    // pre-filtered ±12h window: with ~6-hourly points, filtering first can
    // strip the bracketing points just outside the window, leaving edge
    // samples with nothing to interpolate between.
    let start = now - Duration::hours(12);
    let mut samples = Vec::with_capacity(145);

    // Generate 145 samples: -720, -710, ..., 0, ..., 710, 720 minutes
    for step in 0..=144 {
        let ts = start + Duration::minutes(step * 10);
        let ft = interpolate_hilo(points, ts);

        // Calculate minutes relative to "now" for display positioning
        let mins_rel = (ts - now).num_minutes() as i16;

        samples.push(Sample {
            mins_rel,
            tide_ft: ft,
        });
    }

    TideSeries {
        samples,
        offline: false,
        stale_hours,
    }
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
fn interpolate_hilo(points: &[(DateTime<Local>, f32)], ts: DateTime<Local>) -> f32 {
    use std::f64::consts::PI;

    // Index of the first point at or after `ts`. Clamp outside the data range
    // rather than extrapolating: beyond the ends we have no information, and
    // blending the first and last point would invent a meaningless curve.
    let i = points.partition_point(|&(t, _)| t < ts);
    if i == 0 {
        return points[0].1;
    }
    if i == points.len() {
        return points[points.len() - 1].1;
    }

    let (t0, h0) = points[i - 1];
    let (t1, h1) = points[i];

    let span = (t1 - t0).num_seconds() as f64;
    if span <= 0.0 {
        // Duplicated (or out-of-order) timestamp: avoid dividing by zero.
        return h0;
    }
    let f = ((ts - t0).num_seconds() as f64 / span).clamp(0.0, 1.0);

    let (h0, h1) = (h0 as f64, h1 as f64);
    ((h0 + h1) / 2.0 - (h1 - h0) / 2.0 * (PI * f).cos()) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

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
            stale_hours: None,
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
        assert_eq!(loaded.stale_hours, series.stale_hours);
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

    /// Points that bracket the full ±13h window around `now` (comfortably
    /// past the ±12h the display needs), for a "usable cache" fixture.
    fn bracketing_points(now: DateTime<Local>) -> Vec<(DateTime<Local>, f32)> {
        vec![
            (now - Duration::hours(13), 1.0),
            (now + Duration::hours(13), 9.0),
        ]
    }

    #[test]
    fn durable_cache_roundtrip_preserves_points() {
        // Point TIDE_CACHE_DIR at a scratch directory so this test never
        // touches the real cache location (/var/lib/tide-tracker).
        let temp_dir = TempDir::new().unwrap();
        env::set_var("TIDE_CACHE_DIR", temp_dir.path());

        let now = Local::now();
        let points = bracketing_points(now);

        save_cache("8418150", now, &points).expect("save_cache should succeed");

        let loaded = load_cache().expect("cache should load back");
        assert_eq!(loaded.station_id, "8418150");
        assert_eq!(loaded.fetched_at, now.timestamp());

        let decoded = decode_points(&loaded.points);
        assert_eq!(decoded.len(), points.len());
        for ((expected_dt, expected_ft), (actual_dt, actual_ft)) in
            points.iter().zip(decoded.iter())
        {
            assert_eq!(expected_dt.timestamp(), actual_dt.timestamp());
            assert_eq!(expected_ft, actual_ft);
        }

        env::remove_var("TIDE_CACHE_DIR");
    }

    #[test]
    fn cache_is_usable_rejects_station_mismatch() {
        let now = Local::now();
        let cache = CachedPredictions {
            station_id: "OTHER_STATION".to_string(),
            fetched_at: now.timestamp(),
            points: encode_points(&bracketing_points(now)),
        };

        assert!(!cache_is_usable(&cache, "8418150", now));
    }

    #[test]
    fn cache_is_usable_rejects_non_bracketing_window() {
        let now = Local::now();
        // Only covers ±6h, well short of the ±12h the display window needs.
        let points = vec![
            (now - Duration::hours(6), 1.0),
            (now + Duration::hours(6), 9.0),
        ];
        let cache = CachedPredictions {
            station_id: "8418150".to_string(),
            fetched_at: now.timestamp(),
            points: encode_points(&points),
        };

        assert!(!cache_is_usable(&cache, "8418150", now));
    }

    #[test]
    fn cache_is_usable_accepts_bracketing_window() {
        let now = Local::now();
        let cache = CachedPredictions {
            station_id: "8418150".to_string(),
            fetched_at: now.timestamp(),
            points: encode_points(&bracketing_points(now)),
        };

        assert!(cache_is_usable(&cache, "8418150", now));
    }

    #[test]
    fn series_from_points_has_expected_grid_and_stale_hours() {
        let now = Local::now();
        let points = bracketing_points(now);

        let series = series_from_points(&points, now, Some(5.5));

        assert_eq!(series.samples.len(), 145);
        for (i, sample) in series.samples.iter().enumerate() {
            let expected_mins_rel = -720 + (i as i16) * 10;
            assert_eq!(sample.mins_rel, expected_mins_rel);
        }
        assert_eq!(series.samples.first().unwrap().mins_rel, -720);
        assert_eq!(series.samples.last().unwrap().mins_rel, 720);

        assert_eq!(series.stale_hours, Some(5.5));
        assert!(!series.offline);
    }
}

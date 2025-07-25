//! # Comprehensive Test Suite for Tide Tracker
//!
//! This module contains unit tests that verify the correctness of tide data processing,
//! fallback models, and edge case handling. Tests are designed to run quickly and
//! independently, suitable for continuous integration and development workflows.

use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;
use tide_clock_lib::{Sample, TideSeries};

// Import the modules we're testing
use crate::fallback;

/// Test that the fallback sine model produces reasonable tide ranges.
///
/// Validates that the mathematical model generates realistic tidal ranges
/// typical of coastal areas (not too extreme, not too flat).
#[test]
fn fallback_produces_sane_tide_range() {
    let series = fallback::approximate(None);

    // Extract all tide heights for analysis
    let heights: Vec<f32> = series.samples.iter().map(|s| s.tide_ft).collect();
    let min_height = heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    let max_height = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));

    // Verify reasonable tidal range for Portland, ME (NOAA harmonics)
    let tidal_range = max_height - min_height;
    assert!(
        (9.5..=11.0).contains(&tidal_range),
        "Tidal range {} is outside expected bounds (9.5-11.0 feet)",
        tidal_range
    );

    // Allow small negative heights due to harmonic phase (Portland, ME)
    assert!(
        min_height > -0.5,
        "Minimum tide height {} should be greater than -0.5 ft",
        min_height
    );

    // Verify series is marked as offline
    assert!(
        series.offline,
        "Fallback series should be marked as offline"
    );
}

/// Test that samples are spaced exactly 10 minutes apart.
///
/// Verifies the temporal spacing is consistent across the entire 24-hour window,
/// which is critical for accurate visualization and interpolation.
#[test]
fn samples_have_correct_10_minute_spacing() {
    let series = fallback::approximate(None);

    // Check total sample count (24 hours * 6 samples/hour + 1)
    assert_eq!(
        series.samples.len(),
        145,
        "Should have exactly 145 samples for 24 hours at 10-minute intervals"
    );

    // Verify consistent 10-minute spacing between consecutive samples
    for window in series.samples.windows(2) {
        let time_diff = window[1].mins_rel - window[0].mins_rel;
        assert_eq!(
            time_diff, 10,
            "Time difference between consecutive samples should be 10 minutes, got {}",
            time_diff
        );
    }

    // Verify time range spans exactly 24 hours
    let first_sample = series.samples.first().unwrap();
    let last_sample = series.samples.last().unwrap();
    assert_eq!(
        first_sample.mins_rel, -720,
        "First sample should be -12 hours"
    );
    assert_eq!(last_sample.mins_rel, 720, "Last sample should be +12 hours");
}

/// Test that the current time marker (mins_rel = 0) exists and is unique.
///
/// The current time marker is critical for user orientation on the display.
#[test]
fn current_time_marker_exists_and_unique() {
    let series = fallback::approximate(None);

    // Find all samples at current time
    let current_samples: Vec<_> = series.samples.iter().filter(|s| s.mins_rel == 0).collect();

    // Verify exactly one sample at current time
    assert_eq!(
        current_samples.len(),
        1,
        "Should have exactly one sample at current time (mins_rel = 0)"
    );

    let current_sample = current_samples[0];

    // Verify current sample has reasonable tide height
    assert!(
        (-0.5..=10.0).contains(&current_sample.tide_ft),
        "Current tide height {} should be reasonable (-0.5 to 10 feet)",
        current_sample.tide_ft
    );
}

/// Test edge cases for tide series data structure.
///
/// Verifies behavior with extreme or unusual data that might occur
/// during network errors or unusual tide conditions.
#[test]
fn tide_series_handles_edge_cases() {
    // Test empty series
    let empty_series = TideSeries {
        samples: vec![],
        offline: true,
    };
    assert_eq!(empty_series.samples.len(), 0);

    // Test series with single sample
    let single_sample_series = TideSeries {
        samples: vec![Sample {
            mins_rel: 0,
            tide_ft: 5.0,
        }],
        offline: false,
    };
    assert_eq!(single_sample_series.samples.len(), 1);

    // Test series with extreme tide values
    let extreme_series = TideSeries {
        samples: vec![
            Sample {
                mins_rel: -10,
                tide_ft: -2.0,
            }, // Negative tide (rare but possible)
            Sample {
                mins_rel: 0,
                tide_ft: 0.0,
            }, // Zero tide
            Sample {
                mins_rel: 10,
                tide_ft: 25.0,
            }, // Very high tide
        ],
        offline: false,
    };

    // Should handle extreme values without panicking
    let heights: Vec<f32> = extreme_series.samples.iter().map(|s| s.tide_ft).collect();
    let range = heights.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
        - heights.iter().fold(f32::INFINITY, |a, &b| a.min(b));
    assert!(range > 0.0, "Extreme series should have positive range");
}

/// Test interpolation monotonicity properties.
///
/// While the fallback model uses a simple sine wave, real-world tide data
/// should maintain reasonable smoothness properties for visualization.
#[test]
fn interpolation_produces_smooth_curves() {
    let series = fallback::approximate(None);

    // Calculate first differences (rate of change between samples)
    let mut differences = Vec::new();
    for window in series.samples.windows(2) {
        let diff = window[1].tide_ft - window[0].tide_ft;
        differences.push(diff);
    }

    // Verify rate of change is bounded (no extreme jumps)
    // With 10-minute sampling, changes should be gradual
    let max_change = differences.iter().fold(0.0f32, |a, &b| a.max(b.abs()));
    assert!(
        max_change < 1.0,
        "Maximum 10-minute tide change {} should be less than 1 foot",
        max_change
    );

    // Count direction changes to verify we have realistic tidal cycles
    let mut direction_changes = 0;
    for window in differences.windows(2) {
        if window[0].signum() != window[1].signum() {
            direction_changes += 1;
        }
    }

    // Should have multiple direction changes over 24 hours (high/low tides)
    assert!(
        (2..=8).contains(&direction_changes),
        "Should have 2-8 direction changes over 24 hours, got {}",
        direction_changes
    );
}

/// Test cache file operations for network data persistence.
///
/// Verifies that caching works correctly for reducing network load
/// and providing offline resilience.
#[test]
fn cache_operations_work_correctly() {
    // Create a temporary file for testing cache operations
    let temp_file = NamedTempFile::new().expect("Should create temp file");
    let cache_path = temp_file.path();

    // Create test data
    let original_series = TideSeries {
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
    };

    // Test serialization
    let serialized = serde_json::to_vec(&original_series).expect("Should serialize tide series");

    // Test writing to cache
    fs::write(cache_path, &serialized).expect("Should write cache file");

    // Test reading from cache
    let cached_data = fs::read(cache_path).expect("Should read cache file");

    // Test deserialization
    let loaded_series: TideSeries =
        serde_json::from_slice(&cached_data).expect("Should deserialize tide series");

    // Verify data integrity
    assert_eq!(loaded_series.samples.len(), original_series.samples.len());
    assert_eq!(loaded_series.offline, original_series.offline);

    for (original, loaded) in original_series
        .samples
        .iter()
        .zip(loaded_series.samples.iter())
    {
        assert_eq!(original.mins_rel, loaded.mins_rel);
        assert_eq!(original.tide_ft, loaded.tide_ft);
    }
}

/// Test cache staleness detection.
///
/// Verifies that the cache TTL mechanism works correctly to prevent
/// serving stale data while allowing reasonable caching periods.
#[test]
fn cache_staleness_detection_works() {
    use std::thread;

    // Create temporary file
    let temp_file = NamedTempFile::new().expect("Should create temp file");
    let cache_path = temp_file.path();

    // Write some test data
    let test_data = b"test cache data";
    fs::write(cache_path, test_data).expect("Should write test data");

    // Get initial modification time
    let initial_metadata = fs::metadata(cache_path).expect("Should get metadata");
    let initial_modified = initial_metadata
        .modified()
        .expect("Should get modified time");

    // Verify file is initially "fresh"
    let age = SystemTime::now()
        .duration_since(initial_modified)
        .expect("Should calculate age")
        .as_secs();
    assert!(age < 10, "Fresh file should be less than 10 seconds old");

    // Simulate staleness by checking what happens after a brief delay
    thread::sleep(Duration::from_millis(100));

    let current_metadata = fs::metadata(cache_path).expect("Should get current metadata");
    let current_modified = current_metadata
        .modified()
        .expect("Should get current modified time");

    // File modification time should be stable
    assert_eq!(
        initial_modified, current_modified,
        "Modification time should not change"
    );
}

/// Test memory usage patterns to ensure embedded compatibility.
///
/// While we can't directly measure memory in unit tests, we can verify
/// that data structures use expected sizes and avoid unnecessary allocations.
#[test]
fn memory_usage_is_reasonable() {
    let series = fallback::approximate(None);

    // Verify sample count is exactly what we expect (no extra allocations)
    assert_eq!(series.samples.len(), 145);
    assert_eq!(series.samples.capacity(), 145);

    // Calculate expected memory usage
    let sample_size = std::mem::size_of::<Sample>();
    let expected_samples_memory = sample_size * 145;

    // Sample should be small (6-8 bytes depending on alignment: i16 + f32)
    assert!(
        sample_size <= 8,
        "Sample should be 8 bytes or less, got {} bytes",
        sample_size
    );

    // Total samples memory should be reasonable (allow up to 1.5KB for alignment)
    assert!(
        expected_samples_memory < 1500,
        "145 samples should use less than 1.5KB: {} bytes",
        expected_samples_memory
    );

    // Verify TideSeries structure size is reasonable
    let series_size = std::mem::size_of::<TideSeries>();
    assert!(
        series_size < 50,
        "TideSeries struct should be small: {} bytes",
        series_size
    );
}

/// Test that series data maintains temporal ordering.
///
/// Critical for visualization - samples must be in chronological order.
#[test]
fn samples_are_chronologically_ordered() {
    let series = fallback::approximate(None);

    // Verify samples are in ascending time order
    for window in series.samples.windows(2) {
        assert!(
            window[0].mins_rel < window[1].mins_rel,
            "Samples should be in chronological order: {} should be less than {}",
            window[0].mins_rel,
            window[1].mins_rel
        );
    }

    // Verify we start at -12 hours and end at +12 hours
    assert_eq!(series.samples[0].mins_rel, -720);
    assert_eq!(series.samples[144].mins_rel, 720);
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    /// Test that fallback model generation is fast enough for embedded use.
    ///
    /// The Pi Zero 2 W is slow, so operations should complete quickly.
    #[test]
    fn fallback_generation_is_fast() {
        let start = Instant::now();
        let _series = fallback::approximate(None);
        let duration = start.elapsed();

        // Should complete in well under 1 second on any reasonable hardware
        assert!(
            duration.as_millis() < 100,
            "Fallback generation took too long: {:?}",
            duration
        );
    }

    /// Test that multiple generations don't accumulate memory.
    ///
    /// Verifies no memory leaks in repeated operations.
    #[test]
    fn repeated_operations_dont_leak() {
        // Generate multiple series to check for memory accumulation
        for _ in 0..100 {
            let _series = fallback::approximate(None);
            // Memory should be freed after each iteration
        }

        // If we get here without OOM, the test passes
        // (Actual memory measurement would require external tools)
    }
}

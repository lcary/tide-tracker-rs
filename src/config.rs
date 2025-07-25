/// # Configuration Management
///
/// This module handles loading and parsing configuration from the tide-config.toml file.
/// It provides a centralized way to configure NOAA station settings, display options,
/// and other runtime parameters.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Application configuration loaded from tide-config.toml
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// NOAA station configuration
    pub station: StationConfig,
    /// Display and UI configuration  
    pub display: DisplayConfig,
}

/// NOAA tide station configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct StationConfig {
    /// NOAA station ID (e.g., "8418150" for Portland, ME)
    pub id: String,
    /// Human-readable station name for reference
    pub name: String,
    /// MLLW to Mean Sea Level offset in feet for user-friendly display
    pub msl_offset: f32,
    /// Whether to show heights relative to MSL (true) or MLLW (false)
    /// Default false shows traditional MLLW tide chart values (0-9+ feet)
    /// Set true to show heights relative to mean sea level (-5 to +5 feet)
    pub show_msl: bool,
}

/// Display and visualization configuration
#[derive(Debug, Deserialize, Serialize)]
pub struct DisplayConfig {
    /// Time window in hours (shows -window to +window from current time)
    pub time_window_hours: i64,
    /// Cache TTL in minutes
    pub cache_ttl_minutes: u64,
    /// E-ink display width in pixels
    pub width: i32,
    /// E-ink display height in pixels
    pub height: i32,
    /// Font size for e-ink display (affects text rendering)
    pub font_height: i32,
    /// Hardware GPIO pin configuration
    pub hardware: HardwareConfig,
}

/// Hardware GPIO pin configuration for e-ink display
///
/// Default pin mapping for Waveshare 4.2" e-ink display on Raspberry Pi Zero 2 W:
/// - CS (Chip Select): GPIO 8 (Pin 24, CE0) - SPI device selection (kernel-controlled by default)
/// - DC (Data/Command): GPIO 25 (Pin 22) - Data vs command mode  
/// - RST (Reset): GPIO 17 (Pin 11) - Hardware reset signal
/// - BUSY: GPIO 24 (Pin 18) - Display busy status indicator
///
/// You may override `cs_pin` (e.g., to 7 for CE1/SS1/manual CS) if GPIO 8 is damaged.
#[derive(Debug, Deserialize, Serialize)]
pub struct HardwareConfig {
    /// SPI Chip Select pin (default: GPIO 8, Pin 24, CE0). If not 8, toggled manually.
    #[serde(default = "default_cs_pin")]
    pub cs_pin: u32,
    /// Data/Command control pin (default: GPIO 25, Pin 22)
    pub dc_pin: u32,
    /// Reset pin (default: GPIO 17, Pin 11)
    pub rst_pin: u32,
    /// Busy status pin (default: GPIO 24, Pin 18)
    pub busy_pin: u32,
}

fn default_cs_pin() -> u32 {
    8
}

impl Default for Config {
    fn default() -> Self {
        Config {
            station: StationConfig {
                id: "8418150".to_string(),
                name: "Portland, ME".to_string(),
                msl_offset: 4.9,
                show_msl: false, // Default to traditional MLLW display
            },
            display: DisplayConfig {
                time_window_hours: 12,
                cache_ttl_minutes: 30,
                width: 400,      // Waveshare 4.2" display
                height: 300,     // Waveshare 4.2" display
                font_height: 20, // FONT_10X20 height
                hardware: HardwareConfig {
                    cs_pin: 8,    // GPIO 8 (Pin 24) - SPI Chip Select
                    dc_pin: 25,   // GPIO 25 (Pin 22) - Data/Command
                    rst_pin: 17,  // GPIO 17 (Pin 11) - Reset
                    busy_pin: 24, // GPIO 24 (Pin 18) - Busy status
                },
            },
        }
    }
}

impl Config {
    /// Load configuration from tide-config.toml file
    /// Falls back to default configuration if file doesn't exist or is invalid
    pub fn load() -> Self {
        Self::load_from_path("tide-config.toml")
    }

    /// Load configuration from specified path
    /// Falls back to default configuration if file doesn't exist or is invalid
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Self {
        match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Config>(&contents) {
                Ok(config) => {
                    println!("Loaded configuration for station: {}", config.station.name);
                    config
                }
                Err(e) => {
                    eprintln!("Warning: Invalid config file format: {}", e);
                    eprintln!("Using default configuration (Portland, ME)");
                    Self::default()
                }
            },
            Err(_) => {
                eprintln!("Info: No config file found, using default configuration (Portland, ME)");
                Self::default()
            }
        }
    }

    /// Save current configuration to tide-config.toml
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(self)?;
        fs::write("tide-config.toml", contents)?;
        println!("Configuration saved to tide-config.toml");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.station.id, "8418150");
        assert_eq!(config.station.name, "Portland, ME");
        assert_eq!(config.station.msl_offset, 4.9);
        assert_eq!(config.display.time_window_hours, 12);
        assert_eq!(config.display.cache_ttl_minutes, 30);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.station.id, parsed.station.id);
        assert_eq!(config.station.name, parsed.station.name);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let config = Config::load_from_path("/nonexistent/path");
        // Should fallback to default
        assert_eq!(config.station.id, "8418150");
    }
}

use chrono::{NaiveDate, NaiveDateTime};

pub const VERSION: &str = "0.2";

/// Display theme — controls the OKLCH hue sweep used throughout.
#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Blue,    // cyan(200°) → indigo(280°)
    Warm,    // indigo(280°) → red(360°) → orange(40°)  [default]
    Rainbow, // cyan(200°) → indigo → red → orange(40°)  --color-me
}

/// Unit system for display.
#[derive(Clone, Copy, PartialEq)]
pub enum Units {
    Metric,   // default: °C, km/h, mm, hPa
    Imperial, // --strange-units: °F, mph, in, inHg
    British,  // --yes-sir: °C, mph, mm, hPa
}

impl Units {
    pub fn use_fahrenheit(self) -> bool { self == Units::Imperial }
    pub fn use_mph(self)        -> bool { self != Units::Metric }
    pub fn use_inches(self)     -> bool { self == Units::Imperial }
    pub fn use_inhg(self)       -> bool { self == Units::Imperial }
}

/// Forecast mode — Standard is the only implemented mode;
/// Drone and Pilot are reserved for future profiles.
#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum Mode {
    Standard,
    Drone,
    Pilot,
}

#[derive(Debug)]
pub struct HourlyData {
    pub time: NaiveDateTime,
    pub temp: f64,
    pub apparent_temp: f64,
    pub precip: f64,
    pub precip_prob: f64,
    pub pressure: f64,
    pub humidity: f64,
    pub cloud: f64,
    pub wind_speed: f64,
    pub wind_gust: f64,
}

#[derive(Debug)]
pub struct DroneHourlyData {
    pub time: NaiveDateTime,
    pub temp: f64,
    pub apparent_temp: f64,
    pub precip_prob: f64,
    pub precip: f64,
    pub wind_speed_10m: f64,
    pub wind_speed_80m: f64,
    pub wind_speed_120m: f64,
    pub wind_speed_180m: f64,
    pub wind_dir_10m: f64,
    pub wind_dir_80m: f64,
    pub wind_dir_120m: f64,
    pub wind_dir_180m: f64,
    pub wind_gust_10m: f64,
    pub uv_index: f64,
}

#[derive(Debug)]
pub struct DroneDaySummary {
    pub date: NaiveDate,
    pub sunrise: NaiveDateTime,
    pub sunset: NaiveDateTime,
    pub max_temp: f64,
    pub min_temp: f64,
    pub max_precip_prob: f64,
    pub total_precip: f64,
    pub max_wind_10m: f64,
    pub max_wind_80m: f64,
    pub max_wind_120m: f64,
    pub max_wind_180m: f64,
    pub max_gust_10m: f64,
    pub max_uv: f64,
}

#[derive(Debug)]
pub struct DaySummary {
    pub date: NaiveDate,
    pub sunrise: NaiveDateTime,
    pub sunset: NaiveDateTime,
    pub max_temp: f64,
    pub min_temp: f64,
    pub max_apparent: f64,
    pub min_apparent: f64,
    pub avg_cloud: f64,
    pub max_precip_prob: f64,
    pub total_precip: f64,
    pub avg_pressure: f64,
    pub avg_humidity: f64,
    pub max_wind_speed: f64,
    pub max_wind_gust: f64,
}

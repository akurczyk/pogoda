use chrono::{NaiveDate, NaiveDateTime};

pub const VERSION: &str = "0.1";

/// Display theme — controls the OKLCH hue sweep used throughout.
#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Blue, // cyan(200°) → indigo(280°)  [default]
    Warm, // indigo(280°) → red(360°) → orange(40°)
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
pub struct DaySummary {
    pub date: NaiveDate,
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

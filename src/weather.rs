use chrono::{NaiveDate, NaiveDateTime, Datelike};
use serde::Deserialize;

use crate::types::{DaySummary, HourlyData};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HourlyUnits {
    temperature_2m: String,
    apparent_temperature: String,
    precipitation: String,
    precipitation_probability: String,
    pressure_msl: String,
    relative_humidity_2m: String,
    cloud_cover: String,
    wind_speed_10m: String,
    wind_gusts_10m: String,
}

#[derive(Debug, Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f64>,
    apparent_temperature: Vec<f64>,
    precipitation: Vec<f64>,
    precipitation_probability: Vec<f64>,
    pressure_msl: Vec<f64>,
    relative_humidity_2m: Vec<f64>,
    cloud_cover: Vec<f64>,
    wind_speed_10m: Vec<f64>,
    wind_gusts_10m: Vec<f64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WeatherResponse {
    hourly_units: HourlyUnits,
    hourly: Hourly,
}

pub fn build_url(lat: f64, lng: f64, days: u32) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat}&longitude={lng}\
         &hourly=temperature_2m,precipitation,apparent_temperature,\
precipitation_probability,pressure_msl,relative_humidity_2m,\
cloud_cover,wind_speed_10m,wind_gusts_10m\
         &timezone=auto&forecast_days={days}"
    )
}

pub fn fetch_weather(lat: f64, lng: f64, days: u32) -> anyhow::Result<(String, Vec<HourlyData>)> {
    let url = build_url(lat, lng, days);
    let resp: WeatherResponse = reqwest::blocking::get(&url)?.json()?;
    let h = &resp.hourly;
    let data = h.time.iter().enumerate().map(|(i, t)| {
        let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M").unwrap();
        HourlyData {
            time,
            temp: h.temperature_2m[i],
            apparent_temp: h.apparent_temperature[i],
            precip: h.precipitation[i],
            precip_prob: h.precipitation_probability[i],
            pressure: h.pressure_msl[i],
            humidity: h.relative_humidity_2m[i],
            cloud: h.cloud_cover[i],
            wind_speed: h.wind_speed_10m[i],
            wind_gust: h.wind_gusts_10m[i],
        }
    }).collect();
    Ok((url, data))
}

pub fn day_summary(data: &[HourlyData], date: NaiveDate) -> DaySummary {
    let day: Vec<&HourlyData> = data.iter().filter(|h| h.time.date() == date).collect();
    let temps: Vec<f64> = day.iter().map(|h| h.temp).collect();
    let apparents: Vec<f64> = day.iter().map(|h| h.apparent_temp).collect();
    DaySummary {
        date,
        max_temp: temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        min_temp: temps.iter().cloned().fold(f64::INFINITY, f64::min),
        max_apparent: apparents.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        min_apparent: apparents.iter().cloned().fold(f64::INFINITY, f64::min),
        avg_cloud: day.iter().map(|h| h.cloud).sum::<f64>() / day.len() as f64,
        max_precip_prob: day.iter().map(|h| h.precip_prob).fold(f64::NEG_INFINITY, f64::max),
        total_precip: day.iter().map(|h| h.precip).sum::<f64>(),
        avg_pressure: day.iter().map(|h| h.pressure).sum::<f64>() / day.len() as f64,
        avg_humidity: day.iter().map(|h| h.humidity).sum::<f64>() / day.len() as f64,
        max_wind_speed: day.iter().map(|h| h.wind_speed).fold(f64::NEG_INFINITY, f64::max),
        max_wind_gust: day.iter().map(|h| h.wind_gust).fold(f64::NEG_INFINITY, f64::max),
    }
}

pub fn day_name(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    }
}

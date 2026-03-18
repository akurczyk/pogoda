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
    temperature_2m: Vec<Option<f64>>,
    apparent_temperature: Vec<Option<f64>>,
    precipitation: Vec<Option<f64>>,
    precipitation_probability: Vec<Option<f64>>,
    pressure_msl: Vec<Option<f64>>,
    relative_humidity_2m: Vec<Option<f64>>,
    cloud_cover: Vec<Option<f64>>,
    wind_speed_10m: Vec<Option<f64>>,
    wind_gusts_10m: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize)]
struct Daily {
    time: Vec<String>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WeatherResponse {
    hourly_units: HourlyUnits,
    hourly: Hourly,
    daily: Daily,
}

pub fn build_url(lat: f64, lng: f64, days: u32) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat}&longitude={lng}\
         &hourly=temperature_2m,precipitation,apparent_temperature,\
precipitation_probability,pressure_msl,relative_humidity_2m,\
cloud_cover,wind_speed_10m,wind_gusts_10m\
         &daily=sunrise,sunset\
         &timezone=auto&forecast_days={days}"
    )
}

pub fn fetch_weather(lat: f64, lng: f64, days: u32) -> anyhow::Result<(String, Vec<HourlyData>, Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>)> {
    let url = build_url(lat, lng, days);
    let resp: WeatherResponse = reqwest::blocking::get(&url)?.json()?;
    let h = &resp.hourly;
    let data = h.time.iter().enumerate().map(|(i, t)| {
        let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M").unwrap();
        HourlyData {
            time,
            temp: h.temperature_2m[i].unwrap_or(0.0),
            apparent_temp: h.apparent_temperature[i].unwrap_or(0.0),
            precip: h.precipitation[i].unwrap_or(0.0),
            precip_prob: h.precipitation_probability[i].unwrap_or(0.0),
            pressure: h.pressure_msl[i].unwrap_or(0.0),
            humidity: h.relative_humidity_2m[i].unwrap_or(0.0),
            cloud: h.cloud_cover[i].unwrap_or(0.0),
            wind_speed: h.wind_speed_10m[i].unwrap_or(0.0),
            wind_gust: h.wind_gusts_10m[i].unwrap_or(0.0),
        }
    }).collect();

    let d = &resp.daily;
    let solar: Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)> = d.time.iter().enumerate().map(|(i, t)| {
        let date = NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap();
        let fallback_rise = date.and_hms_opt(6, 0, 0).unwrap();
        let fallback_set  = date.and_hms_opt(20, 0, 0).unwrap();
        let sunrise = NaiveDateTime::parse_from_str(&d.sunrise[i], "%Y-%m-%dT%H:%M")
            .unwrap_or(fallback_rise);
        let sunset  = NaiveDateTime::parse_from_str(&d.sunset[i],  "%Y-%m-%dT%H:%M")
            .unwrap_or(fallback_set);
        (date, sunrise, sunset)
    }).collect();

    Ok((url, data, solar))
}

pub fn day_summary(data: &[HourlyData], solar: &[(NaiveDate, NaiveDateTime, NaiveDateTime)], date: NaiveDate) -> DaySummary {
    let day: Vec<&HourlyData> = data.iter().filter(|h| h.time.date() == date).collect();
    let temps: Vec<f64> = day.iter().map(|h| h.temp).collect();
    let apparents: Vec<f64> = day.iter().map(|h| h.apparent_temp).collect();
    let (sunrise, sunset) = solar.iter()
        .find(|(d, _, _)| *d == date)
        .map(|(_, rise, set)| (*rise, *set))
        .unwrap_or_else(|| (
            date.and_hms_opt(6, 0, 0).unwrap(),
            date.and_hms_opt(20, 0, 0).unwrap(),
        ));
    DaySummary {
        date,
        sunrise,
        sunset,
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

#[derive(Debug, Deserialize)]
struct DroneHourly {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    apparent_temperature: Vec<Option<f64>>,
    precipitation_probability: Vec<Option<f64>>,
    precipitation: Vec<Option<f64>>,
    wind_speed_10m: Vec<Option<f64>>,
    wind_speed_80m: Vec<Option<f64>>,
    wind_speed_120m: Vec<Option<f64>>,
    wind_speed_180m: Vec<Option<f64>>,
    wind_direction_10m: Vec<Option<f64>>,
    wind_direction_80m: Vec<Option<f64>>,
    wind_direction_120m: Vec<Option<f64>>,
    wind_direction_180m: Vec<Option<f64>>,
    wind_gusts_10m: Vec<Option<f64>>,
    uv_index: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize)]
struct DroneWeatherResponse {
    hourly: DroneHourly,
    daily: Daily,
}

pub fn build_drone_url(lat: f64, lng: f64, days: u32) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat}&longitude={lng}\
         &hourly=temperature_2m,apparent_temperature,\
precipitation_probability,precipitation,\
wind_speed_10m,wind_speed_80m,wind_speed_120m,wind_speed_180m,\
wind_direction_10m,wind_direction_80m,wind_direction_120m,wind_direction_180m,\
wind_gusts_10m,uv_index\
         &daily=sunrise,sunset\
         &timezone=auto&forecast_days={days}"
    )
}

pub fn fetch_drone_weather(lat: f64, lng: f64, days: u32)
    -> anyhow::Result<(String, Vec<crate::types::DroneHourlyData>, Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>)>
{
    use crate::types::DroneHourlyData;
    let url = build_drone_url(lat, lng, days);
    let resp: DroneWeatherResponse = reqwest::blocking::get(&url)?.json()?;
    let h = &resp.hourly;
    let data = h.time.iter().enumerate().map(|(i, t)| {
        let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M").unwrap();
        DroneHourlyData {
            time,
            temp: h.temperature_2m[i].unwrap_or(0.0),
            apparent_temp: h.apparent_temperature[i].unwrap_or(0.0),
            precip_prob: h.precipitation_probability[i].unwrap_or(0.0),
            precip: h.precipitation[i].unwrap_or(0.0),
            wind_speed_10m: h.wind_speed_10m[i].unwrap_or(0.0),
            wind_speed_80m: h.wind_speed_80m[i].unwrap_or(0.0),
            wind_speed_120m: h.wind_speed_120m[i].unwrap_or(0.0),
            wind_speed_180m: h.wind_speed_180m[i].unwrap_or(0.0),
            wind_dir_10m: h.wind_direction_10m[i].unwrap_or(0.0),
            wind_dir_80m: h.wind_direction_80m[i].unwrap_or(0.0),
            wind_dir_120m: h.wind_direction_120m[i].unwrap_or(0.0),
            wind_dir_180m: h.wind_direction_180m[i].unwrap_or(0.0),
            wind_gust_10m: h.wind_gusts_10m[i].unwrap_or(0.0),
            uv_index: h.uv_index[i].unwrap_or(0.0),
        }
    }).collect();

    let d = &resp.daily;
    let solar: Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)> = d.time.iter().enumerate().map(|(i, t)| {
        let date = NaiveDate::parse_from_str(t, "%Y-%m-%d").unwrap();
        let fallback_rise = date.and_hms_opt(6, 0, 0).unwrap();
        let fallback_set  = date.and_hms_opt(20, 0, 0).unwrap();
        let sunrise = NaiveDateTime::parse_from_str(&d.sunrise[i], "%Y-%m-%dT%H:%M").unwrap_or(fallback_rise);
        let sunset  = NaiveDateTime::parse_from_str(&d.sunset[i],  "%Y-%m-%dT%H:%M").unwrap_or(fallback_set);
        (date, sunrise, sunset)
    }).collect();

    Ok((url, data, solar))
}

pub fn drone_day_summary(
    data: &[crate::types::DroneHourlyData],
    solar: &[(NaiveDate, NaiveDateTime, NaiveDateTime)],
    date: NaiveDate,
) -> crate::types::DroneDaySummary {
    use crate::types::DroneDaySummary;
    let day: Vec<_> = data.iter().filter(|h| h.time.date() == date).collect();
    let (sunrise, sunset) = solar.iter()
        .find(|(d, _, _)| *d == date)
        .map(|(_, r, s)| (*r, *s))
        .unwrap_or_else(|| (
            date.and_hms_opt(6, 0, 0).unwrap(),
            date.and_hms_opt(20, 0, 0).unwrap(),
        ));
    DroneDaySummary {
        date,
        sunrise,
        sunset,
        max_temp: day.iter().map(|h| h.temp).fold(f64::NEG_INFINITY, f64::max),
        min_temp: day.iter().map(|h| h.temp).fold(f64::INFINITY, f64::min),
        max_precip_prob: day.iter().map(|h| h.precip_prob).fold(f64::NEG_INFINITY, f64::max),
        total_precip: day.iter().map(|h| h.precip).sum(),
        max_wind_10m:  day.iter().map(|h| h.wind_speed_10m).fold(f64::NEG_INFINITY, f64::max),
        max_wind_80m:  day.iter().map(|h| h.wind_speed_80m).fold(f64::NEG_INFINITY, f64::max),
        max_wind_120m: day.iter().map(|h| h.wind_speed_120m).fold(f64::NEG_INFINITY, f64::max),
        max_wind_180m: day.iter().map(|h| h.wind_speed_180m).fold(f64::NEG_INFINITY, f64::max),
        max_gust_10m:  day.iter().map(|h| h.wind_gust_10m).fold(f64::NEG_INFINITY, f64::max),
        max_uv:        day.iter().map(|h| h.uv_index).fold(f64::NEG_INFINITY, f64::max),
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

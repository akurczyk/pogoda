use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::Deserialize;

use crate::types::{DaySummary, HistoricalDailyData, HourlyData};

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
struct WeatherResponse {
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

pub fn parse_forecast(
    body: &str,
) -> anyhow::Result<(
    Vec<HourlyData>,
    Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>,
)> {
    let resp: WeatherResponse = serde_json::from_str(body)?;
    let h = &resp.hourly;
    let data: anyhow::Result<Vec<HourlyData>> = h
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M")
                .map_err(|e| anyhow::anyhow!("invalid time {:?}: {}", t, e))?;
            Ok(HourlyData {
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
            })
        })
        .collect();
    let data = data?;

    let d = &resp.daily;
    let solar: anyhow::Result<Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>> = d
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let date = NaiveDate::parse_from_str(t, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("invalid date {:?}: {}", t, e))?;
            let fallback_rise = date.and_hms_opt(6, 0, 0).unwrap();
            let fallback_set = date.and_hms_opt(20, 0, 0).unwrap();
            let sunrise = NaiveDateTime::parse_from_str(&d.sunrise[i], "%Y-%m-%dT%H:%M")
                .unwrap_or(fallback_rise);
            let sunset = NaiveDateTime::parse_from_str(&d.sunset[i], "%Y-%m-%dT%H:%M")
                .unwrap_or(fallback_set);
            Ok((date, sunrise, sunset))
        })
        .collect();
    let solar = solar?;

    Ok((data, solar))
}

pub fn fetch_weather(
    lat: f64,
    lng: f64,
    days: u32,
) -> anyhow::Result<(
    String,
    Vec<HourlyData>,
    Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>,
)> {
    let url = build_url(lat, lng, days);
    let body = reqwest::blocking::get(&url)?.text()?;
    let (data, solar) = parse_forecast(&body)?;
    Ok((url, data, solar))
}

pub fn day_summary(
    data: &[HourlyData],
    solar: &[(NaiveDate, NaiveDateTime, NaiveDateTime)],
    date: NaiveDate,
) -> DaySummary {
    let day: Vec<&HourlyData> = data.iter().filter(|h| h.time.date() == date).collect();
    let temps: Vec<f64> = day.iter().map(|h| h.temp).collect();
    let apparents: Vec<f64> = day.iter().map(|h| h.apparent_temp).collect();
    let (sunrise, sunset) = solar
        .iter()
        .find(|(d, _, _)| *d == date)
        .map(|(_, rise, set)| (*rise, *set))
        .unwrap_or_else(|| {
            (
                date.and_hms_opt(6, 0, 0).unwrap(),
                date.and_hms_opt(20, 0, 0).unwrap(),
            )
        });
    DaySummary {
        date,
        sunrise,
        sunset,
        max_temp: temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        min_temp: temps.iter().cloned().fold(f64::INFINITY, f64::min),
        max_apparent: apparents.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        min_apparent: apparents.iter().cloned().fold(f64::INFINITY, f64::min),
        avg_cloud: day.iter().map(|h| h.cloud).sum::<f64>() / day.len() as f64,
        max_precip_prob: day
            .iter()
            .map(|h| h.precip_prob)
            .fold(f64::NEG_INFINITY, f64::max),
        total_precip: day.iter().map(|h| h.precip).sum::<f64>(),
        avg_pressure: day.iter().map(|h| h.pressure).sum::<f64>() / day.len() as f64,
        avg_humidity: day.iter().map(|h| h.humidity).sum::<f64>() / day.len() as f64,
        max_wind_speed: day
            .iter()
            .map(|h| h.wind_speed)
            .fold(f64::NEG_INFINITY, f64::max),
        max_wind_gust: day
            .iter()
            .map(|h| h.wind_gust)
            .fold(f64::NEG_INFINITY, f64::max),
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

pub fn parse_drone(
    body: &str,
) -> anyhow::Result<(
    Vec<crate::types::DroneHourlyData>,
    Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>,
)> {
    use crate::types::DroneHourlyData;
    let resp: DroneWeatherResponse = serde_json::from_str(body)?;
    let h = &resp.hourly;
    let data: anyhow::Result<Vec<DroneHourlyData>> = h
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M")
                .map_err(|e| anyhow::anyhow!("invalid time {:?}: {}", t, e))?;
            Ok(DroneHourlyData {
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
            })
        })
        .collect();
    let data = data?;

    let d = &resp.daily;
    let solar: anyhow::Result<Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>> = d
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let date = NaiveDate::parse_from_str(t, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("invalid date {:?}: {}", t, e))?;
            let fallback_rise = date.and_hms_opt(6, 0, 0).unwrap();
            let fallback_set = date.and_hms_opt(20, 0, 0).unwrap();
            let sunrise = NaiveDateTime::parse_from_str(&d.sunrise[i], "%Y-%m-%dT%H:%M")
                .unwrap_or(fallback_rise);
            let sunset = NaiveDateTime::parse_from_str(&d.sunset[i], "%Y-%m-%dT%H:%M")
                .unwrap_or(fallback_set);
            Ok((date, sunrise, sunset))
        })
        .collect();
    let solar = solar?;

    Ok((data, solar))
}

pub fn fetch_drone_weather(
    lat: f64,
    lng: f64,
    days: u32,
) -> anyhow::Result<(
    String,
    Vec<crate::types::DroneHourlyData>,
    Vec<(NaiveDate, NaiveDateTime, NaiveDateTime)>,
)> {
    let url = build_drone_url(lat, lng, days);
    let body = reqwest::blocking::get(&url)?.text()?;
    let (data, solar) = parse_drone(&body)?;
    Ok((url, data, solar))
}

pub fn drone_day_summary(
    data: &[crate::types::DroneHourlyData],
    solar: &[(NaiveDate, NaiveDateTime, NaiveDateTime)],
    date: NaiveDate,
) -> crate::types::DroneDaySummary {
    use crate::types::DroneDaySummary;
    let day: Vec<_> = data.iter().filter(|h| h.time.date() == date).collect();
    let (sunrise, sunset) = solar
        .iter()
        .find(|(d, _, _)| *d == date)
        .map(|(_, r, s)| (*r, *s))
        .unwrap_or_else(|| {
            (
                date.and_hms_opt(6, 0, 0).unwrap(),
                date.and_hms_opt(20, 0, 0).unwrap(),
            )
        });
    DroneDaySummary {
        date,
        sunrise,
        sunset,
        max_temp: day.iter().map(|h| h.temp).fold(f64::NEG_INFINITY, f64::max),
        min_temp: day.iter().map(|h| h.temp).fold(f64::INFINITY, f64::min),
        max_precip_prob: day
            .iter()
            .map(|h| h.precip_prob)
            .fold(f64::NEG_INFINITY, f64::max),
        total_precip: day.iter().map(|h| h.precip).sum(),
        max_wind_10m: day
            .iter()
            .map(|h| h.wind_speed_10m)
            .fold(f64::NEG_INFINITY, f64::max),
        max_wind_80m: day
            .iter()
            .map(|h| h.wind_speed_80m)
            .fold(f64::NEG_INFINITY, f64::max),
        max_wind_120m: day
            .iter()
            .map(|h| h.wind_speed_120m)
            .fold(f64::NEG_INFINITY, f64::max),
        max_wind_180m: day
            .iter()
            .map(|h| h.wind_speed_180m)
            .fold(f64::NEG_INFINITY, f64::max),
        max_gust_10m: day
            .iter()
            .map(|h| h.wind_gust_10m)
            .fold(f64::NEG_INFINITY, f64::max),
        max_uv: day
            .iter()
            .map(|h| h.uv_index)
            .fold(f64::NEG_INFINITY, f64::max),
    }
}

/// Fetch hourly historical data (use for ranges ≤ 31 days).
/// Returns (url, Vec<HourlyData>) — precip_prob is derived from precipitation.
pub fn parse_historical_hourly(body: &str) -> anyhow::Result<Vec<HourlyData>> {
    #[derive(Deserialize)]
    struct H {
        time: Vec<String>,
        temperature_2m: Vec<Option<f64>>,
        apparent_temperature: Vec<Option<f64>>,
        precipitation: Vec<Option<f64>>,
        pressure_msl: Vec<Option<f64>>,
        relative_humidity_2m: Vec<Option<f64>>,
        cloud_cover: Vec<Option<f64>>,
        wind_speed_10m: Vec<Option<f64>>,
        wind_gusts_10m: Vec<Option<f64>>,
    }
    #[derive(Deserialize)]
    struct Resp {
        hourly: H,
    }

    let raw: serde_json::Value = serde_json::from_str(body)?;
    if let Some(reason) = raw.get("reason").and_then(|r| r.as_str()) {
        anyhow::bail!("{}", reason);
    }
    let resp: Resp = serde_json::from_value(raw)?;
    let h = &resp.hourly;
    let data: anyhow::Result<Vec<HourlyData>> = h
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let time = NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M")
                .map_err(|e| anyhow::anyhow!("invalid time {:?}: {}", t, e))?;
            let precip = h.precipitation[i].unwrap_or(0.0);
            Ok(HourlyData {
                time,
                temp: h.temperature_2m[i].unwrap_or(0.0),
                apparent_temp: h.apparent_temperature[i].unwrap_or(0.0),
                precip,
                precip_prob: if precip > 0.0 { 100.0 } else { 0.0 },
                pressure: h.pressure_msl[i].unwrap_or(0.0),
                humidity: h.relative_humidity_2m[i].unwrap_or(0.0),
                cloud: h.cloud_cover[i].unwrap_or(0.0),
                wind_speed: h.wind_speed_10m[i].unwrap_or(0.0),
                wind_gust: h.wind_gusts_10m[i].unwrap_or(0.0),
            })
        })
        .collect();
    Ok(data?)
}

pub fn fetch_historical_hourly(
    lat: f64,
    lng: f64,
    start: NaiveDate,
    end: NaiveDate,
) -> anyhow::Result<(String, Vec<HourlyData>)> {
    let url = format!(
        "https://archive-api.open-meteo.com/v1/archive\
         ?latitude={lat}&longitude={lng}\
         &hourly=temperature_2m,apparent_temperature,precipitation,\
pressure_msl,relative_humidity_2m,cloud_cover,wind_speed_10m,wind_gusts_10m\
         &start_date={start}&end_date={end}&timezone=auto"
    );
    let body = reqwest::blocking::get(&url)?.text()?;
    let data = parse_historical_hourly(&body)?;
    Ok((url, data))
}

/// Fetch daily historical data (use for ranges > 31 days).
pub fn parse_historical_daily(body: &str) -> anyhow::Result<Vec<HistoricalDailyData>> {
    #[derive(Deserialize)]
    struct D {
        time: Vec<String>,
        temperature_2m_max: Vec<Option<f64>>,
        temperature_2m_min: Vec<Option<f64>>,
        precipitation_sum: Vec<Option<f64>>,
        wind_speed_10m_max: Vec<Option<f64>>,
        wind_gusts_10m_max: Vec<Option<f64>>,
    }
    #[derive(Deserialize)]
    struct Resp {
        daily: D,
    }

    let raw: serde_json::Value = serde_json::from_str(body)?;
    if let Some(reason) = raw.get("reason").and_then(|r| r.as_str()) {
        anyhow::bail!("{}", reason);
    }
    let resp: Resp = serde_json::from_value(raw)?;
    let d = &resp.daily;
    let data: anyhow::Result<Vec<HistoricalDailyData>> = d
        .time
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let date = NaiveDate::parse_from_str(t, "%Y-%m-%d")
                .map_err(|e| anyhow::anyhow!("invalid date {:?}: {}", t, e))?;
            Ok(HistoricalDailyData {
                date,
                max_temp: d.temperature_2m_max[i].unwrap_or(0.0),
                min_temp: d.temperature_2m_min[i].unwrap_or(0.0),
                precip_sum: d.precipitation_sum[i].unwrap_or(0.0),
                wind_max: d.wind_speed_10m_max[i].unwrap_or(0.0),
                gust_max: d.wind_gusts_10m_max[i].unwrap_or(0.0),
            })
        })
        .collect();
    Ok(data?)
}

pub fn fetch_historical_daily(
    lat: f64,
    lng: f64,
    start: NaiveDate,
    end: NaiveDate,
) -> anyhow::Result<(String, Vec<HistoricalDailyData>)> {
    let url = format!(
        "https://archive-api.open-meteo.com/v1/archive\
         ?latitude={lat}&longitude={lng}\
         &daily=temperature_2m_max,temperature_2m_min,\
precipitation_sum,wind_speed_10m_max,wind_gusts_10m_max\
         &start_date={start}&end_date={end}&timezone=auto"
    );
    let body = reqwest::blocking::get(&url)?.text()?;
    let data = parse_historical_daily(&body)?;
    Ok((url, data))
}

/// Aggregate daily data into monthly buckets.
pub fn aggregate_monthly(days: &[HistoricalDailyData]) -> Vec<crate::types::HistoricalMonthlyData> {
    use crate::types::HistoricalMonthlyData;
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<(i32, u32), Vec<&HistoricalDailyData>> = BTreeMap::new();
    for d in days {
        buckets
            .entry((d.date.year(), d.date.month()))
            .or_default()
            .push(d);
    }
    buckets
        .into_iter()
        .map(|((year, month), ds)| {
            let n = ds.len() as f64;
            HistoricalMonthlyData {
                year,
                month,
                avg_max_temp: ds.iter().map(|d| d.max_temp).sum::<f64>() / n,
                avg_min_temp: ds.iter().map(|d| d.min_temp).sum::<f64>() / n,
                extreme_max_temp: ds
                    .iter()
                    .map(|d| d.max_temp)
                    .fold(f64::NEG_INFINITY, f64::max),
                extreme_min_temp: ds.iter().map(|d| d.min_temp).fold(f64::INFINITY, f64::min),
                precip_sum: ds.iter().map(|d| d.precip_sum).sum(),
                wind_max: ds
                    .iter()
                    .map(|d| d.wind_max)
                    .fold(f64::NEG_INFINITY, f64::max),
                gust_max: ds
                    .iter()
                    .map(|d| d.gust_max)
                    .fold(f64::NEG_INFINITY, f64::max),
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HistoricalDailyData, HourlyData};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn dt(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> NaiveDateTime {
        date(y, mo, d).and_hms_opt(h, mi, 0).unwrap()
    }

    // ── URL builders ────────────────────────────────────────────────────────

    #[test]
    fn build_url_contains_coords_and_days() {
        let url = build_url(52.52, 13.41, 7);
        assert!(url.contains("latitude=52.52"));
        assert!(url.contains("longitude=13.41"));
        assert!(url.contains("forecast_days=7"));
    }

    #[test]
    fn build_drone_url_contains_multi_altitude_wind() {
        let url = build_drone_url(0.0, 0.0, 3);
        assert!(url.contains("wind_speed_80m"));
        assert!(url.contains("wind_speed_120m"));
        assert!(url.contains("wind_speed_180m"));
        assert!(url.contains("uv_index"));
    }

    // ── day_name ─────────────────────────────────────────────────────────────

    #[test]
    fn day_name_known_dates() {
        assert_eq!(day_name(date(2026, 3, 20)), "Friday");
        assert_eq!(day_name(date(2026, 3, 21)), "Saturday");
        assert_eq!(day_name(date(2026, 3, 22)), "Sunday");
        assert_eq!(day_name(date(2026, 3, 23)), "Monday");
    }

    // ── aggregate_monthly ────────────────────────────────────────────────────

    #[test]
    fn aggregate_monthly_groups_correctly() {
        let days = vec![
            HistoricalDailyData {
                date: date(2024, 1, 1),
                max_temp: 10.0,
                min_temp: 0.0,
                precip_sum: 1.0,
                wind_max: 20.0,
                gust_max: 30.0,
            },
            HistoricalDailyData {
                date: date(2024, 1, 2),
                max_temp: 12.0,
                min_temp: 2.0,
                precip_sum: 0.0,
                wind_max: 15.0,
                gust_max: 25.0,
            },
            HistoricalDailyData {
                date: date(2024, 2, 1),
                max_temp: 5.0,
                min_temp: -5.0,
                precip_sum: 3.0,
                wind_max: 40.0,
                gust_max: 50.0,
            },
        ];
        let monthly = aggregate_monthly(&days);
        assert_eq!(monthly.len(), 2);

        let jan = &monthly[0];
        assert_eq!((jan.year, jan.month), (2024, 1));
        assert!((jan.avg_max_temp - 11.0).abs() < 0.001);
        assert!((jan.avg_min_temp - 1.0).abs() < 0.001);
        assert_eq!(jan.extreme_max_temp, 12.0);
        assert_eq!(jan.extreme_min_temp, 0.0);
        assert!((jan.precip_sum - 1.0).abs() < 0.001);
        assert_eq!(jan.wind_max, 20.0);
        assert_eq!(jan.gust_max, 30.0);

        let feb = &monthly[1];
        assert_eq!(feb.month, 2);
        assert!((feb.avg_max_temp - 5.0).abs() < 0.001);
        assert_eq!(feb.extreme_max_temp, 5.0);
        assert_eq!(feb.extreme_min_temp, -5.0);
    }

    // ── day_summary ──────────────────────────────────────────────────────────

    fn make_hour(
        y: i32,
        mo: u32,
        d: u32,
        h: u32,
        temp: f64,
        apparent: f64,
        precip: f64,
        precip_prob: f64,
        pressure: f64,
        humidity: f64,
        cloud: f64,
        wind: f64,
        gust: f64,
    ) -> HourlyData {
        HourlyData {
            time: dt(y, mo, d, h, 0),
            temp,
            apparent_temp: apparent,
            precip,
            precip_prob,
            pressure,
            humidity,
            cloud,
            wind_speed: wind,
            wind_gust: gust,
        }
    }

    #[test]
    fn day_summary_aggregates_correctly() {
        let data = vec![
            make_hour(
                2024, 6, 1, 0, 10.0, 9.0, 0.0, 0.0, 1013.0, 70.0, 50.0, 10.0, 20.0,
            ),
            make_hour(
                2024, 6, 1, 6, 15.0, 14.0, 1.0, 80.0, 1010.0, 80.0, 90.0, 20.0, 35.0,
            ),
            make_hour(
                2024, 6, 1, 12, 25.0, 24.0, 0.5, 40.0, 1012.0, 60.0, 20.0, 15.0, 25.0,
            ),
            make_hour(
                2024, 6, 2, 0, 12.0, 11.0, 0.0, 0.0, 1015.0, 65.0, 10.0, 5.0, 8.0,
            ),
        ];
        let solar = vec![
            (
                date(2024, 6, 1),
                dt(2024, 6, 1, 5, 0),
                dt(2024, 6, 1, 21, 0),
            ),
            (
                date(2024, 6, 2),
                dt(2024, 6, 2, 5, 1),
                dt(2024, 6, 2, 21, 1),
            ),
        ];
        let s = day_summary(&data, &solar, date(2024, 6, 1));

        assert_eq!(s.date, date(2024, 6, 1));
        assert_eq!(s.max_temp, 25.0);
        assert_eq!(s.min_temp, 10.0);
        assert_eq!(s.max_apparent, 24.0);
        assert_eq!(s.min_apparent, 9.0);
        assert!((s.total_precip - 1.5).abs() < 0.001);
        assert_eq!(s.max_precip_prob, 80.0);
        assert_eq!(s.max_wind_speed, 20.0);
        assert_eq!(s.max_wind_gust, 35.0);
        assert!((s.avg_pressure - 1011.667).abs() < 0.01);
        assert!((s.avg_humidity - 70.0).abs() < 0.001);
        assert!((s.avg_cloud - 53.333).abs() < 0.01);
        assert_eq!(s.sunrise, dt(2024, 6, 1, 5, 0));
        assert_eq!(s.sunset, dt(2024, 6, 1, 21, 0));
    }

    #[test]
    fn day_summary_fallback_solar() {
        // No solar entry — should fall back to 06:00/20:00
        let data = vec![make_hour(
            2024, 1, 1, 12, 5.0, 4.0, 0.0, 0.0, 1010.0, 75.0, 60.0, 8.0, 12.0,
        )];
        let s = day_summary(&data, &[], date(2024, 1, 1));
        assert_eq!(s.sunrise, dt(2024, 1, 1, 6, 0));
        assert_eq!(s.sunset, dt(2024, 1, 1, 20, 0));
    }

    // ── drone_day_summary ────────────────────────────────────────────────────

    fn make_drone_hour(
        y: i32,
        mo: u32,
        d: u32,
        h: u32,
        temp: f64,
        w10: f64,
        w80: f64,
        w120: f64,
        w180: f64,
        gust: f64,
        uv: f64,
        precip: f64,
        prob: f64,
    ) -> crate::types::DroneHourlyData {
        crate::types::DroneHourlyData {
            time: dt(y, mo, d, h, 0),
            temp,
            apparent_temp: temp - 1.0,
            precip_prob: prob,
            precip,
            wind_speed_10m: w10,
            wind_speed_80m: w80,
            wind_speed_120m: w120,
            wind_speed_180m: w180,
            wind_dir_10m: 180.0,
            wind_dir_80m: 190.0,
            wind_dir_120m: 200.0,
            wind_dir_180m: 210.0,
            wind_gust_10m: gust,
            uv_index: uv,
        }
    }

    #[test]
    fn drone_day_summary_aggregates_correctly() {
        let data = vec![
            make_drone_hour(
                2024, 7, 1, 0, 20.0, 5.0, 10.0, 12.0, 14.0, 18.0, 0.0, 0.0, 0.0,
            ),
            make_drone_hour(
                2024, 7, 1, 6, 22.0, 15.0, 20.0, 22.0, 25.0, 30.0, 3.5, 0.5, 60.0,
            ),
            make_drone_hour(
                2024, 7, 1, 12, 28.0, 10.0, 18.0, 20.0, 22.0, 25.0, 7.0, 0.2, 30.0,
            ),
            make_drone_hour(
                2024, 7, 2, 0, 18.0, 8.0, 12.0, 14.0, 16.0, 20.0, 0.0, 0.0, 0.0,
            ),
        ];
        let solar = vec![(
            date(2024, 7, 1),
            dt(2024, 7, 1, 5, 10),
            dt(2024, 7, 1, 21, 5),
        )];
        let s = drone_day_summary(&data, &solar, date(2024, 7, 1));

        assert_eq!(s.date, date(2024, 7, 1));
        assert_eq!(s.max_temp, 28.0);
        assert_eq!(s.min_temp, 20.0);
        assert_eq!(s.max_wind_10m, 15.0);
        assert_eq!(s.max_wind_80m, 20.0);
        assert_eq!(s.max_wind_120m, 22.0);
        assert_eq!(s.max_wind_180m, 25.0);
        assert_eq!(s.max_gust_10m, 30.0);
        assert_eq!(s.max_uv, 7.0);
        assert!((s.total_precip - 0.7).abs() < 0.001);
        assert_eq!(s.max_precip_prob, 60.0);
        assert_eq!(s.sunrise, dt(2024, 7, 1, 5, 10));
        assert_eq!(s.sunset, dt(2024, 7, 1, 21, 5));
    }

    // ── parse_forecast (fixture) ──────────────────────────────────────────────

    #[test]
    fn parse_forecast_fixture() {
        let body = include_str!("../tests/fixtures/forecast.json");
        let (data, solar) = parse_forecast(body).expect("parse failed");
        assert_eq!(
            data.len(),
            24,
            "expected 24 hourly records for 1-day forecast"
        );
        assert_eq!(solar.len(), 1);
        // Spot-check first record: temp should be a plausible value
        assert!(data[0].temp > -50.0 && data[0].temp < 60.0);
        // Solar times should be on the same date as the first data point
        assert_eq!(solar[0].0, data[0].time.date());
        // Sunrise before sunset
        assert!(solar[0].1 < solar[0].2);
    }

    // ── parse_drone (fixture) ─────────────────────────────────────────────────

    #[test]
    fn parse_drone_fixture() {
        let body = include_str!("../tests/fixtures/drone.json");
        let (data, solar) = parse_drone(body).expect("parse failed");
        assert_eq!(data.len(), 24);
        assert_eq!(solar.len(), 1);
        // All altitudes populated
        assert!(data[0].wind_speed_10m >= 0.0);
        assert!(data[0].wind_speed_80m >= 0.0);
        assert!(data[0].wind_speed_120m >= 0.0);
        assert!(data[0].wind_speed_180m >= 0.0);
        assert!(data[0].uv_index >= 0.0);
        // Direction in [0, 360]
        assert!(data[0].wind_dir_10m >= 0.0 && data[0].wind_dir_10m <= 360.0);
    }

    // ── parse_historical_hourly (fixture) ────────────────────────────────────

    #[test]
    fn parse_historical_hourly_fixture() {
        let body = include_str!("../tests/fixtures/historical_hourly.json");
        let data = parse_historical_hourly(body).expect("parse failed");
        // 2 days × 24 hours
        assert_eq!(data.len(), 48);
        // precip_prob derived: non-zero precip → 100, zero → 0
        let first_with_precip = data.iter().find(|h| h.precip > 0.0).unwrap();
        assert_eq!(first_with_precip.precip_prob, 100.0);
        let first_dry = data.iter().find(|h| h.precip == 0.0).unwrap();
        assert_eq!(first_dry.precip_prob, 0.0);
    }

    // ── parse_historical_daily (fixture) ─────────────────────────────────────

    #[test]
    fn parse_historical_daily_fixture() {
        let body = include_str!("../tests/fixtures/historical_daily.json");
        let data = parse_historical_daily(body).expect("parse failed");
        assert_eq!(data.len(), 3);
        // max_temp > min_temp for each day
        for d in &data {
            assert!(d.max_temp > d.min_temp, "max_temp should exceed min_temp");
            assert!(d.gust_max >= d.wind_max, "gusts should be >= wind max");
        }
    }
}

use serde::Deserialize;

pub fn geocode_city(name: &str) -> anyhow::Result<(f64, f64, String, String)> {
    #[derive(Deserialize)]
    struct GeoResult { name: String, country: String, latitude: f64, longitude: f64 }
    #[derive(Deserialize)]
    struct GeoResponse { results: Option<Vec<GeoResult>> }
    let client = reqwest::blocking::Client::new();
    let resp: GeoResponse = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", name), ("count", "1"), ("format", "json")])
        .send()?.json()?;
    let r = resp.results.and_then(|v| v.into_iter().next())
        .ok_or_else(|| anyhow::anyhow!("City '{}' not found", name))?;
    Ok((r.latitude, r.longitude, r.name, r.country))
}

pub fn reverse_geocode(lat: f64, lng: f64) -> anyhow::Result<(String, String)> {
    #[derive(Deserialize)]
    struct Addr {
        city: Option<String>,
        town: Option<String>,
        village: Option<String>,
        country: Option<String>,
    }
    #[derive(Deserialize)]
    struct NomResp { address: Addr }
    let lat_s = lat.to_string();
    let lng_s = lng.to_string();
    let client = reqwest::blocking::Client::new();
    let resp: NomResp = client
        .get("https://nominatim.openstreetmap.org/reverse")
        .query(&[("lat", lat_s.as_str()), ("lon", lng_s.as_str()), ("format", "json")])
        .header("User-Agent", "pogoda/0.8")
        .send()?.json()?;
    let city = resp.address.city
        .or(resp.address.town)
        .or(resp.address.village)
        .unwrap_or_else(|| format!("{:.4},{:.4}", lat, lng));
    let country = resp.address.country.unwrap_or_default();
    Ok((city, country))
}

/// Returns (from_day, to_day), both 1-based inclusive.
/// Accepts "7" → (1,7) or "5-7" → (5,7).
pub fn parse_days(s: Option<&String>) -> (u32, u32) {
    let Some(s) = s else { return (1, 7) };
    parse_days_str(s)
}

pub fn parse_days_str(s: &str) -> (u32, u32) {
    if let Some((a, b)) = s.split_once('-') {
        let from: u32 = a.parse().unwrap_or_else(|_| {
            eprintln!("Error: invalid day range '{}'.", s); std::process::exit(1);
        });
        let to: u32 = b.parse().unwrap_or_else(|_| {
            eprintln!("Error: invalid day range '{}'.", s); std::process::exit(1);
        });
        if from < 1 || to > 16 || from > to {
            eprintln!("Error: day range must be N-M where 1 ≤ N ≤ M ≤ 16.");
            std::process::exit(1);
        }
        (from, to)
    } else {
        let d: u32 = s.parse().unwrap_or_else(|_| {
            eprintln!("Error: '{}' is not a valid number of days.", s);
            std::process::exit(1);
        });
        if d < 1 || d > 16 {
            eprintln!("Error: days must be between 1 and 16.");
            std::process::exit(1);
        }
        (1, d)
    }
}

pub fn looks_like_days(s: &str) -> bool {
    if s.parse::<u32>().is_ok() { return true; }
    if let Some((a, b)) = s.split_once('-') {
        return a.parse::<u32>().is_ok() && b.parse::<u32>().is_ok();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_days_plain_number()  { assert!(looks_like_days("7")); }

    #[test]
    fn looks_like_days_range()         { assert!(looks_like_days("3-7")); }

    #[test]
    fn looks_like_days_city_name()     { assert!(!looks_like_days("Berlin")); }

    #[test]
    fn looks_like_days_float()         { assert!(!looks_like_days("3.14")); }

    #[test]
    fn parse_days_none_returns_default() {
        assert_eq!(parse_days(None), (1, 7));
    }

    #[test]
    fn parse_days_str_plain() {
        assert_eq!(parse_days_str("7"), (1, 7));
    }

    #[test]
    fn parse_days_str_plain_one() {
        assert_eq!(parse_days_str("1"), (1, 1));
    }

    #[test]
    fn parse_days_str_range() {
        assert_eq!(parse_days_str("3-7"), (3, 7));
    }

    #[test]
    fn parse_days_str_range_full() {
        assert_eq!(parse_days_str("1-16"), (1, 16));
    }
}

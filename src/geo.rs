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
        .header("User-Agent", "pogoda/0.1")
        .send()?.json()?;
    let city = resp.address.city
        .or(resp.address.town)
        .or(resp.address.village)
        .unwrap_or_else(|| format!("{:.4},{:.4}", lat, lng));
    let country = resp.address.country.unwrap_or_default();
    Ok((city, country))
}

pub fn parse_days(s: Option<&String>) -> u32 {
    let Some(s) = s else { return 7 };
    let d: u32 = s.parse().unwrap_or_else(|_| {
        eprintln!("Error: '{}' is not a valid number of days.", s);
        std::process::exit(1);
    });
    if d < 1 || d > 16 {
        eprintln!("Error: days must be between 1 and 16.");
        std::process::exit(1);
    }
    d
}

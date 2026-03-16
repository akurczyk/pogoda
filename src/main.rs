use chrono::{NaiveDateTime, Timelike, Datelike};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use serde::Deserialize;
use std::io::{self, Write as IoWrite};

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

#[derive(Debug)]
struct HourlyData {
    time: NaiveDateTime,
    temp: f64,
    apparent_temp: f64,
    precip: f64,
    precip_prob: f64,
    pressure: f64,
    humidity: f64,
    cloud: f64,
    wind_speed: f64,
    wind_gust: f64,
}

#[derive(Debug)]
struct DaySummary {
    date: chrono::NaiveDate,
    max_temp: f64,
    min_temp: f64,
    max_apparent: f64,
    min_apparent: f64,
    avg_cloud: f64,
    max_precip_prob: f64,
    total_precip: f64,
    avg_pressure: f64,
    avg_humidity: f64,
    max_wind_speed: f64,
    max_wind_gust: f64,
}

fn build_url(lat: f64, lng: f64, days: u32) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast\
         ?latitude={lat}&longitude={lng}\
         &hourly=temperature_2m,precipitation,apparent_temperature,\
precipitation_probability,pressure_msl,relative_humidity_2m,\
cloud_cover,wind_speed_10m,wind_gusts_10m\
         &timezone=auto&forecast_days={days}"
    )
}

fn fetch_weather(lat: f64, lng: f64, days: u32) -> anyhow::Result<(String, Vec<HourlyData>)> {
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

fn day_summary(data: &[HourlyData], date: chrono::NaiveDate) -> DaySummary {
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

fn day_name(date: chrono::NaiveDate) -> &'static str {
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

// ─── Color palette: OKLCH uniform perceptual brightness, cyan → indigo ───────
//   L=0.62, C=0.14, H: 200° (cyan) → 280° (indigo)

fn oklch_to_rgb(l: f64, c: f64, h_deg: f64) -> Color {
    let h = h_deg.to_radians();
    let a = c * h.cos();
    let b = c * h.sin();
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let rl =  4.0767416621 * l_.powi(3) - 3.3077115913 * m_.powi(3) + 0.2309699292 * s_.powi(3);
    let gl = -1.2684380046 * l_.powi(3) + 2.6097574011 * m_.powi(3) - 0.3413193965 * s_.powi(3);
    let bl = -0.0041960863 * l_.powi(3) - 0.7034186147 * m_.powi(3) + 1.7076147010 * s_.powi(3);
    let gamma = |x: f64| -> u8 {
        let x = x.clamp(0.0, 1.0);
        let s = if x <= 0.0031308 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    };
    Color::Rgb(gamma(rl), gamma(gl), gamma(bl))
}

fn palette(t: f64) -> Color { oklch_to_rgb(0.62, 0.14, 200.0 + t.clamp(0.0, 1.0) * 80.0) }

fn cloud_color(_pct: f64) -> Color { Color::DarkGray }
fn temp_color(t: f64) -> Color   { palette(((t + 15.0) / 45.0).clamp(0.0, 1.0)) }
fn wind_color(s: f64) -> Color   { palette((s / 60.0).clamp(0.0, 1.0)) }
fn pressure_color(p: f64) -> Color { palette(1.0 - ((p - 985.0) / 55.0).clamp(0.0, 1.0)) }

// ─── Bar drawing ─────────────────────────────────────────────────────────────

fn value_bar(value: f64, min: f64, max: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    if max <= min || width == 0 { return vec![Span::raw(" ".repeat(width))]; }
    let pos = (((value - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..pos { chars[i] = ('─', Style::default().fg(color)); }
    if pos < width { chars[pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    chars.into_iter().map(|(c, s)| Span::styled(c.to_string(), s)).collect()
}

// Dual bar: primary=● trail=─  secondary=◆, all in `color`
fn dual_bar(primary: f64, secondary: f64, min: f64, max: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    if max <= min || width == 0 { return vec![Span::raw(" ".repeat(width))]; }
    let p_pos = (((primary  - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let s_pos = (((secondary - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..=p_pos { chars[i] = ('─', Style::default().fg(color)); }
    if s_pos < width { chars[s_pos] = ('◆', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    if p_pos < width { chars[p_pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    chars.into_iter().map(|(c, s)| Span::styled(c.to_string(), s)).collect()
}

fn temp_bar(temp: f64, apparent: f64, min: f64, max: f64, width: usize) -> Vec<Span<'static>> {
    dual_bar(temp, apparent, min, max, width, temp_color(temp))
}

fn wind_bar(speed: f64, gust: f64, min: f64, max: f64, width: usize) -> Vec<Span<'static>> {
    dual_bar(speed, gust, min, max, width, wind_color(speed))
}

// ─── Day summary ─────────────────────────────────────────────────────────────

// (label 10 chars, value 7 chars) → 17 chars total, padded to day_w=18.
// Rows 0-1 have empty value (whole text in label).
fn summary_parts(s: &DaySummary) -> Vec<(String, String)> {
    vec![
        (format!("{}", s.date.format("%Y-%m-%d")), String::new()),
        (format!("{}", day_name(s.date)), String::new()),
        (format!("{:<10}", "Temp max:"), format!("{:>5.1}°C", s.max_temp)),
        (format!("{:<10}", "Temp min:"), format!("{:>5.1}°C", s.min_temp)),
        (format!("{:<10}", "Feel max:"), format!("{:>5.1}°C", s.max_apparent)),
        (format!("{:<10}", "Feel min:"), format!("{:>5.1}°C", s.min_apparent)),
        (format!("{:<10}", "Cloud avg:"), format!("{:>6.0}%",  s.avg_cloud)),
        (format!("{:<10}", "Rain prob:"), format!("{:>6.0}%",  s.max_precip_prob)),
        (format!("{:<10}", "Rain sum:"), format!("{:>5.1}mm",  s.total_precip)),
        (format!("{:<10}", "Wind:"),     format!("{:>3.0}km/h",  s.max_wind_speed)),
        (format!("{:<10}", "Gusts:"),    format!("{:>3.0}km/h",  s.max_wind_gust)),
        (format!("{:<10}", "Pressure:"), format!("{:>4.0}hPa", s.avg_pressure)),
        (format!("{:<10}", "Humidity:"), format!("{:>6.0}%",   s.avg_humidity)),
    ]
}

// ─── Banner ───────────────────────────────────────────────────────────────────

fn print_banner(out: &mut impl IoWrite, shadow_rgb: (u8, u8, u8)) -> io::Result<()> {
    // 5-row pixel font, 4 cols per letter, 1=filled (██) 0=empty (  )
    // Shadow is offset +1 pixel to the right only (same row), in indigo.
    // Display area: 5 rows × 5 cols per letter (4 main + 1 shadow col).
    //
    // P         O         G         O         D         A
    let font: &[&[u8]] = &[
        &[0b1110, 0b1001, 0b1110, 0b1000, 0b1000],
        &[0b0110, 0b1001, 0b1001, 0b1001, 0b0110],
        &[0b0111, 0b1000, 0b1011, 0b1001, 0b0111],
        &[0b0110, 0b1001, 0b1001, 0b1001, 0b0110],
        &[0b1110, 0b1001, 0b1001, 0b1001, 0b1110],
        &[0b0110, 0b1001, 0b1111, 0b1001, 0b1001],
    ];

    let pixel = |letter: &[u8], row: usize, col: usize| -> bool {
        row < 5 && col < 4 && (letter[row] >> (3 - col)) & 1 == 1
    };
    let (sr, sg, sb) = shadow_rgb;

    for drow in 0..5usize {
        write!(out, "  ")?;
        for letter in font {
            for dcol in 0..5usize {
                let main   = pixel(letter, drow, dcol);
                let shadow = dcol > 0 && pixel(letter, drow, dcol - 1); // right-only shadow
                if main {
                    write!(out, "\x1b[36m██\x1b[0m")?;
                } else if shadow {
                    write!(out, "\x1b[38;2;{sr};{sg};{sb}m█\x1b[0m ")?;
                } else {
                    write!(out, "  ")?;
                }
            }
            write!(out, "  ")?; // letter spacing
        }
        writeln!(out)?;
    }
    writeln!(out)?;
    Ok(())
}

// ─── Overview area charts ────────────────────────────────────────────────────

fn write_colored(out: &mut impl IoWrite, ch: &str, color: Color) -> io::Result<()> {
    match color {
        Color::Rgb(r, g, b) => write!(out, "\x1b[38;2;{r};{g};{b}m{ch}\x1b[0m"),
        Color::White        => write!(out, "\x1b[1;37m{ch}\x1b[0m"),
        Color::DarkGray     => write!(out, "\x1b[90m{ch}\x1b[0m"),
        Color::Cyan         => write!(out, "\x1b[36m{ch}\x1b[0m"),
        _                   => write!(out, "{ch}"),
    }
}

// Filled area chart (block chars). secondary = optional (values, s_min, s_max) overlay.
fn print_one_chart(
    out: &mut impl IoWrite,
    title: &str,
    primary: &[f64],
    secondary: Option<(&[f64], f64, f64)>,
    p_min: f64, p_max: f64,
    chart_h: usize,
    label_w: usize,
    chart_w: usize,
    term_w: usize,
    fmt: &dyn Fn(f64) -> String,
    p_color: &dyn Fn(f64) -> Color,
    s_color: Color,
) -> io::Result<()> {
    const BLOCKS: &[&str] = &[" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let total_sub = (chart_h * 8) as f64;
    let p_range = (p_max - p_min).max(0.001);

    // Header line
    let hdr = format!("─ {} ", title);
    let pad = term_w.saturating_sub(hdr.chars().count());
    write!(out, "\x1b[90m{}{}\x1b[0m\n", hdr, "─".repeat(pad))?;

    for r in 0..chart_h {
        let label = if r == 0 { fmt(p_max) } else if r == chart_h - 1 { fmt(p_min) } else { String::new() };
        write!(out, "\x1b[90m{:>label_w$}│\x1b[0m", label)?;

        let row_bottom = (chart_h - 1 - r) * 8;
        let row_top    = (chart_h - r) * 8;

        for c in 0..chart_w {
            let n = primary.len().max(1);
            let idx = (c * n / chart_w).min(n - 1);

            let p_val = primary[idx];
            let p_sub = (((p_val - p_min) / p_range).clamp(0.0, 1.0) * total_sub).round() as usize;
            let p_frac = p_sub.saturating_sub(row_bottom).min(8);
            let p_ch = if p_sub >= row_top { "█" } else { BLOCKS[p_frac] };

            let sec_in_row = secondary.and_then(|(sv, s_min, s_max)| {
                let s_val = sv[idx];
                let s_range = (s_max - s_min).max(0.001);
                let s_sub = (((s_val - s_min) / s_range).clamp(0.0, 1.0) * total_sub).round() as usize;
                let s_frac = s_sub.saturating_sub(row_bottom).min(8);
                if s_sub > row_bottom && s_sub <= row_top { Some(s_frac) } else { None }
            });

            if let Some(s_frac) = sec_in_row {
                write_colored(out, BLOCKS[s_frac], s_color)?;
            } else if p_ch != " " {
                write_colored(out, p_ch, p_color(p_val))?;
            } else {
                write!(out, " ")?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}

fn print_overview(out: &mut impl IoWrite, data: &[HourlyData], term_w: usize) -> io::Result<()> {
    if data.is_empty() { return Ok(()); }

    const CHART_H: usize = 4;
    let label_w: usize = 8;
    let chart_w = term_w.saturating_sub(label_w + 1);
    let n = data.len();

    let temps:  Vec<f64> = data.iter().map(|h| h.temp).collect();
    let feels:  Vec<f64> = data.iter().map(|h| h.apparent_temp).collect();
    let clouds: Vec<f64> = data.iter().map(|h| h.cloud).collect();
    let rain_p: Vec<f64> = data.iter().map(|h| h.precip_prob).collect();
    let rain_m: Vec<f64> = data.iter().map(|h| h.precip).collect();
    let winds:  Vec<f64> = data.iter().map(|h| h.wind_speed).collect();
    let gusts:  Vec<f64> = data.iter().map(|h| h.wind_gust).collect();
    let press:  Vec<f64> = data.iter().map(|h| h.pressure).collect();
    let humid:  Vec<f64> = data.iter().map(|h| h.humidity).collect();

    let temp_min  = temps.iter().chain(feels.iter()).cloned().fold(f64::INFINITY, f64::min) - 1.0;
    let temp_max  = temps.iter().chain(feels.iter()).cloned().fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let press_min = press.iter().cloned().fold(f64::INFINITY, f64::min) - 1.0;
    let press_max = press.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let rain_max  = rain_m.iter().cloned().fold(0.0_f64, f64::max).max(0.1);
    let wind_max  = gusts.iter().chain(winds.iter()).cloned().fold(0.0_f64, f64::max) + 1.0;

    print_one_chart(out, "TEMP °C",
        &temps, None, temp_min, temp_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}°C", v), &temp_color, Color::White)?;

    print_one_chart(out, "FEEL °C",
        &feels, None, temp_min, temp_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}°C", v), &temp_color, Color::White)?;

    print_one_chart(out, "CLOUD %",
        &clouds, None, 0.0, 100.0,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|_| Color::DarkGray, Color::White)?;

    print_one_chart(out, "RAIN %",
        &rain_p, None, 0.0, 100.0,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|v| palette(v / 100.0), Color::White)?;

    print_one_chart(out, "RAIN mm",
        &rain_m, None, 0.0, rain_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.1}mm", v), &|v| palette((v / rain_max).clamp(0.0, 1.0)), Color::White)?;

    print_one_chart(out, "WIND km/h",
        &winds, None, 0.0, wind_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}k/h", v), &wind_color, Color::White)?;

    print_one_chart(out, "GUSTS km/h",
        &gusts, None, 0.0, wind_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}k/h", v), &wind_color, Color::White)?;

    print_one_chart(out, "PRESSURE hPa",
        &press, None, press_min, press_max,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}hPa", v), &pressure_color, Color::White)?;

    print_one_chart(out, "HUMIDITY %",
        &humid, None, 0.0, 100.0,
        CHART_H, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|v| palette(v / 100.0), Color::White)?;

    // X-axis: date labels at day boundaries
    write!(out, "\x1b[90m{:>label_w$}┴", "")?;
    let mut x_chars: Vec<char> = vec!['─'; chart_w];
    for (di, hd) in data.iter().enumerate() {
        if hd.time.hour() == 0 {
            let col = di * chart_w / n;
            for (j, ch) in hd.time.format("%a %d").to_string().chars().enumerate() {
                if col + j < chart_w { x_chars[col + j] = ch; }
            }
        }
    }
    writeln!(out, "{}\x1b[0m", x_chars.iter().collect::<String>())?;
    writeln!(out)?;
    Ok(())
}

// ─── Geocoding ────────────────────────────────────────────────────────────────

fn geocode_city(name: &str) -> anyhow::Result<(f64, f64, String, String)> {
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

fn reverse_geocode(lat: f64, lng: f64) -> anyhow::Result<(String, String)> {
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

fn parse_days(s: Option<&String>) -> u32 {
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

// ─── ANSI output ─────────────────────────────────────────────────────────────

fn emit_span(out: &mut impl IoWrite, span: &Span) -> io::Result<()> {
    let style = span.style;
    let has_style = style.fg.is_some() || !style.add_modifier.is_empty();
    if style.add_modifier.contains(Modifier::BOLD) { write!(out, "\x1b[1m")?; }
    match style.fg {
        Some(Color::Rgb(r, g, b)) => write!(out, "\x1b[38;2;{r};{g};{b}m")?,
        Some(Color::White)        => write!(out, "\x1b[37m")?,
        Some(Color::DarkGray)     => write!(out, "\x1b[90m")?,
        Some(Color::Cyan)         => write!(out, "\x1b[36m")?,
        Some(Color::Blue)         => write!(out, "\x1b[34m")?,
        _ => {}
    }
    write!(out, "{}", span.content)?;
    if has_style { write!(out, "\x1b[0m")?; }
    Ok(())
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn print_usage() {
    eprintln!("Pogoda — hourly weather forecast\n");
    eprintln!("Usage:");
    eprintln!("  pogoda <latitude> <longitude> [days]");
    eprintln!("  pogoda <lat,lng> [days]");
    eprintln!("  pogoda <city> [days]\n");
    eprintln!("  days  Forecast days 1–16 (default: 7)\n");
    eprintln!("Examples:");
    eprintln!("  pogoda 52.52 13.41");
    eprintln!("  pogoda 51.10,17.00 14");
    eprintln!("  pogoda Wrocław");
    eprintln!("  pogoda Berlin 10");
    eprintln!("  pogoda New York 7");
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    // Parse location and days from arguments.
    // Supported forms: "<lat> <lng> [days]" | "<lat,lng> [days]" | "<city...> [days]"
    let (lat, lng, days, location) = {
        let first = &args[1];
        if let Some(comma_pos) = first.find(',') {
            // "lat,lng" comma format
            let lat: f64 = first[..comma_pos].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid latitude in '{}'.", first); std::process::exit(1);
            });
            let lng: f64 = first[comma_pos+1..].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid longitude in '{}'.", first); std::process::exit(1);
            });
            let days = parse_days(args.get(2));
            let loc = reverse_geocode(lat, lng).ok();
            (lat, lng, days, loc)
        } else if let Ok(lat) = first.parse::<f64>() {
            // "<lat> <lng>" numeric format
            if args.len() < 3 { print_usage(); std::process::exit(1); }
            let lng: f64 = args[2].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid longitude '{}'.", args[2]); std::process::exit(1);
            });
            let days = parse_days(args.get(3));
            let loc = reverse_geocode(lat, lng).ok();
            (lat, lng, days, loc)
        } else {
            // City name — optional last arg is days if it parses as a number
            let (city_parts, days) = if args.len() > 2 {
                if let Ok(d) = args.last().unwrap().parse::<u32>() {
                    if d < 1 || d > 16 {
                        eprintln!("Error: days must be between 1 and 16.");
                        std::process::exit(1);
                    }
                    (&args[1..args.len()-1], d)
                } else {
                    (&args[1..], 7u32)
                }
            } else {
                (&args[1..], 7u32)
            };
            let city_name = city_parts.join(" ");
            match geocode_city(&city_name) {
                Ok((lat, lng, city, country)) => (lat, lng, days, Some((city, country))),
                Err(e) => { eprintln!("Error: {}", e); std::process::exit(1); }
            }
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let term_w = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(120);

    let (api_url, data) = fetch_weather(lat, lng, days)?;

    let indigo = match palette(1.0) { Color::Rgb(r, g, b) => (r, g, b), _ => (90, 0, 170) };
    writeln!(out)?;
    print_banner(&mut out, indigo)?;

    let forecast_date = data.first().map(|h| h.time.date()).unwrap_or_default();
    let lat_str = if lat >= 0.0 { format!("{:.2}°N", lat) } else { format!("{:.2}°S", lat.abs()) };
    let lng_str = if lng >= 0.0 { format!("{:.2}°E", lng) } else { format!("{:.2}°W", lng.abs()) };
    let days_str = if days == 1 { "1 day".to_string() } else { format!("{} days", days) };
    let date_str = format!("{} {}, {}", forecast_date.format("%B"), forecast_date.day(), forecast_date.year());
    let loc_prefix = match &location {
        Some((city, country)) if !country.is_empty() => format!("{}, {}  ·  ", city, country),
        Some((city, _)) => format!("{}  ·  ", city),
        None => String::new(),
    };
    writeln!(out, "Location: {}{}, {}  ·  {}  ·  {}\n",
        loc_prefix, lat_str, lng_str, days_str, date_str)?;

    print_overview(&mut out, &data, term_w)?;

    let mut dates: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
    dates.dedup();

    let summaries: Vec<DaySummary> = dates.iter().map(|d| day_summary(&data, *d)).collect();

    let temp_min = data.iter().map(|h| h.apparent_temp.min(h.temp)).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|h| h.apparent_temp.max(h.temp)).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let pressure_min = data.iter().map(|h| h.pressure).fold(f64::INFINITY, f64::min) - 2.0;
    let pressure_max = data.iter().map(|h| h.pressure).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let wind_max = data.iter().map(|h| h.wind_gust.max(h.wind_speed)).fold(0.0_f64, f64::max) + 2.0;

    // ── Column layout ────────────────────────────────────────────────────────
    // Each chart column: (header, label_w, default_bar_w)
    // label_w = exact char count of the value format string.
    let day_w:  usize = 18;
    let hour_w: usize = 6;
    const COL_DEFS: &[(&str, usize, usize)] = &[
        ("TEMP/FEEL °C", 11, 9),
        ("CLOUD %",       3, 10),
        ("RAIN %",        3, 10),
        ("RAIN mm",       4,  8),
        ("WIND km/h",    11,  9),
        ("PRESSURE hPa",  6,  8),
        ("HUMIDITY %",    3, 10),
    ];
    const MIN_BAR: usize = 3;

    // Drop rightmost columns until everything fits with MIN_BAR per bar
    let mut n_cols = COL_DEFS.len();
    loop {
        let needed: usize = day_w + hour_w
            + COL_DEFS[..n_cols].iter().map(|(_, lw, _)| 1 + lw + 1 + MIN_BAR).sum::<usize>();
        if needed <= term_w || n_cols == 1 { break; }
        n_cols -= 1;
    }

    // Distribute available bar space proportionally among active columns
    let active = &COL_DEFS[..n_cols];
    let fixed: usize = day_w + hour_w
        + active.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = active.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = active.iter().map(|(_, _, bw)| {
        ((bw * available) / default_total.max(1)).max(MIN_BAR)
    }).collect();
    // Give any leftover chars to the last bar so total == term_w
    let used: usize = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w { bar_ws[n_cols - 1] += term_w - used; }

    let sep_w: usize = fixed + bar_ws.iter().sum::<usize>();

    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    // Header: titles span the full column (label + sep + bar)
    let hdr_col = |lw: usize, bw: usize, title: &str| -> String {
        format!(" {:<width$}", title, width = lw + 1 + bw)
    };
    let mut hdr_spans = vec![
        Span::styled(format!("{:<day_w$}", "DAY SUMMARY"), hdr),
        Span::raw(format!("{:hour_w$}", "")),
    ];
    for (i, (title, lw, _)) in active.iter().enumerate() {
        hdr_spans.push(Span::styled(hdr_col(*lw, bar_ws[i], title), hdr));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));

    let mut day_summary_idx: usize;
    let mut current_date: Option<chrono::NaiveDate> = None;
    let mut day_row_count = 0usize;
    let mut day_parts_cache: Vec<(String, String)> = Vec::new();

    for hd in &data {
        let hour = hd.time.hour();
        let date = hd.time.date();

        if current_date != Some(date) {
            if current_date.is_some() {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));
            }
            current_date = Some(date);
            day_row_count = 0;
            day_summary_idx = dates.iter().position(|d| *d == date).unwrap_or(0);
            day_parts_cache = summary_parts(&summaries[day_summary_idx]);
        }

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut spans: Vec<Span> = if day_row_count < day_parts_cache.len() {
            let (label, value) = &day_parts_cache[day_row_count];
            if day_row_count == 1 {
                vec![Span::styled(format!("{:<day_w$}", label), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]
            } else if value.is_empty() {
                vec![Span::styled(format!("{:<day_w$}", label), bold)]
            } else {
                vec![Span::raw(label.clone()), Span::styled(value.clone(), bold), Span::raw(" ")]
            }
        } else {
            vec![Span::raw(format!("{:<day_w$}", ""))]
        };
        day_row_count += 1;

        spans.push(Span::styled(format!("{:02}:00 ", hour), dim));

        for (i, _) in active.iter().enumerate() {
            let bw = bar_ws[i];
            let (label, bar): (Span, Vec<Span>) = match i {
                0 => { // TEMP/FEEL
                    let c = temp_color(hd.temp);
                    (Span::styled(format!("{:>5.1}/{:>5.1}", hd.temp, hd.apparent_temp), Style::default().fg(c)),
                     temp_bar(hd.temp, hd.apparent_temp, temp_min, temp_max, bw))
                }
                1 => { // CLOUD
                    let c = cloud_color(hd.cloud);
                    (Span::styled(format!("{:>3.0}", hd.cloud), Style::default().fg(c)),
                     value_bar(hd.cloud, 0.0, 100.0, bw, c))
                }
                2 => { // RAIN %
                    let c = palette(hd.precip_prob / 100.0);
                    (Span::styled(format!("{:>3.0}", hd.precip_prob), Style::default().fg(c)),
                     value_bar(hd.precip_prob, 0.0, 100.0, bw, c))
                }
                3 => { // RAIN mm
                    let c = palette((hd.precip / 10.0).clamp(0.0, 1.0));
                    (Span::styled(format!("{:>4.1}", hd.precip), Style::default().fg(c)),
                     value_bar(hd.precip, 0.0, 10.0, bw, c))
                }
                4 => { // WIND
                    let c = wind_color(hd.wind_speed);
                    (Span::styled(format!("{:>5.1}/{:>5.1}", hd.wind_speed, hd.wind_gust), Style::default().fg(c)),
                     wind_bar(hd.wind_speed, hd.wind_gust, 0.0, wind_max, bw))
                }
                5 => { // PRESSURE
                    let c = pressure_color(hd.pressure);
                    (Span::styled(format!("{:>6.0}", hd.pressure), Style::default().fg(c)),
                     value_bar(hd.pressure, pressure_min, pressure_max, bw, c))
                }
                _ => { // HUMIDITY
                    let c = palette(hd.humidity / 100.0);
                    (Span::styled(format!("{:>3.0}", hd.humidity), Style::default().fg(c)),
                     value_bar(hd.humidity, 0.0, 100.0, bw, c))
                }
            };
            spans.push(Span::raw(" "));
            spans.push(label);
            spans.push(Span::raw(" "));
            spans.extend(bar);
        }

        lines.push(Line::from(spans));
    }

    writeln!(out)?;
    for line in &lines {
        for span in &line.spans {
            emit_span(&mut out, span)?;
        }
        writeln!(out)?;
    }

    // ── Footer ───────────────────────────────────────────────────────────────
    writeln!(out)?;
    write!(out, "\x1b[90m")?;
    writeln!(out, "Data source: Open-Meteo (open-meteo.com) — free, open-source weather API")?;
    writeln!(out, "API URL:     {api_url}")?;
    write!(out, "\x1b[0m")?;

    Ok(())
}

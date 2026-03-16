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
    eprintln!("Usage: pogoda <latitude> <longitude> [days]\n");
    eprintln!("  latitude   Decimal latitude   (e.g. 52.52 for Berlin)");
    eprintln!("  longitude  Decimal longitude  (e.g. 13.41 for Berlin)");
    eprintln!("  days       Forecast days 1–16 (default: 7)\n");
    eprintln!("Examples:");
    eprintln!("  pogoda 52.52 13.41");
    eprintln!("  pogoda 48.85 2.35 14     # Paris, 14 days");
    eprintln!("  pogoda 40.71 -74.01 3    # New York, 3 days");
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        print_usage();
        std::process::exit(1);
    }
    let lat: f64 = args[1].parse().unwrap_or_else(|_| { print_usage(); std::process::exit(1); });
    let lng: f64 = args[2].parse().unwrap_or_else(|_| { print_usage(); std::process::exit(1); });
    let days: u32 = if args.len() >= 4 {
        let d: u32 = args[3].parse().unwrap_or_else(|_| { print_usage(); std::process::exit(1); });
        if d < 1 || d > 16 {
            eprintln!("Error: days must be between 1 and 16.");
            std::process::exit(1);
        }
        d
    } else {
        7
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Indigo = palette(1.0) = oklch(0.62, 0.14, 280°)
    let indigo = match palette(1.0) { Color::Rgb(r, g, b) => (r, g, b), _ => (90, 0, 170) };
    print_banner(&mut out, indigo)?;

    eprintln!("Fetching weather data...");
    let (api_url, data) = fetch_weather(lat, lng, days)?;

    let mut dates: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
    dates.dedup();

    let summaries: Vec<DaySummary> = dates.iter().map(|d| day_summary(&data, *d)).collect();

    let temp_min = data.iter().map(|h| h.apparent_temp.min(h.temp)).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|h| h.apparent_temp.max(h.temp)).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let pressure_min = data.iter().map(|h| h.pressure).fold(f64::INFINITY, f64::min) - 2.0;
    let pressure_max = data.iter().map(|h| h.pressure).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let wind_max = data.iter().map(|h| h.wind_gust.max(h.wind_speed)).fold(0.0_f64, f64::max) + 2.0;

    // ── Column layout ────────────────────────────────────────────────────────
    // Label widths = exact char count of the formatted string (no unit suffix in value):
    //   temp/wind: "{:>5.1}/{:>5.1}" = 11 chars
    //   pct:       "{:>3.0}"         =  3 chars
    //   precip:    "{:>4.1}"         =  4 chars
    //   press:     "{:>6.0}"         =  6 chars
    let day_w:     usize = 18;
    let hour_w:    usize = 6;
    let temp_lw:   usize = 11;
    let temp_bw:   usize = 9;
    let pct_lw:    usize = 3;
    let pct_bw:    usize = 10;
    let precip_lw: usize = 4;
    let precip_bw: usize = 8;
    let wind_lw:   usize = 11;
    let wind_bw:   usize = 9;
    let press_lw:  usize = 6;
    let press_bw:  usize = 8;

    let temp_inner   = temp_lw   + 1 + temp_bw;
    let pct_inner    = pct_lw    + 1 + pct_bw;
    let precip_inner = precip_lw + 1 + precip_bw;
    let wind_inner   = wind_lw   + 1 + wind_bw;
    let press_inner  = press_lw  + 1 + press_bw;

    let sep_w = day_w + hour_w
        + 1 + temp_inner
        + 1 + pct_inner
        + 1 + pct_inner
        + 1 + precip_inner
        + 1 + wind_inner
        + 1 + press_inner
        + 1 + pct_inner;

    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    // Header titles span full column (label + sep + bar)
    let hdr_col = |lw: usize, bw: usize, title: &str| -> String {
        format!(" {:<width$}", title, width = lw + 1 + bw)
    };
    lines.push(Line::from(vec![
        Span::styled(format!("{:<day_w$}", "DAY SUMMARY"),  hdr),
        Span::raw(format!("{:hour_w$}", "")),
        Span::styled(hdr_col(temp_lw,   temp_bw,   "TEMP/FEEL °C"),  hdr),
        Span::styled(hdr_col(pct_lw,    pct_bw,    "CLOUD %"),        hdr),
        Span::styled(hdr_col(pct_lw,    pct_bw,    "RAIN %"),         hdr),
        Span::styled(hdr_col(precip_lw, precip_bw, "RAIN mm"),        hdr),
        Span::styled(hdr_col(wind_lw,   wind_bw,   "WIND km/h"),      hdr),
        Span::styled(hdr_col(press_lw,  press_bw,  "PRESSURE hPa"),   hdr),
        Span::styled(hdr_col(pct_lw,    pct_bw,    "HUMIDITY %"),     hdr),
    ]));
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

        // ── Temperature ──────────────────────────────────────────────────────
        let tc = temp_color(hd.temp);
        let temp_label = Span::styled(format!("{:>5.1}/{:>5.1}", hd.temp, hd.apparent_temp), Style::default().fg(tc));
        let temp_spans = temp_bar(hd.temp, hd.apparent_temp, temp_min, temp_max, temp_bw);

        // ── Cloud cover ───────────────────────────────────────────────────────
        let cc = cloud_color(hd.cloud);
        let cloud_label = Span::styled(format!("{:>3.0}", hd.cloud), Style::default().fg(cc));
        let cloud_spans = value_bar(hd.cloud, 0.0, 100.0, pct_bw, cc);

        // ── Rain probability ──────────────────────────────────────────────────
        let pc = palette(hd.precip_prob / 100.0);
        let prob_label = Span::styled(format!("{:>3.0}", hd.precip_prob), Style::default().fg(pc));
        let prob_spans = value_bar(hd.precip_prob, 0.0, 100.0, pct_bw, pc);

        // ── Precipitation mm ──────────────────────────────────────────────────
        let rc = palette((hd.precip / 10.0).clamp(0.0, 1.0));
        let precip_label = Span::styled(format!("{:>4.1}", hd.precip), Style::default().fg(rc));
        let precip_spans = value_bar(hd.precip, 0.0, 10.0, precip_bw, rc);

        // ── Wind speed / gusts ────────────────────────────────────────────────
        let wc = wind_color(hd.wind_speed);
        let wind_label = Span::styled(format!("{:>5.1}/{:>5.1}", hd.wind_speed, hd.wind_gust), Style::default().fg(wc));
        let wind_spans = wind_bar(hd.wind_speed, hd.wind_gust, 0.0, wind_max, wind_bw);

        // ── Pressure ──────────────────────────────────────────────────────────
        let prc = pressure_color(hd.pressure);
        let press_label = Span::styled(format!("{:>6.0}", hd.pressure), Style::default().fg(prc));
        let press_spans = value_bar(hd.pressure, pressure_min, pressure_max, press_bw, prc);

        // ── Humidity ──────────────────────────────────────────────────────────
        let hc = palette(hd.humidity / 100.0);
        let humid_label = Span::styled(format!("{:>3.0}", hd.humidity), Style::default().fg(hc));
        let humid_spans = value_bar(hd.humidity, 0.0, 100.0, pct_bw, hc);

        spans.push(Span::raw(" ")); spans.push(temp_label);   spans.push(Span::raw(" ")); spans.extend(temp_spans);
        spans.push(Span::raw(" ")); spans.push(cloud_label);  spans.push(Span::raw(" ")); spans.extend(cloud_spans);
        spans.push(Span::raw(" ")); spans.push(prob_label);   spans.push(Span::raw(" ")); spans.extend(prob_spans);
        spans.push(Span::raw(" ")); spans.push(precip_label); spans.push(Span::raw(" ")); spans.extend(precip_spans);
        spans.push(Span::raw(" ")); spans.push(wind_label);   spans.push(Span::raw(" ")); spans.extend(wind_spans);
        spans.push(Span::raw(" ")); spans.push(press_label);  spans.push(Span::raw(" ")); spans.extend(press_spans);
        spans.push(Span::raw(" ")); spans.push(humid_label);  spans.push(Span::raw(" ")); spans.extend(humid_spans);

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

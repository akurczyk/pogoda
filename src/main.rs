mod colors;
mod geo;
mod render;
mod types;
mod units;
mod weather;

use chrono::Datelike;
use ratatui::style::Color;
use std::io::{self, Write as IoWrite};

use colors::palette;
use geo::{geocode_city, parse_days, reverse_geocode};
use render::{banner::print_banner, charts::print_overview, table::print_table};
use types::{Theme, VERSION};
use weather::fetch_weather;

fn print_usage() {
    eprintln!("Pogoda - Terminal Weather Forecast  v{VERSION}\n");
    eprintln!("Usage:");
    eprintln!("  pogoda <latitude> <longitude> [days]");
    eprintln!("  pogoda <lat,lng> [days]");
    eprintln!("  pogoda <city> [days]\n");
    eprintln!("  days  Forecast days 1–16 (default: 7)\n");
    eprintln!("Modifiers:");
    eprintln!("  --strange-units    American units: °F, mph, in, inHg");
    eprintln!("  --i-am-not-blue    Warm color palette: indigo → red → orange");
    eprintln!("  --i-am-blue        Cool color palette: cyan → blue → indigo");
    eprintln!("  (Cool blue palette is used by default)\n");
    eprintln!("Examples:");
    eprintln!("  pogoda 52.52 13.41");
    eprintln!("  pogoda 51.10,17.00 14");
    eprintln!("  pogoda Wrocław");
    eprintln!("  pogoda Berlin 10 --strange-units");
    eprintln!("  pogoda New York 7 --i-am-not-blue");
}

fn main() -> anyhow::Result<()> {
    let raw_args: Vec<String> = std::env::args().collect();
    let imperial  = raw_args.iter().any(|a| a == "--strange-units");
    let want_warm = raw_args.iter().any(|a| a == "--i-am-not-blue");
    let args: Vec<String> = raw_args.into_iter()
        .filter(|a| a != "--strange-units" && a != "--i-am-not-blue" && a != "--i-am-blue")
        .collect();

    let theme = if want_warm { Theme::Warm } else { Theme::Blue };
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let (lat, lng, days, location) = {
        let first = &args[1];
        if let Some(comma_pos) = first.find(',') {
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
            if args.len() < 3 { print_usage(); std::process::exit(1); }
            let lng: f64 = args[2].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid longitude '{}'.", args[2]); std::process::exit(1);
            });
            let days = parse_days(args.get(3));
            let loc = reverse_geocode(lat, lng).ok();
            (lat, lng, days, loc)
        } else {
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

    let banner_main   = match palette(0.0, theme) { Color::Rgb(r, g, b) => (r, g, b), _ => (0, 188, 212) };
    let banner_shadow = match palette(1.0, theme) { Color::Rgb(r, g, b) => (r, g, b), _ => (90, 0, 170) };
    writeln!(out)?;
    print_banner(&mut out, banner_main, banner_shadow)?;

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

    print_overview(&mut out, &data, term_w, imperial, theme)?;

    let mut dates: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
    dates.dedup();
    let summaries: Vec<_> = dates.iter().map(|d| weather::day_summary(&data, *d)).collect();

    print_table(&mut out, &data, &dates, &summaries, term_w, imperial, theme)?;

    writeln!(out)?;
    write!(out, "\x1b[90m")?;
    writeln!(out, "Data source: Open-Meteo (open-meteo.com) — free, open-source weather API")?;
    writeln!(out, "API URL:     {api_url}  ·  v{VERSION}")?;
    writeln!(out)?;
    writeln!(out, "Modifiers: --strange-units  --i-am-not-blue  --i-am-blue")?;
    writeln!(out)?;
    writeln!(out, "https://github.com/akurczyk/pogoda")?;
    write!(out, "\x1b[0m")?;

    Ok(())
}

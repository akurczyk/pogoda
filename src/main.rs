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
use types::{Theme, Units, VERSION};
use units::{c_to_f, hpa_to_inhg, kmh_to_mph, mm_to_in};
use weather::fetch_weather;

fn print_usage() {
    eprintln!("Pogoda - Terminal Weather Forecast  v{VERSION}\n");
    eprintln!("Usage:");
    eprintln!("  pogoda <latitude> <longitude> [days]");
    eprintln!("  pogoda <lat,lng> [days]");
    eprintln!("  pogoda <city> [days]\n");
    eprintln!("  days  Forecast days 1–16 (default: 7)\n");
    eprintln!("Modifiers:");
    eprintln!("  --strange-units      American units: °F, mph, in, inHg");
    eprintln!("  --yes-sir            British units: °C, mph, mm, hPa");
    eprintln!("  --i-am-blue          Cool color palette: cyan → blue → indigo");
    eprintln!("  --color-me           Full palette: cyan → blue → indigo → red → orange");
    eprintln!("  --i-cant-afford-cga  Monochromatic output (no colors)");
    eprintln!("  --high-charts        Taller charts (24 rows)");
    eprintln!("  --no-charts          Skip the overview charts");
    eprintln!("  --no-table           Skip the hourly table");
    eprintln!("  --tabular-bells      Output CSV data instead of charts/table");
    eprintln!("  --no-eyecandy        Skip logo, location header and footer\n");
    eprintln!("  (Warm palette indigo → red → orange is used by default)\n");
    eprintln!("Examples:");
    eprintln!("  pogoda 52.52 13.41");
    eprintln!("  pogoda 51.10,17.00 14");
    eprintln!("  pogoda Wrocław");
    eprintln!("  pogoda Berlin 10 --strange-units");
    eprintln!("  pogoda London 7 --yes-sir");
    eprintln!("  pogoda New York 7 --i-am-blue\n");
    eprintln!("https://github.com/akurczyk/pogoda  v{VERSION}");
}

fn main() -> anyhow::Result<()> {
    let raw_args: Vec<String> = std::env::args().collect();

    let imperial      = raw_args.iter().any(|a| a == "--strange-units");
    let british       = raw_args.iter().any(|a| a == "--yes-sir");
    let want_blue     = raw_args.iter().any(|a| a == "--i-am-blue");
    let want_rainbow  = raw_args.iter().any(|a| a == "--color-me");
    let high_charts   = raw_args.iter().any(|a| a == "--high-charts");
    let no_charts     = raw_args.iter().any(|a| a == "--no-charts");
    let no_table      = raw_args.iter().any(|a| a == "--no-table");
    let tabular_bells = raw_args.iter().any(|a| a == "--tabular-bells");
    let mono          = raw_args.iter().any(|a| a == "--i-cant-afford-cga");
    let no_eyecandy   = raw_args.iter().any(|a| a == "--no-eyecandy");

    let units = if imperial { Units::Imperial } else if british { Units::British } else { Units::Metric };
    let theme = if want_blue { Theme::Blue } else if want_rainbow { Theme::Rainbow } else { Theme::Warm };
    let chart_h: usize = if high_charts { 24 } else { 4 };

    let args: Vec<String> = raw_args.into_iter()
        .filter(|a| !matches!(a.as_str(),
            "--strange-units" | "--yes-sir" | "--i-am-blue" |
            "--high-charts" | "--no-charts" | "--no-table" | "--tabular-bells" |
            "--i-cant-afford-cga" | "--no-eyecandy" | "--color-me"))
        .collect();

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
    let (api_url, data, solar) = fetch_weather(lat, lng, days)?;

    // CSV mode: skip all visual output
    if tabular_bells {
        let temp_unit = if units.use_fahrenheit() { "F"   } else { "C"    };
        let wind_unit = if units.use_mph()         { "mph" } else { "km/h" };
        let rain_unit = if units.use_inches()       { "in"  } else { "mm"   };
        let pres_unit = if units.use_inhg()         { "inHg"} else { "hPa"  };
        writeln!(out,
            "time,temp_{},feel_{},cloud_pct,precip_prob_pct,precip_{},wind_{},gust_{},pressure_{},humidity_pct",
            temp_unit, temp_unit, rain_unit, wind_unit, wind_unit, pres_unit)?;
        for h in &data {
            let temp     = if units.use_fahrenheit() { c_to_f(h.temp)           } else { h.temp           };
            let feel     = if units.use_fahrenheit() { c_to_f(h.apparent_temp)  } else { h.apparent_temp  };
            let precip   = if units.use_inches()      { mm_to_in(h.precip)       } else { h.precip         };
            let wind     = if units.use_mph()          { kmh_to_mph(h.wind_speed) } else { h.wind_speed     };
            let gust     = if units.use_mph()          { kmh_to_mph(h.wind_gust)  } else { h.wind_gust      };
            let pressure = if units.use_inhg()         { hpa_to_inhg(h.pressure)  } else { h.pressure       };
            writeln!(out, "{},{:.1},{:.1},{:.0},{:.0},{:.2},{:.1},{:.1},{:.2},{:.0}",
                h.time.format("%Y-%m-%dT%H:%M"),
                temp, feel, h.cloud, h.precip_prob, precip, wind, gust, pressure, h.humidity)?;
        }
        return Ok(());
    }

    if !no_eyecandy {
        let banner_main   = match palette(0.0, theme) { Color::Rgb(r, g, b) => (r, g, b), _ => (0, 188, 212) };
        let banner_shadow = match palette(1.0, theme) { Color::Rgb(r, g, b) => (r, g, b), _ => (90, 0, 170) };
        writeln!(out)?;
        print_banner(&mut out, banner_main, banner_shadow, mono)?;

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
    }

    if !no_charts {
        print_overview(&mut out, &data, term_w, units, theme, chart_h, mono)?;
    }

    let mut dates: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
    dates.dedup();
    let summaries: Vec<_> = dates.iter().map(|d| weather::day_summary(&data, &solar, *d)).collect();

    if !no_table {
        print_table(&mut out, &data, &dates, &summaries, term_w, units, theme, mono)?;
    }

    if !no_eyecandy {
        let dim   = if mono { "" } else { "\x1b[90m" };
        let reset = if mono { "" } else { "\x1b[0m" };
        writeln!(out)?;
        write!(out, "{dim}")?;
        writeln!(out, "Data source: Open-Meteo (open-meteo.com) — free, open-source weather API")?;
        writeln!(out, "API URL:     {api_url}")?;
        writeln!(out)?;
        writeln!(out, "Modifiers: --strange-units  --yes-sir  --i-am-blue  --color-me  --i-cant-afford-cga  --no-eyecandy")?;
        writeln!(out, "           --high-charts  --no-charts  --no-table  --tabular-bells")?;
        writeln!(out)?;
        writeln!(out, "https://github.com/akurczyk/pogoda  v{VERSION}")?;
        write!(out, "{reset}")?;
    }

    Ok(())
}

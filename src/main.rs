mod colors;
mod geo;
mod locale;
mod render;
mod types;
mod units;
mod weather;

rust_i18n::i18n!("locales", fallback = "en");

use ratatui::style::Color;
use rust_i18n::t;
use std::io::{self, Write as IoWrite};

use colors::palette;
use geo::{geocode_city, looks_like_days, parse_days, parse_days_str, reverse_geocode};
use render::historical::{
    print_historical_daily_charts, print_historical_daily_table, print_historical_monthly_charts,
    print_historical_monthly_table, write_hist_footer,
};
use render::{banner::print_banner, charts::print_overview, table::print_table};
use types::{Theme, Units, VERSION};
use units::{c_to_f, hpa_to_inhg, kmh_to_mph, mm_to_in};
use weather::{
    aggregate_monthly, fetch_drone_weather, fetch_historical_daily, fetch_historical_hourly,
    fetch_weather,
};

fn print_usage() {
    eprintln!("{}\n", t!("usage.header", version = VERSION));
    eprintln!("{}:", t!("usage.usage_title"));
    eprintln!("  {}", t!("usage.usage_lat_lng"));
    eprintln!("  {}", t!("usage.usage_latlng"));
    eprintln!("  {}\n", t!("usage.usage_city"));
    eprintln!("  {}\n", t!("usage.days_desc"));
    eprintln!("{}:", t!("usage.modifiers_title"));
    eprintln!("  {}", t!("usage.flag_drone"));
    eprintln!("  {}", t!("usage.flag_delorean"));
    eprintln!("  {}", t!("usage.flag_units"));
    eprintln!("  {}", t!("usage.flag_lang"));
    eprintln!("  {}", t!("usage.flag_blue"));
    eprintln!("  {}", t!("usage.flag_color_me"));
    eprintln!("  {}", t!("usage.flag_classic"));
    eprintln!("  {}", t!("usage.flag_rainforest"));
    eprintln!("  {}", t!("usage.flag_mono"));
    eprintln!("  {}", t!("usage.flag_high_charts"));
    eprintln!("  {}", t!("usage.flag_no_charts"));
    eprintln!("  {}", t!("usage.flag_no_table"));
    eprintln!("  {}", t!("usage.flag_tabular"));
    eprintln!("  {}\n", t!("usage.flag_no_eyecandy"));
    eprintln!("  {}\n", t!("usage.default_palette"));
    eprintln!("{}:", t!("usage.examples_title"));
    eprintln!("  {}", t!("usage.ex1"));
    eprintln!("  {}", t!("usage.ex2"));
    eprintln!("  {}", t!("usage.ex3"));
    eprintln!("  {}", t!("usage.ex4"));
    eprintln!("  {}", t!("usage.ex5"));
    eprintln!("  {}", t!("usage.ex6"));
    eprintln!("  {}", t!("usage.ex7"));
    eprintln!("  {}", t!("usage.ex8"));
    eprintln!("  {}\n", t!("usage.ex9"));
    eprintln!("{}", t!("footer.github"));
}

fn main() -> anyhow::Result<()> {
    let raw_args: Vec<String> = std::env::args().collect();

    // ── Value-taking flags ────────────────────────────────────────────────────
    let cli_units: Option<Units> = raw_args
        .iter()
        .position(|a| a == "--units")
        .and_then(|pos| raw_args.get(pos + 1))
        .and_then(|s| match s.as_str() {
            "imperial" => Some(Units::Imperial),
            "british" => Some(Units::British),
            "metric" => Some(Units::Metric),
            _ => None,
        });

    let cli_lang: Option<&'static str> = raw_args
        .iter()
        .position(|a| a == "--lang")
        .and_then(|pos| raw_args.get(pos + 1))
        .and_then(|s| match s.as_str() {
            "en" => Some("en"),
            "ca" => Some("ca"),
            "cs" => Some("cs"),
            "da" => Some("da"),
            "de" => Some("de"),
            "el" => Some("el"),
            "es-es" => Some("es-es"),
            "es-419" => Some("es-419"),
            "fi" => Some("fi"),
            "fr-fr" => Some("fr-fr"),
            "fr-ca" => Some("fr-ca"),
            "hr" => Some("hr"),
            "hu" => Some("hu"),
            "it" => Some("it"),
            "nb" => Some("nb"),
            "nl" => Some("nl"),
            "pl" => Some("pl"),
            "pt-br" => Some("pt-br"),
            "pt-pt" => Some("pt-pt"),
            "ro" => Some("ro"),
            "ru" => Some("ru"),
            "sk" => Some("sk"),
            "sv" => Some("sv"),
            "tr" => Some("tr"),
            "uk" => Some("uk"),
            _ => None,
        });

    // ── Locale auto-detection ─────────────────────────────────────────────────
    let (auto_units, auto_lang) = locale::detect();
    let units = cli_units.or(auto_units).unwrap_or(Units::Metric);
    let lang = cli_lang.or(auto_lang).unwrap_or("en");
    rust_i18n::set_locale(lang);

    // ── Boolean flags ─────────────────────────────────────────────────────────
    let want_blue = raw_args.iter().any(|a| a == "--i-am-blue");
    let want_rainbow = raw_args.iter().any(|a| a == "--color-me");
    let want_classic = raw_args.iter().any(|a| a == "--classic-colors");
    let want_rainforest = raw_args.iter().any(|a| a == "--rainforest");
    let high_charts = raw_args.iter().any(|a| a == "--high-charts");
    let no_charts = raw_args.iter().any(|a| a == "--no-charts");
    let no_table = raw_args.iter().any(|a| a == "--no-table");
    let tabular_bells = raw_args.iter().any(|a| a == "--tabular-bells");
    let mono = raw_args.iter().any(|a| a == "--i-cant-afford-cga");
    let no_eyecandy = raw_args.iter().any(|a| a == "--no-eyecandy");
    let drone = raw_args.iter().any(|a| a == "--i-drone-you");

    // --delorean DD.MM.YYYY DD.MM.YYYY — fail loudly on missing or unparseable dates so the
    // bad args never silently leak into the positional (city/lat/lng) parsing.
    let delorean_dates: Option<(chrono::NaiveDate, chrono::NaiveDate)> =
        if let Some(pos) = raw_args.iter().position(|a| a == "--delorean") {
            let parse = |s: &str| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y");
            let s = raw_args.get(pos + 1).map(String::as_str).unwrap_or("");
            let e = raw_args.get(pos + 2).map(String::as_str).unwrap_or("");
            match (parse(s), parse(e)) {
                (Ok(start), Ok(end)) => Some((start, end)),
                _ => {
                    eprintln!("{}", t!("errors.delorean_invalid_dates"));
                    std::process::exit(1);
                }
            }
        } else {
            None
        };

    let theme = if want_blue {
        Theme::Blue
    } else if want_rainbow {
        Theme::Rainbow
    } else if want_classic {
        Theme::Classic
    } else if want_rainforest {
        Theme::Rainforest
    } else {
        Theme::Warm
    };
    let chart_h: usize = if high_charts { 24 } else { 4 };

    // Strip all known flags (and their value arguments) from positional args.
    let args: Vec<String> = {
        let mut skip = 0i32;
        let mut out: Vec<String> = Vec::new();
        for a in raw_args.iter() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            // Flags that consume following value arguments
            if a == "--units" || a == "--lang" {
                skip = 1;
                continue;
            }
            if a == "--delorean" {
                skip = 2;
                continue;
            }
            if matches!(
                a.as_str(),
                "--i-am-blue"
                    | "--color-me"
                    | "--classic-colors"
                    | "--rainforest"
                    | "--high-charts"
                    | "--no-charts"
                    | "--no-table"
                    | "--tabular-bells"
                    | "--i-cant-afford-cga"
                    | "--no-eyecandy"
                    | "--i-drone-you"
            ) {
                continue;
            }
            out.push(a.clone());
        }
        out
    };

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let (lat, lng, days, location) = {
        let first = &args[1];
        if let Some(comma_pos) = first.find(',') {
            let lat: f64 = first[..comma_pos].parse().unwrap_or_else(|_| {
                eprintln!("{}", t!("errors.invalid_lat", val = first.as_str()));
                std::process::exit(1);
            });
            let lng: f64 = first[comma_pos + 1..].parse().unwrap_or_else(|_| {
                eprintln!("{}", t!("errors.invalid_lng", val = first.as_str()));
                std::process::exit(1);
            });
            let days = parse_days(args.get(2));
            let loc = reverse_geocode(lat, lng).ok();
            (lat, lng, days, loc)
        } else if let Ok(lat) = first.parse::<f64>() {
            if args.len() < 3 {
                print_usage();
                std::process::exit(1);
            }
            let lng: f64 = args[2].parse().unwrap_or_else(|_| {
                eprintln!("{}", t!("errors.invalid_lng_pos", val = args[2].as_str()));
                std::process::exit(1);
            });
            let days = parse_days(args.get(3));
            let loc = reverse_geocode(lat, lng).ok();
            (lat, lng, days, loc)
        } else {
            let (city_parts, days) = if args.len() > 2 && looks_like_days(args.last().unwrap()) {
                let days = parse_days_str(args.last().unwrap());
                (&args[1..args.len() - 1], days)
            } else {
                (&args[1..], (1u32, 7u32))
            };
            let city_name = city_parts.join(" ");
            match geocode_city(&city_name) {
                Ok((lat, lng, city, country)) => (lat, lng, days, Some((city, country))),
                Err(e) => {
                    eprintln!("{}", t!("errors.generic", msg = e.to_string().as_str()));
                    std::process::exit(1);
                }
            }
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let term_w = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(120);

    let (day_from, day_to) = days;

    if let Some((hist_start, hist_end)) = delorean_dates {
        let n_days = (hist_end - hist_start).num_days() + 1;
        if n_days < 1 {
            eprintln!("{}", t!("errors.end_before_start"));
            std::process::exit(1);
        }

        if !no_eyecandy {
            let banner_main = match palette(0.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (0, 188, 212),
            };
            let banner_shadow = match palette(1.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (90, 0, 170),
            };
            writeln!(out)?;
            print_banner(&mut out, banner_main, banner_shadow, mono)?;
            let lat_str = if lat >= 0.0 {
                format!("{:.2}°N", lat)
            } else {
                format!("{:.2}°S", lat.abs())
            };
            let lng_str = if lng >= 0.0 {
                format!("{:.2}°E", lng)
            } else {
                format!("{:.2}°W", lng.abs())
            };
            let range_str = format!(
                "{} – {}",
                hist_start.format("%d.%m.%Y"),
                hist_end.format("%d.%m.%Y")
            );
            let loc_prefix = match &location {
                Some((city, country)) if !country.is_empty() => {
                    format!("{}, {}  ·  ", city, country)
                }
                Some((city, _)) => format!("{}  ·  ", city),
                None => String::new(),
            };
            let mode_str = if n_days <= 31 {
                t!("location.hist_hourly")
            } else if n_days <= 365 {
                t!("location.hist_daily")
            } else {
                t!("location.hist_monthly")
            };
            writeln!(
                out,
                "{}  {}{}, {}  ·  {}  ·  {}\n",
                t!("location.prefix"),
                loc_prefix,
                lat_str,
                lng_str,
                range_str,
                mode_str
            )?;
        }

        if n_days <= 31 {
            let (url, data) = fetch_historical_hourly(lat, lng, hist_start, hist_end)?;

            if tabular_bells {
                let (t_lbl, w_lbl, r_lbl, p_lbl) = (
                    units.temp_label(),
                    units.wind_label(),
                    units.rain_label(),
                    units.pressure_label(),
                );
                writeln!(
                    out,
                    "time,temp_{t_lbl},feel_{t_lbl},cloud_pct,precip_{r_lbl},wind_{w_lbl},gust_{w_lbl},pressure_{p_lbl},humidity_pct"
                )?;
                for h in &data {
                    let temp = if units.use_fahrenheit() {
                        c_to_f(h.temp)
                    } else {
                        h.temp
                    };
                    let feel = if units.use_fahrenheit() {
                        c_to_f(h.apparent_temp)
                    } else {
                        h.apparent_temp
                    };
                    let precip = if units.use_inches() {
                        mm_to_in(h.precip)
                    } else {
                        h.precip
                    };
                    let wind = if units.use_mph() {
                        kmh_to_mph(h.wind_speed)
                    } else {
                        h.wind_speed
                    };
                    let gust = if units.use_mph() {
                        kmh_to_mph(h.wind_gust)
                    } else {
                        h.wind_gust
                    };
                    let pressure = if units.use_inhg() {
                        hpa_to_inhg(h.pressure)
                    } else {
                        h.pressure
                    };
                    writeln!(
                        out,
                        "{},{:.1},{:.1},{:.0},{:.2},{:.1},{:.1},{:.2},{:.0}",
                        h.time.format("%Y-%m-%dT%H:%M"),
                        temp,
                        feel,
                        h.cloud,
                        precip,
                        wind,
                        gust,
                        pressure,
                        h.humidity
                    )?;
                }
            } else {
                if !no_charts {
                    print_overview(&mut out, &data, term_w, units, theme, chart_h, mono, true)?;
                }
                let mut dates: Vec<chrono::NaiveDate> =
                    data.iter().map(|h| h.time.date()).collect();
                dates.dedup();
                let solar: Vec<(
                    chrono::NaiveDate,
                    chrono::NaiveDateTime,
                    chrono::NaiveDateTime,
                )> = dates
                    .iter()
                    .map(|d| {
                        (
                            *d,
                            d.and_hms_opt(6, 0, 0).unwrap(),
                            d.and_hms_opt(20, 0, 0).unwrap(),
                        )
                    })
                    .collect();
                let summaries: Vec<_> = dates
                    .iter()
                    .map(|d| weather::day_summary(&data, &solar, *d))
                    .collect();
                if !no_table {
                    print_table(
                        &mut out, &data, &dates, &summaries, term_w, units, theme, mono, true,
                    )?;
                }
            }

            if !no_eyecandy {
                write_hist_footer(&mut out, &url, mono, VERSION)?;
            }
        } else {
            let (url, daily) = fetch_historical_daily(lat, lng, hist_start, hist_end)?;

            if n_days <= 365 {
                if tabular_bells {
                    let (t_lbl, w_lbl, r_lbl) =
                        (units.temp_label(), units.wind_label(), units.rain_label());
                    writeln!(
                        out,
                        "date,tmax_{t_lbl},tmin_{t_lbl},precip_{r_lbl},wind_max_{w_lbl},gust_max_{w_lbl}"
                    )?;
                    for d in &daily {
                        let tmax = if units.use_fahrenheit() {
                            c_to_f(d.max_temp)
                        } else {
                            d.max_temp
                        };
                        let tmin = if units.use_fahrenheit() {
                            c_to_f(d.min_temp)
                        } else {
                            d.min_temp
                        };
                        let rain = if units.use_inches() {
                            mm_to_in(d.precip_sum)
                        } else {
                            d.precip_sum
                        };
                        let wmax = if units.use_mph() {
                            kmh_to_mph(d.wind_max)
                        } else {
                            d.wind_max
                        };
                        let gmax = if units.use_mph() {
                            kmh_to_mph(d.gust_max)
                        } else {
                            d.gust_max
                        };
                        writeln!(
                            out,
                            "{},{:.1},{:.1},{:.2},{:.1},{:.1}",
                            d.date, tmax, tmin, rain, wmax, gmax
                        )?;
                    }
                } else {
                    if !no_charts {
                        print_historical_daily_charts(
                            &mut out, &daily, term_w, units, theme, chart_h, mono,
                        )?;
                    }
                    if !no_table {
                        print_historical_daily_table(&mut out, &daily, term_w, units, theme, mono)?;
                    }
                }
            } else {
                let monthly = aggregate_monthly(&daily);
                if tabular_bells {
                    let (t_lbl, w_lbl, r_lbl) =
                        (units.temp_label(), units.wind_label(), units.rain_label());
                    writeln!(
                        out,
                        "month,avg_tmax_{t_lbl},avg_tmin_{t_lbl},precip_sum_{r_lbl},wind_max_{w_lbl},gust_max_{w_lbl}"
                    )?;
                    for m in &monthly {
                        let tmax = if units.use_fahrenheit() {
                            c_to_f(m.avg_max_temp)
                        } else {
                            m.avg_max_temp
                        };
                        let tmin = if units.use_fahrenheit() {
                            c_to_f(m.avg_min_temp)
                        } else {
                            m.avg_min_temp
                        };
                        let rain = if units.use_inches() {
                            mm_to_in(m.precip_sum)
                        } else {
                            m.precip_sum
                        };
                        let wmax = if units.use_mph() {
                            kmh_to_mph(m.wind_max)
                        } else {
                            m.wind_max
                        };
                        let gmax = if units.use_mph() {
                            kmh_to_mph(m.gust_max)
                        } else {
                            m.gust_max
                        };
                        writeln!(
                            out,
                            "{}-{:02},{:.1},{:.1},{:.2},{:.1},{:.1}",
                            m.year, m.month, tmax, tmin, rain, wmax, gmax
                        )?;
                    }
                } else {
                    if !no_charts {
                        print_historical_monthly_charts(
                            &mut out, &monthly, term_w, units, theme, chart_h, mono,
                        )?;
                    }
                    if !no_table {
                        print_historical_monthly_table(
                            &mut out, &monthly, term_w, units, theme, mono,
                        )?;
                    }
                }
            }

            if !no_eyecandy {
                write_hist_footer(&mut out, &url, mono, VERSION)?;
            }
        }

        return Ok(());
    }

    if drone {
        let (api_url, mut drone_data, mut solar) = fetch_drone_weather(lat, lng, day_to)?;
        if day_from > 1 {
            let mut unique: Vec<chrono::NaiveDate> =
                drone_data.iter().map(|h| h.time.date()).collect();
            unique.dedup();
            let keep: std::collections::HashSet<_> =
                unique.into_iter().skip((day_from - 1) as usize).collect();
            drone_data.retain(|h| keep.contains(&h.time.date()));
            solar.retain(|(d, _, _)| keep.contains(d));
        }

        if tabular_bells {
            let (t_lbl, w_lbl, r_lbl) =
                (units.temp_label(), units.wind_label(), units.rain_label());
            writeln!(
                out,
                "time,temp_{t_lbl},feel_{t_lbl},precip_prob_pct,precip_{r_lbl},\
                wind10m_{w_lbl},wind80m_{w_lbl},wind120m_{w_lbl},wind180m_{w_lbl},\
                dir10m_deg,dir80m_deg,dir120m_deg,dir180m_deg,\
                gust10m_{w_lbl},uv_index"
            )?;
            for h in &drone_data {
                let temp = if units.use_fahrenheit() {
                    c_to_f(h.temp)
                } else {
                    h.temp
                };
                let feel = if units.use_fahrenheit() {
                    c_to_f(h.apparent_temp)
                } else {
                    h.apparent_temp
                };
                let rain = if units.use_inches() {
                    mm_to_in(h.precip)
                } else {
                    h.precip
                };
                let w10 = if units.use_mph() {
                    kmh_to_mph(h.wind_speed_10m)
                } else {
                    h.wind_speed_10m
                };
                let w80 = if units.use_mph() {
                    kmh_to_mph(h.wind_speed_80m)
                } else {
                    h.wind_speed_80m
                };
                let w120 = if units.use_mph() {
                    kmh_to_mph(h.wind_speed_120m)
                } else {
                    h.wind_speed_120m
                };
                let w180 = if units.use_mph() {
                    kmh_to_mph(h.wind_speed_180m)
                } else {
                    h.wind_speed_180m
                };
                let gust = if units.use_mph() {
                    kmh_to_mph(h.wind_gust_10m)
                } else {
                    h.wind_gust_10m
                };
                writeln!(
                    out,
                    "{},{:.1},{:.1},{:.0},{:.2},{:.1},{:.1},{:.1},{:.1},{:.0},{:.0},{:.0},{:.0},{:.1},{:.1}",
                    h.time.format("%Y-%m-%dT%H:%M"),
                    temp,
                    feel,
                    h.precip_prob,
                    rain,
                    w10,
                    w80,
                    w120,
                    w180,
                    h.wind_dir_10m,
                    h.wind_dir_80m,
                    h.wind_dir_120m,
                    h.wind_dir_180m,
                    gust,
                    h.uv_index
                )?;
            }
            return Ok(());
        }

        if !no_eyecandy {
            let banner_main = match palette(0.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (0, 188, 212),
            };
            let banner_shadow = match palette(1.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (90, 0, 170),
            };
            writeln!(out)?;
            print_banner(&mut out, banner_main, banner_shadow, mono)?;
            let forecast_date = drone_data
                .first()
                .map(|h| h.time.date())
                .unwrap_or_default();
            let lat_str = if lat >= 0.0 {
                format!("{:.2}°N", lat)
            } else {
                format!("{:.2}°S", lat.abs())
            };
            let lng_str = if lng >= 0.0 {
                format!("{:.2}°E", lng)
            } else {
                format!("{:.2}°W", lng.abs())
            };
            let days_str = if day_from > 1 {
                t!(
                    "location.days_range",
                    from = day_from.to_string().as_str(),
                    to = day_to.to_string().as_str()
                )
                .to_string()
            } else if day_to == 1 {
                t!("location.day_1").to_string()
            } else {
                t!("location.days_fmt_n", n = day_to.to_string().as_str()).to_string()
            };
            let chrono_loc = locale::chrono_locale(lang);
            let date_fmt = t!("location.date_full");
            let date_str = forecast_date
                .format_localized(date_fmt.as_ref(), chrono_loc)
                .to_string();
            let loc_prefix = match &location {
                Some((city, country)) if !country.is_empty() => {
                    format!("{}, {}  ·  ", city, country)
                }
                Some((city, _)) => format!("{}  ·  ", city),
                None => String::new(),
            };
            writeln!(
                out,
                "{}  {}{}, {}  ·  {}  ·  {}  ·  {}\n",
                t!("location.prefix"),
                loc_prefix,
                lat_str,
                lng_str,
                days_str,
                date_str,
                t!("location.drone_profile")
            )?;
        }

        let mut dates: Vec<chrono::NaiveDate> = drone_data.iter().map(|h| h.time.date()).collect();
        dates.dedup();
        let summaries: Vec<_> = dates
            .iter()
            .map(|d| weather::drone_day_summary(&drone_data, &solar, *d))
            .collect();

        if !no_table {
            render::drone::print_drone_table(
                &mut out,
                &drone_data,
                &dates,
                &summaries,
                term_w,
                units,
                theme,
                mono,
            )?;
        }

        if !no_eyecandy {
            let dim = if mono { "" } else { "\x1b[90m" };
            let reset = if mono { "" } else { "\x1b[0m" };
            let mods = t!("usage.modifiers_title");
            let indent = " ".repeat(mods.chars().count() + 1);
            writeln!(out)?;
            write!(out, "{dim}")?;
            writeln!(out, "{}", t!("footer.data_source"))?;
            writeln!(out, "{} {api_url}", t!("footer.api_url_label"))?;
            writeln!(out)?;
            writeln!(out, "{} --i-drone-you  --delorean  --units  --lang", mods)?;
            writeln!(
                out,
                "{}--i-am-blue  --color-me  --classic-colors  --rainforest  --i-cant-afford-cga",
                indent
            )?;
            writeln!(
                out,
                "{}--no-eyecandy  --high-charts  --no-charts  --no-table  --tabular-bells",
                indent
            )?;
            writeln!(out)?;
            writeln!(out, "{}  v{VERSION}", t!("footer.github"))?;
            write!(out, "{reset}")?;
        }
    } else {
        let (api_url, mut data, mut solar) = fetch_weather(lat, lng, day_to)?;
        if day_from > 1 {
            let mut unique: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
            unique.dedup();
            let keep: std::collections::HashSet<_> =
                unique.into_iter().skip((day_from - 1) as usize).collect();
            data.retain(|h| keep.contains(&h.time.date()));
            solar.retain(|(d, _, _)| keep.contains(d));
        }

        // CSV mode: skip all visual output
        if tabular_bells {
            let (t_lbl, w_lbl, r_lbl, p_lbl) = (
                units.temp_label(),
                units.wind_label(),
                units.rain_label(),
                units.pressure_label(),
            );
            writeln!(
                out,
                "time,temp_{t_lbl},feel_{t_lbl},cloud_pct,precip_prob_pct,precip_{r_lbl},wind_{w_lbl},gust_{w_lbl},pressure_{p_lbl},humidity_pct"
            )?;
            for h in &data {
                let temp = if units.use_fahrenheit() {
                    c_to_f(h.temp)
                } else {
                    h.temp
                };
                let feel = if units.use_fahrenheit() {
                    c_to_f(h.apparent_temp)
                } else {
                    h.apparent_temp
                };
                let precip = if units.use_inches() {
                    mm_to_in(h.precip)
                } else {
                    h.precip
                };
                let wind = if units.use_mph() {
                    kmh_to_mph(h.wind_speed)
                } else {
                    h.wind_speed
                };
                let gust = if units.use_mph() {
                    kmh_to_mph(h.wind_gust)
                } else {
                    h.wind_gust
                };
                let pressure = if units.use_inhg() {
                    hpa_to_inhg(h.pressure)
                } else {
                    h.pressure
                };
                writeln!(
                    out,
                    "{},{:.1},{:.1},{:.0},{:.0},{:.2},{:.1},{:.1},{:.2},{:.0}",
                    h.time.format("%Y-%m-%dT%H:%M"),
                    temp,
                    feel,
                    h.cloud,
                    h.precip_prob,
                    precip,
                    wind,
                    gust,
                    pressure,
                    h.humidity
                )?;
            }
            return Ok(());
        }

        if !no_eyecandy {
            let banner_main = match palette(0.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (0, 188, 212),
            };
            let banner_shadow = match palette(1.0, theme) {
                Color::Rgb(r, g, b) => (r, g, b),
                _ => (90, 0, 170),
            };
            writeln!(out)?;
            print_banner(&mut out, banner_main, banner_shadow, mono)?;

            let forecast_date = data.first().map(|h| h.time.date()).unwrap_or_default();
            let lat_str = if lat >= 0.0 {
                format!("{:.2}°N", lat)
            } else {
                format!("{:.2}°S", lat.abs())
            };
            let lng_str = if lng >= 0.0 {
                format!("{:.2}°E", lng)
            } else {
                format!("{:.2}°W", lng.abs())
            };
            let days_str = if day_from > 1 {
                t!(
                    "location.days_range",
                    from = day_from.to_string().as_str(),
                    to = day_to.to_string().as_str()
                )
                .to_string()
            } else if day_to == 1 {
                t!("location.day_1").to_string()
            } else {
                t!("location.days_fmt_n", n = day_to.to_string().as_str()).to_string()
            };
            let chrono_loc = locale::chrono_locale(lang);
            let date_fmt = t!("location.date_full");
            let date_str = forecast_date
                .format_localized(date_fmt.as_ref(), chrono_loc)
                .to_string();
            let loc_prefix = match &location {
                Some((city, country)) if !country.is_empty() => {
                    format!("{}, {}  ·  ", city, country)
                }
                Some((city, _)) => format!("{}  ·  ", city),
                None => String::new(),
            };
            writeln!(
                out,
                "{}  {}{}, {}  ·  {}  ·  {}\n",
                t!("location.prefix"),
                loc_prefix,
                lat_str,
                lng_str,
                days_str,
                date_str
            )?;
        }

        if !no_charts {
            print_overview(&mut out, &data, term_w, units, theme, chart_h, mono, false)?;
        }

        let mut dates: Vec<chrono::NaiveDate> = data.iter().map(|h| h.time.date()).collect();
        dates.dedup();
        let summaries: Vec<_> = dates
            .iter()
            .map(|d| weather::day_summary(&data, &solar, *d))
            .collect();

        if !no_table {
            print_table(
                &mut out, &data, &dates, &summaries, term_w, units, theme, mono, false,
            )?;
        }

        if !no_eyecandy {
            let dim = if mono { "" } else { "\x1b[90m" };
            let reset = if mono { "" } else { "\x1b[0m" };
            let mods = t!("usage.modifiers_title");
            let indent = " ".repeat(mods.chars().count() + 1);
            writeln!(out)?;
            write!(out, "{dim}")?;
            writeln!(out, "{}", t!("footer.data_source"))?;
            writeln!(out, "{} {api_url}", t!("footer.api_url_label"))?;
            writeln!(out)?;
            writeln!(out, "{} --i-drone-you  --delorean  --units  --lang", mods)?;
            writeln!(
                out,
                "{}--i-am-blue  --color-me  --classic-colors  --rainforest  --i-cant-afford-cga",
                indent
            )?;
            writeln!(
                out,
                "{}--no-eyecandy  --high-charts  --no-charts  --no-table  --tabular-bells",
                indent
            )?;
            writeln!(out)?;
            writeln!(out, "{}  v{VERSION}", t!("footer.github"))?;
            write!(out, "{reset}")?;
        }
    }

    Ok(())
}

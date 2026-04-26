use chrono::Timelike;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use rust_i18n::t;
use std::io::{self, Write as IoWrite};

use crate::colors::{cloud_color, palette, pressure_color, temp_color, wind_color};
use crate::render::{
    bars::{rain_block_cell, temp_bar, value_bar, wind_bar},
    emit_span,
};
use crate::types::{DaySummary, HourlyData, Theme, Units};
use crate::units::{c_to_f, hpa_to_inhg, kmh_to_mph, mm_to_in};
use crate::weather::day_name;

/// (header, label_w, default_bar_w)
pub const COL_DEFS: &[(&str, usize, usize)] = &[
    ("TEMP/FEEL °C", 11, 9),
    ("CLOUD %", 3, 10),
    ("RAIN %→/mm↑", 9, 10),
    ("WIND km/h", 11, 9),
    ("PRESSURE hPa", 6, 8),
    ("HUMIDITY %", 3, 10),
];

/// All summary-row label keys used by `summary_parts`.
const SUMMARY_KEYS: &[&str] = &[
    "summary.sunrise",
    "summary.sunset",
    "summary.temp_max",
    "summary.temp_min",
    "summary.feel_max",
    "summary.feel_min",
    "summary.cloud_avg",
    "summary.rain_prob",
    "summary.rain_sum",
    "summary.wind",
    "summary.gusts",
    "summary.pressure",
    "summary.humidity",
];

/// Width of the formatted value half of a summary row (e.g. " 58.3°F", "   58%", "  0.00in").
/// Every value in `summary_parts` is padded to exactly this many display chars.
const SUMMARY_VALUE_W: usize = 7;

/// Char-count of the longest localized summary label for the active locale.
/// Used to size the day-summary column so translated labels don't push the table out of alignment.
fn summary_label_w() -> usize {
    SUMMARY_KEYS
        .iter()
        .map(|k| t!(*k).chars().count())
        .max()
        .unwrap_or(0)
}

pub fn col_title(i: usize, units: Units, historical: bool) -> String {
    match i {
        0 => format!("{} °{}", t!("col.temp_feel"), units.temp_label()),
        1 => t!("col.cloud").to_string(),
        2 => {
            if historical {
                format!("{} {}", t!("col.rain"), units.rain_label())
            } else {
                format!("{} %→/{}↑", t!("col.rain"), units.rain_label())
            }
        }
        3 => format!("{} {}", t!("col.wind"), units.wind_label()),
        4 => format!("{} {}", t!("col.pressure"), units.pressure_label()),
        5 => t!("col.humidity").to_string(),
        _ => COL_DEFS[i].0.to_string(),
    }
}

/// Per-day summary column. Labels padded to `label_w`, values to `SUMMARY_VALUE_W`.
pub fn summary_parts(s: &DaySummary, units: Units, label_w: usize) -> Vec<(String, String)> {
    let t = |v: f64| {
        if units.use_fahrenheit() {
            format!("{:>5.1}°F", c_to_f(v))
        } else {
            format!("{:>5.1}°C", v)
        }
    };
    let w = |v: f64| {
        if units.use_mph() {
            format!("{:>4.0}mph", kmh_to_mph(v))
        } else {
            format!("{:>3.0}km/h", v)
        }
    };
    let r = |v: f64| {
        if units.use_inches() {
            format!("{:>5.2}in", mm_to_in(v))
        } else {
            format!("{:>5.1}mm", v)
        }
    };
    let p = |v: f64| {
        if units.use_inhg() {
            format!("{:>5.2}in", hpa_to_inhg(v))
        } else {
            format!("{:>4.0}hPa", v)
        }
    };
    let lbl = |k: &str| format!("{:<label_w$}", t!(k));
    vec![
        (format!("{}", s.date.format("%Y-%m-%d")), String::new()),
        (day_name(s.date), String::new()),
        (
            lbl("summary.sunrise"),
            format!("{:>7}", s.sunrise.format("%H:%M")),
        ),
        (
            lbl("summary.sunset"),
            format!("{:>7}", s.sunset.format("%H:%M")),
        ),
        (lbl("summary.temp_max"), t(s.max_temp)),
        (lbl("summary.temp_min"), t(s.min_temp)),
        (lbl("summary.feel_max"), t(s.max_apparent)),
        (lbl("summary.feel_min"), t(s.min_apparent)),
        (lbl("summary.cloud_avg"), format!("{:>6.0}%", s.avg_cloud)),
        (
            lbl("summary.rain_prob"),
            format!("{:>6.0}%", s.max_precip_prob),
        ),
        (lbl("summary.rain_sum"), r(s.total_precip)),
        (lbl("summary.wind"), w(s.max_wind_speed)),
        (lbl("summary.gusts"), w(s.max_wind_gust)),
        (lbl("summary.pressure"), p(s.avg_pressure)),
        (lbl("summary.humidity"), format!("{:>6.0}%", s.avg_humidity)),
    ]
}

pub fn print_table(
    out: &mut impl IoWrite,
    data: &[HourlyData],
    dates: &[chrono::NaiveDate],
    summaries: &[DaySummary],
    term_w: usize,
    units: Units,
    theme: Theme,
    mono: bool,
    historical: bool,
) -> io::Result<()> {
    use chrono::NaiveDateTime;
    let temp_min = data
        .iter()
        .map(|h| h.apparent_temp.min(h.temp))
        .fold(f64::INFINITY, f64::min)
        - 2.0;
    let temp_max = data
        .iter()
        .map(|h| h.apparent_temp.max(h.temp))
        .fold(f64::NEG_INFINITY, f64::max)
        + 2.0;
    let pressure_min = data
        .iter()
        .map(|h| h.pressure)
        .fold(f64::INFINITY, f64::min)
        - 2.0;
    let pressure_max = data
        .iter()
        .map(|h| h.pressure)
        .fold(f64::NEG_INFINITY, f64::max)
        + 2.0;
    let wind_max = data
        .iter()
        .map(|h| h.wind_gust.max(h.wind_speed))
        .fold(0.0_f64, f64::max)
        + 2.0;
    let max_mm = data
        .iter()
        .map(|h| h.precip)
        .fold(0.0_f64, f64::max)
        .max(0.1);

    // Day-summary column width is locale-dependent: it must hold the widest summary label
    // (label_w + value 7 + trailing space 1) AND the column header itself. Floor of 18 keeps
    // the original English layout unchanged.
    let header_w = t!("table.day_summary").chars().count();
    let natural_label_w = summary_label_w();
    let day_w: usize = (natural_label_w + SUMMARY_VALUE_W + 1)
        .max(header_w)
        .max(18);
    let label_w: usize = day_w - SUMMARY_VALUE_W - 1;
    let hour_w: usize = 6;
    const MIN_BAR: usize = 3;

    let mut n_cols = COL_DEFS.len();
    loop {
        let needed: usize = day_w
            + hour_w
            + COL_DEFS[..n_cols]
                .iter()
                .map(|(_, lw, _)| 1 + lw + 1 + MIN_BAR)
                .sum::<usize>();
        if needed <= term_w || n_cols == 1 {
            break;
        }
        n_cols -= 1;
    }

    let active = &COL_DEFS[..n_cols];
    let fixed: usize = day_w + hour_w + active.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = active.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = active
        .iter()
        .map(|(_, _, bw)| ((bw * available) / default_total.max(1)).max(MIN_BAR))
        .collect();
    let used: usize = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w {
        bar_ws[n_cols - 1] += term_w - used;
    }

    let sep_w: usize = fixed + bar_ws.iter().sum::<usize>();
    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    let hdr_col = |lw: usize, bw: usize, title: &str| -> String {
        format!(" {:<width$}", title, width = lw + 1 + bw)
    };
    let mut hdr_spans = vec![
        Span::styled(format!("{:<day_w$}", t!("table.day_summary")), hdr),
        Span::raw(format!("{:hour_w$}", "")),
    ];
    for (i, (_, lw, _)) in active.iter().enumerate() {
        hdr_spans.push(Span::styled(
            hdr_col(*lw, bar_ws[i], &col_title(i, units, historical)),
            hdr,
        ));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));

    let mut current_sunrise: Option<NaiveDateTime> = None;
    let mut current_sunset: Option<NaiveDateTime> = None;

    let mut day_summary_idx: usize;
    let mut current_date: Option<chrono::NaiveDate> = None;
    let mut day_row_count = 0usize;
    let mut day_parts_cache: Vec<(String, String)> = Vec::new();

    for hd in data {
        let hour = hd.time.hour();
        let date = hd.time.date();

        if current_date != Some(date) {
            if current_date.is_some() {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));
            }
            current_date = Some(date);
            day_row_count = 0;
            day_summary_idx = dates.iter().position(|d| *d == date).unwrap_or(0);
            current_sunrise = Some(summaries[day_summary_idx].sunrise);
            current_sunset = Some(summaries[day_summary_idx].sunset);
            day_parts_cache = summary_parts(&summaries[day_summary_idx], units, label_w);
        }

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut spans: Vec<Span> = if day_row_count < day_parts_cache.len() {
            let (label, value) = &day_parts_cache[day_row_count];
            if day_row_count == 1 {
                vec![Span::styled(
                    format!("{:<day_w$}", label),
                    Style::default()
                        .fg(palette(0.0, theme))
                        .add_modifier(Modifier::BOLD),
                )]
            } else if value.is_empty() {
                vec![Span::styled(format!("{:<day_w$}", label), bold)]
            } else {
                vec![
                    Span::raw(label.clone()),
                    Span::styled(value.clone(), bold),
                    Span::raw(" "),
                ]
            }
        } else {
            vec![Span::raw(format!("{:<day_w$}", ""))]
        };
        day_row_count += 1;

        let h_style = if current_sunrise.is_some_and(|sr| hd.time >= sr)
            && current_sunset.is_some_and(|ss| hd.time < ss)
        {
            Style::default().fg(palette(0.5, theme))
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!("{:02}:00 ", hour), h_style));

        for (i, _) in active.iter().enumerate() {
            let bw = bar_ws[i];
            let (label, bar): (Span, Vec<Span>) = match i {
                0 => {
                    let (dt, df) = if units.use_fahrenheit() {
                        (c_to_f(hd.temp), c_to_f(hd.apparent_temp))
                    } else {
                        (hd.temp, hd.apparent_temp)
                    };
                    let c = temp_color(hd.temp, theme);
                    (
                        Span::styled(format!("{:>5.1}/{:>5.1}", dt, df), Style::default().fg(c)),
                        temp_bar(hd.temp, hd.apparent_temp, temp_min, temp_max, bw, theme),
                    )
                }
                1 => {
                    let c = cloud_color(hd.cloud);
                    (
                        Span::styled(format!("{:>3.0}", hd.cloud), Style::default().fg(c)),
                        value_bar(hd.cloud, 0.0, 100.0, bw, c),
                    )
                }
                2 => {
                    let mm_disp = if units.use_inches() {
                        mm_to_in(hd.precip)
                    } else {
                        hd.precip
                    };
                    let max_disp = if units.use_inches() {
                        mm_to_in(max_mm)
                    } else {
                        max_mm
                    };
                    let c = palette((mm_disp / max_disp).clamp(0.0, 1.0), theme);
                    if historical {
                        let label = if units.use_inches() {
                            format!("{:>8.2}", mm_disp)
                        } else {
                            format!("{:>8.1}", mm_disp)
                        };
                        (
                            Span::styled(label, Style::default().fg(c)),
                            value_bar(mm_disp, 0.0, max_disp, bw, c),
                        )
                    } else {
                        let prob_t = hd.precip_prob / 100.0;
                        let mm_t = (hd.precip / max_mm).clamp(0.0, 1.0);
                        let c = palette((prob_t * 0.5 + mm_t * 0.5).clamp(0.0, 1.0), theme);
                        let label = if units.use_inches() {
                            format!("{:>3.0}%/{:>4.2}", hd.precip_prob, mm_disp)
                        } else {
                            format!("{:>3.0}%/{:>4.1}", hd.precip_prob, mm_disp)
                        };
                        (
                            Span::styled(label, Style::default().fg(c)),
                            rain_block_cell(hd.precip_prob, mm_disp, max_disp, bw, c),
                        )
                    }
                }
                3 => {
                    let (ds, dg) = if units.use_mph() {
                        (kmh_to_mph(hd.wind_speed), kmh_to_mph(hd.wind_gust))
                    } else {
                        (hd.wind_speed, hd.wind_gust)
                    };
                    let c = wind_color(hd.wind_speed, theme);
                    (
                        Span::styled(format!("{:>5.1}/{:>5.1}", ds, dg), Style::default().fg(c)),
                        wind_bar(hd.wind_speed, hd.wind_gust, 0.0, wind_max, bw, theme),
                    )
                }
                4 => {
                    let c = pressure_color(hd.pressure, theme);
                    let disp = if units.use_inhg() {
                        format!("{:>6.2}", hpa_to_inhg(hd.pressure))
                    } else {
                        format!("{:>6.0}", hd.pressure)
                    };
                    (
                        Span::styled(disp, Style::default().fg(c)),
                        value_bar(hd.pressure, pressure_min, pressure_max, bw, c),
                    )
                }
                _ => {
                    let c = palette(hd.humidity / 100.0, theme);
                    (
                        Span::styled(format!("{:>3.0}", hd.humidity), Style::default().fg(c)),
                        value_bar(hd.humidity, 0.0, 100.0, bw, c),
                    )
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
            emit_span(out, span, mono)?;
        }
        writeln!(out)?;
    }
    Ok(())
}

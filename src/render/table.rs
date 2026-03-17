use chrono::Timelike;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::{self, Write as IoWrite};

use crate::colors::{cloud_color, palette, pressure_color, temp_color, wind_color};
use crate::render::{bars::{temp_bar, value_bar, wind_bar}, emit_span};
use crate::types::{DaySummary, HourlyData, Theme};
use crate::units::{c_to_f, hpa_to_inhg, kmh_to_mph, mm_to_in};
use crate::weather::day_name;

/// (header, label_w, default_bar_w)
pub const COL_DEFS: &[(&str, usize, usize)] = &[
    ("TEMP/FEEL °C", 11, 9),
    ("CLOUD %",       3, 10),
    ("RAIN %",        3, 10),
    ("RAIN mm",       4,  8),
    ("WIND km/h",    11,  9),
    ("PRESSURE hPa",  6,  8),
    ("HUMIDITY %",    3, 10),
];

pub fn col_title(i: usize, imperial: bool) -> &'static str {
    if imperial {
        match i {
            0 => "TEMP/FEEL °F",
            3 => "RAIN in",
            4 => "WIND mph",
            5 => "PRES inHg",
            _ => COL_DEFS[i].0,
        }
    } else {
        COL_DEFS[i].0
    }
}

/// Per-day summary column (label 10 chars, value 7 chars).
pub fn summary_parts(s: &DaySummary, imperial: bool) -> Vec<(String, String)> {
    let t = |v: f64| if imperial { format!("{:>5.1}°F", c_to_f(v))       } else { format!("{:>5.1}°C", v)     };
    let w = |v: f64| if imperial { format!("{:>4.0}mph", kmh_to_mph(v))   } else { format!("{:>3.0}km/h", v)   };
    let r = |v: f64| if imperial { format!("{:>5.2}in", mm_to_in(v))      } else { format!("{:>5.1}mm", v)     };
    let p = |v: f64| if imperial { format!("{:>5.2}in", hpa_to_inhg(v))   } else { format!("{:>4.0}hPa", v)    };
    vec![
        (format!("{}", s.date.format("%Y-%m-%d")), String::new()),
        (format!("{}", day_name(s.date)), String::new()),
        (format!("{:<10}", "Temp max:"),  t(s.max_temp)),
        (format!("{:<10}", "Temp min:"),  t(s.min_temp)),
        (format!("{:<10}", "Feel max:"),  t(s.max_apparent)),
        (format!("{:<10}", "Feel min:"),  t(s.min_apparent)),
        (format!("{:<10}", "Cloud avg:"), format!("{:>6.0}%", s.avg_cloud)),
        (format!("{:<10}", "Rain prob:"), format!("{:>6.0}%", s.max_precip_prob)),
        (format!("{:<10}", "Rain sum:"),  r(s.total_precip)),
        (format!("{:<10}", "Wind:"),      w(s.max_wind_speed)),
        (format!("{:<10}", "Gusts:"),     w(s.max_wind_gust)),
        (format!("{:<10}", "Pressure:"),  p(s.avg_pressure)),
        (format!("{:<10}", "Humidity:"),  format!("{:>6.0}%", s.avg_humidity)),
    ]
}

pub fn print_table(
    out: &mut impl IoWrite,
    data: &[HourlyData],
    dates: &[chrono::NaiveDate],
    summaries: &[DaySummary],
    term_w: usize,
    imperial: bool,
    theme: Theme,
) -> io::Result<()> {
    let temp_min = data.iter().map(|h| h.apparent_temp.min(h.temp)).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|h| h.apparent_temp.max(h.temp)).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let pressure_min = data.iter().map(|h| h.pressure).fold(f64::INFINITY, f64::min) - 2.0;
    let pressure_max = data.iter().map(|h| h.pressure).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let wind_max = data.iter().map(|h| h.wind_gust.max(h.wind_speed)).fold(0.0_f64, f64::max) + 2.0;

    let day_w:  usize = 18;
    let hour_w: usize = 6;
    const MIN_BAR: usize = 3;

    let mut n_cols = COL_DEFS.len();
    loop {
        let needed: usize = day_w + hour_w
            + COL_DEFS[..n_cols].iter().map(|(_, lw, _)| 1 + lw + 1 + MIN_BAR).sum::<usize>();
        if needed <= term_w || n_cols == 1 { break; }
        n_cols -= 1;
    }

    let active = &COL_DEFS[..n_cols];
    let fixed: usize = day_w + hour_w + active.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = active.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = active.iter().map(|(_, _, bw)| {
        ((bw * available) / default_total.max(1)).max(MIN_BAR)
    }).collect();
    let used: usize = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w { bar_ws[n_cols - 1] += term_w - used; }

    let sep_w: usize = fixed + bar_ws.iter().sum::<usize>();
    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    let hdr_col = |lw: usize, bw: usize, title: &str| -> String {
        format!(" {:<width$}", title, width = lw + 1 + bw)
    };
    let mut hdr_spans = vec![
        Span::styled(format!("{:<day_w$}", "DAY SUMMARY"), hdr),
        Span::raw(format!("{:hour_w$}", "")),
    ];
    for (i, (_, lw, _)) in active.iter().enumerate() {
        hdr_spans.push(Span::styled(hdr_col(*lw, bar_ws[i], col_title(i, imperial)), hdr));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));

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
            day_parts_cache = summary_parts(&summaries[day_summary_idx], imperial);
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
                0 => {
                    let (dt, df) = if imperial {
                        (c_to_f(hd.temp), c_to_f(hd.apparent_temp))
                    } else {
                        (hd.temp, hd.apparent_temp)
                    };
                    let c = temp_color(hd.temp, theme);
                    (Span::styled(format!("{:>5.1}/{:>5.1}", dt, df), Style::default().fg(c)),
                     temp_bar(hd.temp, hd.apparent_temp, temp_min, temp_max, bw, theme))
                }
                1 => {
                    let c = cloud_color(hd.cloud);
                    (Span::styled(format!("{:>3.0}", hd.cloud), Style::default().fg(c)),
                     value_bar(hd.cloud, 0.0, 100.0, bw, c))
                }
                2 => {
                    let c = palette(hd.precip_prob / 100.0, theme);
                    (Span::styled(format!("{:>3.0}", hd.precip_prob), Style::default().fg(c)),
                     value_bar(hd.precip_prob, 0.0, 100.0, bw, c))
                }
                3 => {
                    let c = palette((hd.precip / 10.0).clamp(0.0, 1.0), theme);
                    let disp = if imperial {
                        format!("{:>4.2}", mm_to_in(hd.precip))
                    } else {
                        format!("{:>4.1}", hd.precip)
                    };
                    (Span::styled(disp, Style::default().fg(c)),
                     value_bar(hd.precip, 0.0, 10.0, bw, c))
                }
                4 => {
                    let (ds, dg) = if imperial {
                        (kmh_to_mph(hd.wind_speed), kmh_to_mph(hd.wind_gust))
                    } else {
                        (hd.wind_speed, hd.wind_gust)
                    };
                    let c = wind_color(hd.wind_speed, theme);
                    (Span::styled(format!("{:>5.1}/{:>5.1}", ds, dg), Style::default().fg(c)),
                     wind_bar(hd.wind_speed, hd.wind_gust, 0.0, wind_max, bw, theme))
                }
                5 => {
                    let c = pressure_color(hd.pressure, theme);
                    let disp = if imperial {
                        format!("{:>6.2}", hpa_to_inhg(hd.pressure))
                    } else {
                        format!("{:>6.0}", hd.pressure)
                    };
                    (Span::styled(disp, Style::default().fg(c)),
                     value_bar(hd.pressure, pressure_min, pressure_max, bw, c))
                }
                _ => {
                    let c = palette(hd.humidity / 100.0, theme);
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
            emit_span(out, span)?;
        }
        writeln!(out)?;
    }
    Ok(())
}

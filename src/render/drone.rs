use chrono::Timelike;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::{self, Write as IoWrite};

use crate::colors::{palette, temp_color, wind_color};
use crate::render::{bars::{rain_block_cell, value_bar}, emit_span};
use crate::types::{DroneDaySummary, DroneHourlyData, Theme, Units};
use crate::units::{c_to_f, kmh_to_mph, mm_to_in};
use crate::weather::day_name;

/// Arrow showing where wind pushes the drone (opposite of meteorological "from" direction).
pub fn wind_arrow(from_deg: f64) -> char {
    let to_deg = (from_deg + 180.0) % 360.0;
    let sector = ((to_deg + 22.5) / 45.0) as usize % 8;
    ['↑', '↗', '→', '↘', '↓', '↙', '←', '↖'][sector]
}

/// (header, label_w, default_bar_w)
pub const DRONE_COL_DEFS: &[(&str, usize, usize)] = &[
    ("TEMP °C",        5, 7),
    ("RAIN",           9, 8),
    ("W  10m",         6, 6),
    ("W  80m",         6, 6),
    ("W 120m",         6, 6),
    ("W 180m",         6, 6),
    ("GUSTS",          5, 6),
    ("UV IDX",         4, 5),
];

pub fn drone_col_title(i: usize, units: Units) -> &'static str {
    match i {
        0 => if units.use_fahrenheit() { "TEMP °F"      } else { "TEMP °C"      },
        1 => if units.use_inches()     { "RAIN %→/in↑" } else { "RAIN %→/mm↑" },
        2 => if units.use_mph()        { "W  10m mph"   } else { "W 10m km/h"  },
        3 => if units.use_mph()        { "W  80m mph"   } else { "W 80m km/h"  },
        4 => if units.use_mph()        { "W 120m mph"   } else { "W 120m km/h" },
        5 => if units.use_mph()        { "W 180m mph"   } else { "W 180m km/h" },
        _ => DRONE_COL_DEFS[i].0,
    }
}

pub fn drone_summary_parts(s: &DroneDaySummary, units: Units) -> Vec<(String, String)> {
    let t = |v: f64| if units.use_fahrenheit() { format!("{:>5.1}°F", c_to_f(v))      } else { format!("{:>5.1}°C", v)    };
    let w = |v: f64| if units.use_mph()         { format!("{:>4.0}mph", kmh_to_mph(v)) } else { format!("{:>3.0}km/h", v)  };
    let r = |v: f64| if units.use_inches()       { format!("{:>5.2}in", mm_to_in(v))   } else { format!("{:>5.1}mm", v)    };
    vec![
        (format!("{}", s.date.format("%Y-%m-%d")), String::new()),
        (format!("{}", day_name(s.date)), String::new()),
        (format!("{:<10}", "Sunrise:"),   format!("{:>7}", s.sunrise.format("%H:%M"))),
        (format!("{:<10}", "Sunset:"),    format!("{:>7}", s.sunset.format("%H:%M"))),
        (format!("{:<10}", "Temp max:"),  t(s.max_temp)),
        (format!("{:<10}", "Temp min:"),  t(s.min_temp)),
        (format!("{:<10}", "Rain prob:"), format!("{:>6.0}%", s.max_precip_prob)),
        (format!("{:<10}", "Rain sum:"),  r(s.total_precip)),
        (format!("{:<10}", "W 10m:"),     format!("{}", w(s.max_wind_10m))),
        (format!("{:<10}", "W 80m:"),     format!("{}", w(s.max_wind_80m))),
        (format!("{:<10}", "W 120m:"),    format!("{}", w(s.max_wind_120m))),
        (format!("{:<10}", "W 180m:"),    format!("{}", w(s.max_wind_180m))),
        (format!("{:<10}", "Gusts:"),     format!("{}", w(s.max_gust_10m))),
        (format!("{:<10}", "UV max:"),    format!(" {:>6.1}", s.max_uv)),
    ]
}

fn push_wind_chart(
    lines: &mut Vec<Line<'static>>,
    data: &[DroneHourlyData],
    date: chrono::NaiveDate,
    left_pad: usize,
    _wind_max: f64,
    theme: Theme,
    dim_sty: Style,
) {
    let day: Vec<&DroneHourlyData> = data.iter().filter(|h| h.time.date() == date).collect();
    let pad = Span::raw(" ".repeat(left_pad));
    // label width: 4 chars for altitude + 1 space separator
    const LBL: usize = 5;
    let altitudes: &[(&str, fn(&DroneHourlyData) -> (f64, f64))] = &[
        ("180m", |h| (h.wind_speed_180m, h.wind_dir_180m)),
        ("120m", |h| (h.wind_speed_120m, h.wind_dir_120m)),
        (" 80m", |h| (h.wind_speed_80m,  h.wind_dir_80m)),
        (" 10m", |h| (h.wind_speed_10m,  h.wind_dir_10m)),
    ];
    for (label, get) in altitudes {
        let mut spans: Vec<Span> = vec![
            pad.clone(),
            Span::styled(format!("{:<LBL$}", label), dim_sty),
        ];
        for hour in 0..24usize {
            if let Some(hd) = day.iter().find(|h| h.time.hour() as usize == hour) {
                let (speed, dir) = get(hd);
                if speed == 0.0 {
                    spans.push(Span::raw(" "));
                } else {
                    let arrow = wind_arrow(dir);
                    let c = wind_color(speed, theme);
                    let sty = Style::default().bg(c).fg(Color::White);
                    spans.push(Span::styled(arrow.to_string(), sty));
                }
            } else {
                spans.push(Span::raw(" "));
            }
        }
        lines.push(Line::from(spans));
    }
    // Hours ruler
    let mut hour_chars = vec![' '; 24];
    for h in [0usize, 6, 12, 18] {
        let s = format!("{}", h);
        for (j, c) in s.chars().enumerate() {
            if h + j < 24 { hour_chars[h + j] = c; }
        }
    }
    let ruler: String = hour_chars.into_iter().collect();
    lines.push(Line::from(vec![
        pad.clone(),
        Span::styled(format!("{:<LBL$}", ""), dim_sty),
        Span::styled(ruler, dim_sty),
    ]));
}

pub fn print_drone_table(
    out: &mut impl IoWrite,
    data: &[DroneHourlyData],
    dates: &[chrono::NaiveDate],
    summaries: &[DroneDaySummary],
    term_w: usize,
    units: Units,
    theme: Theme,
    mono: bool,
) -> io::Result<()> {
    use chrono::NaiveDate;
    if data.is_empty() { return Ok(()); }

    let temp_min = data.iter().map(|h| h.temp).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|h| h.temp).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let wind_max = data.iter().map(|h|
        h.wind_speed_10m.max(h.wind_speed_80m).max(h.wind_speed_120m).max(h.wind_speed_180m)
    ).fold(0.0_f64, f64::max) + 2.0;
    let max_mm   = data.iter().map(|h| h.precip).fold(0.0_f64, f64::max).max(0.1);
    let gust_max = data.iter().map(|h| h.wind_gust_10m).fold(0.0_f64, f64::max) + 2.0;
    let uv_max   = data.iter().map(|h| h.uv_index).fold(0.0_f64, f64::max).max(1.0);

    let day_w:  usize = 18;
    let hour_w: usize = 6;
    const MIN_BAR: usize = 3;

    let mut n_cols = DRONE_COL_DEFS.len();
    loop {
        let needed: usize = day_w + hour_w
            + DRONE_COL_DEFS[..n_cols].iter().map(|(_, lw, _)| 1 + lw + 1 + MIN_BAR).sum::<usize>();
        if needed <= term_w || n_cols == 1 { break; }
        n_cols -= 1;
    }

    let active = &DRONE_COL_DEFS[..n_cols];
    let fixed: usize = day_w + hour_w + active.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = active.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = active.iter().map(|(_, _, bw)| {
        ((bw * available) / default_total.max(1)).max(MIN_BAR)
    }).collect();
    let used: usize = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w { bar_ws[n_cols - 1] += term_w - used; }

    let sep_w = fixed + bar_ws.iter().sum::<usize>();

    // Offset to wind columns (col 2 = W 10m); cols 0 and 1 precede them.
    let wind_chart_offset = day_w + hour_w
        + (0..2.min(n_cols)).map(|i| 1 + active[i].1 + 1 + bar_ws[i]).sum::<usize>();

    let hdr_sty = Style::default().add_modifier(Modifier::BOLD);
    let dim_sty = Style::default().fg(Color::DarkGray);

    let mut lines: Vec<Line> = Vec::new();

    let hdr_col = |lw: usize, bw: usize, title: &str| format!(" {:<width$}", title, width = lw + 1 + bw);
    let mut hdr_spans = vec![
        Span::styled(format!("{:<day_w$}", "DAY SUMMARY"), hdr_sty),
        Span::raw(format!("{:hour_w$}", "")),
    ];
    for (i, (_, lw, _)) in active.iter().enumerate() {
        hdr_spans.push(Span::styled(hdr_col(*lw, bar_ws[i], drone_col_title(i, units)), hdr_sty));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
    let mut current_sunrise = epoch;
    let mut current_sunset  = epoch;
    let mut day_summary_idx: usize;
    let mut current_date: Option<chrono::NaiveDate> = None;
    let mut day_row_count = 0usize;
    let mut day_parts_cache: Vec<(String, String)> = Vec::new();

    for hd in data {
        let hour = hd.time.hour();
        let date = hd.time.date();

        if current_date != Some(date) {
            if let Some(prev_date) = current_date {
                push_wind_chart(&mut lines, data, prev_date, wind_chart_offset, wind_max, theme, dim_sty);
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));
            }
            current_date = Some(date);
            day_row_count = 0;
            day_summary_idx = dates.iter().position(|d| *d == date).unwrap_or(0);
            current_sunrise = summaries[day_summary_idx].sunrise;
            current_sunset  = summaries[day_summary_idx].sunset;
            day_parts_cache = drone_summary_parts(&summaries[day_summary_idx], units);
        }

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut spans: Vec<Span> = if day_row_count < day_parts_cache.len() {
            let (label, value) = &day_parts_cache[day_row_count];
            if day_row_count == 1 {
                vec![Span::styled(format!("{:<day_w$}", label),
                    Style::default().fg(palette(0.0, theme)).add_modifier(Modifier::BOLD))]
            } else if value.is_empty() {
                vec![Span::styled(format!("{:<day_w$}", label), bold)]
            } else {
                vec![Span::raw(label.clone()), Span::styled(value.clone(), bold), Span::raw(" ")]
            }
        } else {
            vec![Span::raw(format!("{:<day_w$}", ""))]
        };
        day_row_count += 1;

        let h_style = if hd.time >= current_sunrise && hd.time < current_sunset {
            Style::default().fg(palette(0.5, theme))
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!("{:02}:00 ", hour), h_style));

        let arrows = [
            wind_arrow(hd.wind_dir_10m),
            wind_arrow(hd.wind_dir_80m),
            wind_arrow(hd.wind_dir_120m),
            wind_arrow(hd.wind_dir_180m),
        ];
        let any_shear = arrows[1..].iter().any(|&a| a != arrows[0]);

        for (i, _) in active.iter().enumerate() {
            let bw = bar_ws[i];
            spans.push(Span::raw(" "));
            match i {
                0 => {
                    let dt = if units.use_fahrenheit() { c_to_f(hd.temp) } else { hd.temp };
                    let c = temp_color(hd.temp, theme);
                    spans.push(Span::styled(format!("{:>5.1}", dt), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(hd.temp, temp_min, temp_max, bw, c));
                }
                1 => {
                    let prob_t = hd.precip_prob / 100.0;
                    let mm_t   = (hd.precip / max_mm).clamp(0.0, 1.0);
                    let c = palette((prob_t * 0.5 + mm_t * 0.5).clamp(0.0, 1.0), theme);
                    let mm_disp  = if units.use_inches() { mm_to_in(hd.precip) } else { hd.precip };
                    let max_disp = if units.use_inches() { mm_to_in(max_mm) }    else { max_mm };
                    let label = if units.use_inches() {
                        format!("{:>3.0}%/{:>4.2}", hd.precip_prob, mm_disp)
                    } else {
                        format!("{:>3.0}%/{:>4.1}", hd.precip_prob, mm_disp)
                    };
                    spans.push(Span::styled(label, Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(rain_block_cell(hd.precip_prob, mm_disp, max_disp, bw, c));
                }
                2 | 3 | 4 | 5 => {
                    let (speed, dir) = match i {
                        2 => (hd.wind_speed_10m,  hd.wind_dir_10m),
                        3 => (hd.wind_speed_80m,  hd.wind_dir_80m),
                        4 => (hd.wind_speed_120m, hd.wind_dir_120m),
                        _ => (hd.wind_speed_180m, hd.wind_dir_180m),
                    };
                    let blinks = any_shear;
                    let disp  = if units.use_mph() { kmh_to_mph(speed) } else { speed };
                    let arrow = wind_arrow(dir);
                    let c = wind_color(speed, theme);
                    let num_sty = Style::default().fg(c);
                    let arr_sty = if blinks {
                        Style::default().fg(c).add_modifier(Modifier::BOLD)
                    } else {
                        num_sty
                    };
                    spans.push(Span::styled(format!("{:>5.1}", disp), num_sty));
                    spans.push(Span::styled(arrow.to_string(), arr_sty));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(speed, 0.0, wind_max, bw, c));
                }
                6 => {
                    let disp = if units.use_mph() { kmh_to_mph(hd.wind_gust_10m) } else { hd.wind_gust_10m };
                    let c = wind_color(hd.wind_gust_10m, theme);
                    spans.push(Span::styled(format!("{:>5.1}", disp), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(hd.wind_gust_10m, 0.0, gust_max, bw, c));
                }
                _ => {
                    let c = palette((hd.uv_index / 11.0).clamp(0.0, 1.0), theme);
                    spans.push(Span::styled(format!("{:>4.1}", hd.uv_index), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(hd.uv_index, 0.0, uv_max, bw, c));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    if let Some(last_date) = current_date {
        push_wind_chart(&mut lines, data, last_date, wind_chart_offset, wind_max, theme, dim_sty);
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

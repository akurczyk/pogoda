use ratatui::style::Color;
use std::io::{self, Write as IoWrite};

use crate::colors::{palette, pressure_color, temp_color, wind_color};
use crate::types::{HourlyData, Theme, Units};
use crate::units::{c_to_f, hpa_to_inhg, kmh_to_mph, mm_to_in};
use crate::render::write_colored;

/// Place `label` at `col` only if every slot is still the background char.
fn try_place(chars: &mut Vec<char>, col: usize, label: &str, bg: char) {
    let lc: Vec<char> = label.chars().collect();
    let end = (col + lc.len()).min(chars.len());
    if col < chars.len() && chars[col..end].iter().all(|&c| c == bg) {
        for (j, &ch) in lc[..end - col].iter().enumerate() { chars[col + j] = ch; }
    }
}

pub fn print_one_chart(
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
    hour_ruler: &[char],
    title_connector: char,
    mono: bool,
) -> io::Result<()> {
    const BLOCKS: &[&str] = &[" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let total_sub = (chart_h * 8) as f64;
    let p_range = (p_max - p_min).max(0.001);

    let dim   = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };

    // Title — centered, with connector at label_w
    let title_inner = format!(" {} ", title);
    let total = term_w.saturating_sub(title_inner.chars().count());
    let left  = total / 2;
    let mut title_line: Vec<char> = vec!['─'; term_w];
    for (j, ch) in title_inner.chars().enumerate() {
        if left + j < term_w { title_line[left + j] = ch; }
    }
    if label_w < term_w { title_line[label_w] = title_connector; }
    write!(out, "{dim}{}{reset}\n", title_line.iter().collect::<String>())?;

    for r in 0..chart_h {
        let t = (chart_h - 1 - r) as f64 / (chart_h - 1).max(1) as f64;
        let label = fmt(p_min + t * (p_max - p_min));
        write!(out, "{dim}{:>label_w$}│{reset}", label)?;

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
                write_colored(out, BLOCKS[s_frac], s_color, mono)?;
            } else if p_ch != " " {
                write_colored(out, p_ch, p_color(p_val), mono)?;
            } else {
                write!(out, " ")?;
            }
        }
        writeln!(out)?;
    }

    // Hour ruler at the bottom of the chart
    write!(out, "{dim}{}├", " ".repeat(label_w))?;
    writeln!(out, "{}{reset}", hour_ruler.iter().collect::<String>())?;

    Ok(())
}

pub fn print_overview(out: &mut impl IoWrite, data: &[HourlyData], term_w: usize, units: Units, theme: Theme, chart_h: usize, mono: bool) -> io::Result<()> {
    use chrono::Timelike;
    if data.is_empty() { return Ok(()); }

    let label_w: usize = 10;
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

    let tf = |v: f64| if units.use_fahrenheit() { format!("{:.1}°F", c_to_f(v))      } else { format!("{:.1}°C", v)    };
    let wf = |v: f64| if units.use_mph()         { format!("{:.1}mph", kmh_to_mph(v)) } else { format!("{:.1}k/h", v)   };
    let rf = |v: f64| if units.use_inches()       { format!("{:.2}in", mm_to_in(v))   } else { format!("{:.1}mm", v)    };
    let pf = |v: f64| if units.use_inhg()         { format!("{:.2}in", hpa_to_inhg(v))} else { format!("{:.1}hPa", v)   };

    // Build the hour ruler once — reused in every chart header and the bottom axis.
    let mut hour_ruler: Vec<char> = vec!['─'; chart_w];
    let cols_per_hour = chart_w as f64 / n as f64;
    for &iv in &[1u32, 2, 3, 4, 6, 8, 12] {
        if cols_per_hour * iv as f64 >= 3.0 {
            for (di, hd) in data.iter().enumerate() {
                let h = hd.time.hour();
                if h != 0 && h % iv == 0 {
                    let col = di * chart_w / n;
                    try_place(&mut hour_ruler, col, &format!("{:02}", h), '─');
                }
            }
            break;
        }
    }

    let r = &hour_ruler;
    print_one_chart(out, if units.use_fahrenheit() { "TEMP °F" } else { "TEMP °C" },
        &temps, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┬', mono)?;

    print_one_chart(out, if units.use_fahrenheit() { "FEEL °F" } else { "FEEL °C" },
        &feels, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, "CLOUD %",
        &clouds, None, 0.0, 100.0,
        chart_h, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|_| Color::DarkGray, Color::White, r, '┼', mono)?;

    print_one_chart(out, "RAIN %",
        &rain_p, None, 0.0, 100.0,
        chart_h, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|v| palette(v / 100.0, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_inches() { "RAIN in" } else { "RAIN mm" },
        &rain_m, None, 0.0, rain_max,
        chart_h, label_w, chart_w, term_w,
        &|v| rf(v), &|v| palette((v / rain_max).clamp(0.0, 1.0), theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_mph() { "WIND mph" } else { "WIND km/h" },
        &winds, None, 0.0, wind_max,
        chart_h, label_w, chart_w, term_w,
        &|v| wf(v), &|v| wind_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_mph() { "GUSTS mph" } else { "GUSTS km/h" },
        &gusts, None, 0.0, wind_max,
        chart_h, label_w, chart_w, term_w,
        &|v| wf(v), &|v| wind_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_inhg() { "PRES inHg" } else { "PRESSURE hPa" },
        &press, None, press_min, press_max,
        chart_h, label_w, chart_w, term_w,
        &|v| pf(v), &|v| pressure_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, "HUMIDITY %",
        &humid, None, 0.0, 100.0,
        chart_h, label_w, chart_w, term_w,
        &|v| format!("{:.0}%", v), &|v| palette(v / 100.0, theme), Color::White, r, '┼', mono)?;

    // Day names below the last chart's ruler, on a ─ background
    let dim   = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };

    write!(out, "{dim}{}├", " ".repeat(label_w))?;
    let mut day_chars: Vec<char> = vec!['─'; chart_w];
    for (di, hd) in data.iter().enumerate() {
        if hd.time.hour() == 0 {
            let col = di * chart_w / n;
            try_place(&mut day_chars, col, &hd.time.format("%a %d").to_string(), '─');
        }
    }
    writeln!(out, "{}{reset}", day_chars.iter().collect::<String>())?;
    writeln!(out)?;
    Ok(())
}

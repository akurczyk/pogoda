use std::io::{self, Write as IoWrite};
use chrono::Datelike;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::colors::{palette, temp_color, wind_color};
use crate::render::{bars::{dual_bar, value_bar}, emit_span};
use crate::render::charts::{print_one_chart, try_place};
use crate::types::{HistoricalDailyData, HistoricalMonthlyData, Theme, Units};
use crate::units::{c_to_f, kmh_to_mph, mm_to_in};

const MONTH_NAMES: &[&str] = &[
    "", "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// ── Charts ────────────────────────────────────────────────────────────────────

pub fn print_historical_daily_charts(
    out: &mut impl IoWrite,
    data: &[HistoricalDailyData],
    term_w: usize,
    units: Units,
    theme: Theme,
    chart_h: usize,
    mono: bool,
) -> io::Result<()> {
    if data.is_empty() { return Ok(()); }

    let label_w: usize = 10;
    let chart_w = term_w.saturating_sub(label_w + 1);
    let n = data.len();

    let max_temps: Vec<f64> = data.iter().map(|d| d.max_temp).collect();
    let min_temps: Vec<f64> = data.iter().map(|d| d.min_temp).collect();
    let precip:    Vec<f64> = data.iter().map(|d| d.precip_sum).collect();
    let winds:     Vec<f64> = data.iter().map(|d| d.wind_max).collect();
    let gusts:     Vec<f64> = data.iter().map(|d| d.gust_max).collect();

    let temp_min = min_temps.iter().cloned().fold(f64::INFINITY, f64::min) - 1.0;
    let temp_max = max_temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let rain_max = precip.iter().cloned().fold(0.0_f64, f64::max).max(0.1);
    let wind_max = gusts.iter().chain(winds.iter()).cloned().fold(0.0_f64, f64::max) + 1.0;

    let tf = |v: f64| if units.use_fahrenheit() { format!("{:.1}°F", c_to_f(v)) } else { format!("{:.1}°C", v) };
    let wf = |v: f64| if units.use_mph() { format!("{:.1}mph", kmh_to_mph(v)) } else { format!("{:.1}k/h", v) };
    let rf = |v: f64| if units.use_inches() { format!("{:.2}in", mm_to_in(v)) } else { format!("{:.1}mm", v) };

    let plain_ruler: Vec<char> = vec!['─'; chart_w];
    let r = &plain_ruler;

    print_one_chart(out, if units.use_fahrenheit() { "TEMP MAX °F" } else { "TEMP MAX °C" },
        &max_temps, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┬', mono)?;

    print_one_chart(out, if units.use_fahrenheit() { "TEMP MIN °F" } else { "TEMP MIN °C" },
        &min_temps, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_inches() { "RAIN in" } else { "RAIN mm" },
        &precip, None, 0.0, rain_max,
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

    // Bottom axis: month/year labels
    let dim   = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    write!(out, "{dim}{}└", " ".repeat(label_w))?;
    let mut axis: Vec<char> = vec!['─'; chart_w];
    let mut last_month = 0u32;
    for (di, d) in data.iter().enumerate() {
        if d.date.month() != last_month {
            let col = di * chart_w / n;
            try_place(&mut axis, col,
                &format!("{} {}", MONTH_NAMES[d.date.month() as usize], d.date.year()), '─');
            last_month = d.date.month();
        }
    }
    writeln!(out, "{}{reset}", axis.iter().collect::<String>())?;
    writeln!(out)?;
    Ok(())
}

pub fn print_historical_monthly_charts(
    out: &mut impl IoWrite,
    data: &[HistoricalMonthlyData],
    term_w: usize,
    units: Units,
    theme: Theme,
    chart_h: usize,
    mono: bool,
) -> io::Result<()> {
    if data.is_empty() { return Ok(()); }

    let label_w: usize = 10;
    let chart_w = term_w.saturating_sub(label_w + 1);
    let n = data.len();

    let max_t:  Vec<f64> = data.iter().map(|m| m.avg_max_temp).collect();
    let min_t:  Vec<f64> = data.iter().map(|m| m.avg_min_temp).collect();
    let precip: Vec<f64> = data.iter().map(|m| m.precip_sum).collect();
    let winds:  Vec<f64> = data.iter().map(|m| m.wind_max).collect();
    let gusts:  Vec<f64> = data.iter().map(|m| m.gust_max).collect();

    let temp_min = min_t.iter().cloned().fold(f64::INFINITY, f64::min) - 1.0;
    let temp_max = max_t.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let rain_max = precip.iter().cloned().fold(0.0_f64, f64::max).max(0.1);
    let wind_max = gusts.iter().chain(winds.iter()).cloned().fold(0.0_f64, f64::max) + 1.0;

    let tf = |v: f64| if units.use_fahrenheit() { format!("{:.1}°F", c_to_f(v)) } else { format!("{:.1}°C", v) };
    let wf = |v: f64| if units.use_mph() { format!("{:.1}mph", kmh_to_mph(v)) } else { format!("{:.1}k/h", v) };
    let rf = |v: f64| if units.use_inches() { format!("{:.2}in", mm_to_in(v)) } else { format!("{:.1}mm", v) };

    let plain_ruler: Vec<char> = vec!['─'; chart_w];
    let r = &plain_ruler;
    print_one_chart(out, if units.use_fahrenheit() { "TEMP MAX °F" } else { "TEMP MAX °C" },
        &max_t, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┬', mono)?;

    print_one_chart(out, if units.use_fahrenheit() { "TEMP MIN °F" } else { "TEMP MIN °C" },
        &min_t, None, temp_min, temp_max,
        chart_h, label_w, chart_w, term_w,
        &|v| tf(v), &|v| temp_color(v, theme), Color::White, r, '┼', mono)?;

    print_one_chart(out, if units.use_inches() { "RAIN in" } else { "RAIN mm" },
        &precip, None, 0.0, rain_max,
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

    let dim   = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    write!(out, "{dim}{}└", " ".repeat(label_w))?;
    let mut axis: Vec<char> = vec!['─'; chart_w];
    let cols_per_year = (chart_w as f64 / n as f64) * 12.0;
    let year_interval: i32 = if cols_per_year >= 5.0 { 1 }
        else if cols_per_year >= 3.0 { 2 }
        else if cols_per_year >= 2.0 { 5 }
        else if cols_per_year >= 1.0 { 10 }
        else { 20 };
    let first_year = data.first().map(|m| m.year).unwrap_or(0);
    let mut last_yr = 0i32;
    for (mi, m) in data.iter().enumerate() {
        if m.year != last_yr {
            if (m.year - first_year) % year_interval == 0 {
                let col = mi * chart_w / n;
                try_place(&mut axis, col, &format!("{}", m.year), '─');
            }
            last_yr = m.year;
        }
    }
    writeln!(out, "{}{reset}", axis.iter().collect::<String>())?;
    writeln!(out)?;
    Ok(())
}

// ── Tables ────────────────────────────────────────────────────────────────────

/// (header, label_w, default_bar_w)
const HIST_DAILY_COLS: &[(&str, usize, usize)] = &[
    ("TEMP °C",   11, 9),
    ("RAIN mm",    7, 8),
    ("WIND km/h", 11, 9),
];

fn hist_daily_col_title(i: usize, units: Units) -> &'static str {
    match i {
        0 => if units.use_fahrenheit() { "TEMP °F"   } else { "TEMP °C"   },
        1 => if units.use_inches()     { "RAIN in"   } else { "RAIN mm"   },
        2 => if units.use_mph()        { "WIND mph"  } else { "WIND km/h" },
        _ => HIST_DAILY_COLS[i].0,
    }
}

pub fn print_historical_daily_table(
    out: &mut impl IoWrite,
    data: &[HistoricalDailyData],
    term_w: usize,
    units: Units,
    theme: Theme,
    mono: bool,
) -> io::Result<()> {
    if data.is_empty() { return Ok(()); }

    let date_w: usize = 13;
    const MIN_BAR: usize = 3;
    let n_cols = HIST_DAILY_COLS.len();

    let fixed: usize = date_w + HIST_DAILY_COLS.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = HIST_DAILY_COLS.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = HIST_DAILY_COLS.iter().map(|(_, _, bw)| {
        ((bw * available) / default_total.max(1)).max(MIN_BAR)
    }).collect();
    let used = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w { bar_ws[n_cols - 1] += term_w - used; }
    let sep_w = fixed + bar_ws.iter().sum::<usize>();

    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let temp_min = data.iter().map(|d| d.min_temp).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|d| d.max_temp).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let rain_max = data.iter().map(|d| d.precip_sum).fold(0.0_f64, f64::max).max(0.1);
    let wind_max = data.iter().map(|d| d.gust_max.max(d.wind_max)).fold(0.0_f64, f64::max) + 2.0;

    let hdr_col = |lw: usize, bw: usize, title: &str| format!(" {:<width$}", title, width = lw + 1 + bw);

    let mut lines: Vec<Line> = Vec::new();

    let mut hdr_spans = vec![Span::styled(format!("{:<date_w$}", "DATE"), hdr)];
    for (i, (_, lw, _)) in HIST_DAILY_COLS.iter().enumerate() {
        hdr_spans.push(Span::styled(hdr_col(*lw, bar_ws[i], hist_daily_col_title(i, units)), hdr));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));

    let mut last_month = 0u32;
    for d in data {
        if d.date.month() != last_month {
            if last_month != 0 {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));
            }
            let label = format!("{} {}", MONTH_NAMES[d.date.month() as usize], d.date.year());
            lines.push(Line::from(vec![
                Span::styled(format!("{:<sep_w$}", label),
                    Style::default().fg(palette(0.0, theme)).add_modifier(Modifier::BOLD))
            ]));
            last_month = d.date.month();
        }

        let date_label = format!("{} {:02} {}",
            &crate::weather::day_name(d.date)[..3],
            d.date.day(),
            MONTH_NAMES[d.date.month() as usize]);
        let mut spans = vec![Span::styled(format!("{:<date_w$}", date_label), dim)];

        for (i, _) in HIST_DAILY_COLS.iter().enumerate() {
            let bw = bar_ws[i];
            spans.push(Span::raw(" "));
            match i {
                0 => {
                    let (max_d, min_d) = if units.use_fahrenheit() {
                        (c_to_f(d.max_temp), c_to_f(d.min_temp))
                    } else {
                        (d.max_temp, d.min_temp)
                    };
                    let c = temp_color((d.max_temp + d.min_temp) / 2.0, theme);
                    spans.push(Span::styled(format!("{:>5.1}/{:>5.1}", max_d, min_d), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    // ● marks max, ◆ marks min
                    spans.extend(dual_bar(d.max_temp, d.min_temp, temp_min, temp_max, bw, c));
                }
                1 => {
                    let disp  = if units.use_inches() { mm_to_in(d.precip_sum) } else { d.precip_sum };
                    let max_d = if units.use_inches() { mm_to_in(rain_max) }     else { rain_max };
                    let c = palette((disp / max_d).clamp(0.0, 1.0), theme);
                    let label = if units.use_inches() { format!("{:>7.3}", disp) } else { format!("{:>7.1}", disp) };
                    spans.push(Span::styled(label, Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(disp, 0.0, max_d, bw, c));
                }
                _ => {
                    let (ws, wg) = if units.use_mph() {
                        (kmh_to_mph(d.wind_max), kmh_to_mph(d.gust_max))
                    } else {
                        (d.wind_max, d.gust_max)
                    };
                    let c = wind_color(d.wind_max, theme);
                    spans.push(Span::styled(format!("{:>5.1}/{:>5.1}", ws, wg), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(d.wind_max, d.gust_max, 0.0, wind_max, bw, c));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    writeln!(out)?;
    for line in &lines { for span in &line.spans { emit_span(out, span, mono)?; } writeln!(out)?; }
    Ok(())
}

/// (header, label_w, default_bar_w)
const HIST_MON_COLS: &[(&str, usize, usize)] = &[
    ("TEMP °C",   11, 9),
    ("RAIN mm",    7, 8),
    ("WIND km/h", 11, 9),
];

fn hist_mon_col_title(i: usize, units: Units) -> &'static str {
    match i {
        0 => if units.use_fahrenheit() { "TEMP °F"  } else { "TEMP °C"   },
        1 => if units.use_inches()     { "RAIN in"  } else { "RAIN mm"   },
        2 => if units.use_mph()        { "WIND mph" } else { "WIND km/h" },
        _ => HIST_MON_COLS[i].0,
    }
}

pub fn print_historical_monthly_table(
    out: &mut impl IoWrite,
    data: &[HistoricalMonthlyData],
    term_w: usize,
    units: Units,
    theme: Theme,
    mono: bool,
) -> io::Result<()> {
    if data.is_empty() { return Ok(()); }

    let month_w: usize = 10;
    const MIN_BAR: usize = 3;
    let n_cols = HIST_MON_COLS.len();

    let fixed: usize = month_w + HIST_MON_COLS.iter().map(|(_, lw, _)| 1 + lw + 1).sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = HIST_MON_COLS.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = HIST_MON_COLS.iter().map(|(_, _, bw)| {
        ((bw * available) / default_total.max(1)).max(MIN_BAR)
    }).collect();
    let used = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w { bar_ws[n_cols - 1] += term_w - used; }
    let sep_w = fixed + bar_ws.iter().sum::<usize>();

    let hdr_sty = Style::default().add_modifier(Modifier::BOLD);
    let dim_sty = Style::default().fg(Color::DarkGray);

    let temp_min = data.iter().map(|m| m.avg_min_temp).fold(f64::INFINITY, f64::min) - 2.0;
    let temp_max = data.iter().map(|m| m.avg_max_temp).fold(f64::NEG_INFINITY, f64::max) + 2.0;
    let rain_max = data.iter().map(|m| m.precip_sum).fold(0.0_f64, f64::max).max(0.1);
    let wind_max = data.iter().map(|m| m.gust_max.max(m.wind_max)).fold(0.0_f64, f64::max) + 2.0;

    let hdr_col = |lw: usize, bw: usize, title: &str| format!(" {:<width$}", title, width = lw + 1 + bw);

    let mut lines: Vec<Line> = Vec::new();

    let mut hdr_spans = vec![Span::styled(format!("{:<month_w$}", "MONTH"), hdr_sty)];
    for (i, (_, lw, _)) in HIST_MON_COLS.iter().enumerate() {
        hdr_spans.push(Span::styled(hdr_col(*lw, bar_ws[i], hist_mon_col_title(i, units)), hdr_sty));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));

    let mut last_year = 0i32;
    for m in data {
        if m.year != last_year {
            if last_year != 0 {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));
            }
            lines.push(Line::from(vec![
                Span::styled(format!("{:<sep_w$}", m.year),
                    Style::default().fg(palette(0.0, theme)).add_modifier(Modifier::BOLD))
            ]));
            last_year = m.year;
        }

        let label = format!("{:<month_w$}", MONTH_NAMES[m.month as usize]);
        let mut spans = vec![Span::styled(label, dim_sty)];

        for (i, _) in HIST_MON_COLS.iter().enumerate() {
            let bw = bar_ws[i];
            spans.push(Span::raw(" "));
            match i {
                0 => {
                    let (max_d, min_d) = if units.use_fahrenheit() {
                        (c_to_f(m.avg_max_temp), c_to_f(m.avg_min_temp))
                    } else {
                        (m.avg_max_temp, m.avg_min_temp)
                    };
                    let c = temp_color((m.avg_max_temp + m.avg_min_temp) / 2.0, theme);
                    spans.push(Span::styled(format!("{:>5.1}/{:>5.1}", max_d, min_d), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    // ● marks avg_max, ◆ marks avg_min
                    spans.extend(dual_bar(m.avg_max_temp, m.avg_min_temp, temp_min, temp_max, bw, c));
                }
                1 => {
                    let disp  = if units.use_inches() { mm_to_in(m.precip_sum) } else { m.precip_sum };
                    let max_d = if units.use_inches() { mm_to_in(rain_max) }     else { rain_max };
                    let c = palette((disp / max_d).clamp(0.0, 1.0), theme);
                    let label = if units.use_inches() { format!("{:>7.3}", disp) } else { format!("{:>7.1}", disp) };
                    spans.push(Span::styled(label, Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(value_bar(disp, 0.0, max_d, bw, c));
                }
                _ => {
                    let (ws, wg) = if units.use_mph() {
                        (kmh_to_mph(m.wind_max), kmh_to_mph(m.gust_max))
                    } else {
                        (m.wind_max, m.gust_max)
                    };
                    let c = wind_color(m.wind_max, theme);
                    spans.push(Span::styled(format!("{:>5.1}/{:>5.1}", ws, wg), Style::default().fg(c)));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(m.wind_max, m.gust_max, 0.0, wind_max, bw, c));
                }
            }
        }
        lines.push(Line::from(spans));
    }

    writeln!(out)?;
    for line in &lines { for span in &line.spans { emit_span(out, span, mono)?; } writeln!(out)?; }
    Ok(())
}

// ── Footer ────────────────────────────────────────────────────────────────────

pub fn write_hist_footer(
    out: &mut impl IoWrite,
    api_url: &str,
    mono: bool,
    version: &str,
) -> io::Result<()> {
    let dim   = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    writeln!(out)?;
    write!(out, "{dim}")?;
    writeln!(out, "Data source: Open-Meteo Historical Weather API (archive-api.open-meteo.com)")?;
    writeln!(out, "API URL:     {api_url}")?;
    writeln!(out)?;
    writeln!(out, "Modifiers: --i-drone-you  --delorean  --strange-units  --yes-sir")?;
    writeln!(out, "           --i-am-blue  --color-me  --classic-colors  --rainforest  --i-cant-afford-cga")?;
    writeln!(out, "           --no-eyecandy  --high-charts  --no-charts  --no-table  --tabular-bells")?;
    writeln!(out)?;
    writeln!(out, "https://github.com/akurczyk/pogoda  v{version}")?;
    write!(out, "{reset}")?;
    Ok(())
}

use chrono::{Datelike, NaiveDate};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use rust_i18n::t;
use std::io::{self, Write as IoWrite};

use crate::colors::{palette, temp_color, wind_color};
use crate::render::charts::{print_one_chart, try_place};
use crate::render::{
    bars::{dual_bar, value_bar},
    emit_span,
};
use crate::types::{HistoricalDailyData, HistoricalMonthlyData, Theme, Units};
use crate::units::{c_to_f, kmh_to_mph, mm_to_in};

fn month_abbr(year: i32, month: u32) -> String {
    let loc = crate::locale::chrono_locale(&rust_i18n::locale());
    NaiveDate::from_ymd_opt(year, month, 1)
        .map(|d| d.format_localized("%b", loc).to_string())
        .unwrap_or_default()
}

// ── Axis helpers ─────────────────────────────────────────────────────────────

fn write_daily_axis(
    out: &mut impl IoWrite,
    data: &[HistoricalDailyData],
    label_w: usize,
    chart_w: usize,
    connector: char,
    mono: bool,
) -> io::Result<()> {
    let dim = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    let n = data.len();
    if n == 0 {
        return Ok(());
    }
    write!(out, "{dim}{}{connector}", " ".repeat(label_w))?;
    let mut axis: Vec<char> = vec!['─'; chart_w];
    let mut last_month = 0u32;
    for (di, d) in data.iter().enumerate() {
        if d.date.month() != last_month {
            let col = di * chart_w / n;
            try_place(
                &mut axis,
                col,
                &format!(
                    "{} {}",
                    month_abbr(d.date.year(), d.date.month()),
                    d.date.year()
                ),
                '─',
            );
            last_month = d.date.month();
        }
    }
    writeln!(out, "{}{reset}", axis.iter().collect::<String>())
}

fn write_monthly_axis(
    out: &mut impl IoWrite,
    data: &[HistoricalMonthlyData],
    label_w: usize,
    chart_w: usize,
    connector: char,
    mono: bool,
) -> io::Result<()> {
    let dim = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    let n = data.len();
    if n == 0 {
        return Ok(());
    }
    write!(out, "{dim}{}{connector}", " ".repeat(label_w))?;
    let mut axis: Vec<char> = vec!['─'; chart_w];
    let cols_per_year = (chart_w as f64 / n as f64) * 12.0;
    let year_interval: i32 = if cols_per_year >= 5.0 {
        1
    } else if cols_per_year >= 3.0 {
        2
    } else if cols_per_year >= 2.0 {
        5
    } else if cols_per_year >= 1.0 {
        10
    } else {
        20
    };
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
    writeln!(out, "{}{reset}", axis.iter().collect::<String>())
}

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
    if data.is_empty() {
        return Ok(());
    }

    let label_w: usize = 10;
    let chart_w = term_w.saturating_sub(label_w + 1);

    let max_temps: Vec<f64> = data.iter().map(|d| d.max_temp).collect();
    let min_temps: Vec<f64> = data.iter().map(|d| d.min_temp).collect();
    let precip: Vec<f64> = data.iter().map(|d| d.precip_sum).collect();
    let winds: Vec<f64> = data.iter().map(|d| d.wind_max).collect();
    let gusts: Vec<f64> = data.iter().map(|d| d.gust_max).collect();

    let temp_min = min_temps.iter().cloned().fold(f64::INFINITY, f64::min) - 1.0;
    let temp_max = max_temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let rain_max = precip.iter().cloned().fold(0.0_f64, f64::max).max(0.1);
    let wind_max = gusts
        .iter()
        .chain(winds.iter())
        .cloned()
        .fold(0.0_f64, f64::max)
        + 1.0;

    let tf = |v: f64| {
        if units.use_fahrenheit() {
            format!("{:.1}°F", c_to_f(v))
        } else {
            format!("{:.1}°C", v)
        }
    };
    let wf = |v: f64| {
        if units.use_mph() {
            format!("{:.1}mph", kmh_to_mph(v))
        } else {
            format!("{:.1}km/h", v)
        }
    };
    let rf = |v: f64| {
        if units.use_inches() {
            format!("{:.2}in", mm_to_in(v))
        } else {
            format!("{:.1}mm", v)
        }
    };

    let plain_ruler: Vec<char> = vec!['─'; chart_w];
    let r = &plain_ruler;

    let title_tmax = format!("{} °{}", t!("chart.temp_max"), units.temp_label());
    print_one_chart(
        out,
        &title_tmax,
        &max_temps,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┬',
        false,
        mono,
    )?;
    write_daily_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_tmin = format!("{} °{}", t!("chart.temp_min"), units.temp_label());
    print_one_chart(
        out,
        &title_tmin,
        &min_temps,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_daily_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_rain = format!("{} {}", t!("chart.rain"), units.rain_label());
    print_one_chart(
        out,
        &title_rain,
        &precip,
        None,
        0.0,
        rain_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| rf(v),
        &|v| palette((v / rain_max).clamp(0.0, 1.0), theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_daily_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_wind = format!("{} {}", t!("chart.wind"), units.wind_label());
    print_one_chart(
        out,
        &title_wind,
        &winds,
        None,
        0.0,
        wind_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| wf(v),
        &|v| wind_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_daily_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_gusts = format!("{} {}", t!("chart.gusts"), units.wind_label());
    print_one_chart(
        out,
        &title_gusts,
        &gusts,
        None,
        0.0,
        wind_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| wf(v),
        &|v| wind_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_daily_axis(out, data, label_w, chart_w, '└', mono)?;

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
    if data.is_empty() {
        return Ok(());
    }

    let label_w: usize = 10;
    let chart_w = term_w.saturating_sub(label_w + 1);

    let max_t: Vec<f64> = data.iter().map(|m| m.avg_max_temp).collect();
    let min_t: Vec<f64> = data.iter().map(|m| m.avg_min_temp).collect();
    let ext_max: Vec<f64> = data.iter().map(|m| m.extreme_max_temp).collect();
    let ext_min: Vec<f64> = data.iter().map(|m| m.extreme_min_temp).collect();
    let precip: Vec<f64> = data.iter().map(|m| m.precip_sum).collect();
    let winds: Vec<f64> = data.iter().map(|m| m.wind_max).collect();
    let gusts: Vec<f64> = data.iter().map(|m| m.gust_max).collect();

    let temp_min = ext_min
        .iter()
        .chain(min_t.iter())
        .cloned()
        .fold(f64::INFINITY, f64::min)
        - 1.0;
    let temp_max = ext_max
        .iter()
        .chain(max_t.iter())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max)
        + 1.0;
    let rain_max = precip.iter().cloned().fold(0.0_f64, f64::max).max(0.1);
    let wind_max = gusts
        .iter()
        .chain(winds.iter())
        .cloned()
        .fold(0.0_f64, f64::max)
        + 1.0;

    let tf = |v: f64| {
        if units.use_fahrenheit() {
            format!("{:.1}°F", c_to_f(v))
        } else {
            format!("{:.1}°C", v)
        }
    };
    let wf = |v: f64| {
        if units.use_mph() {
            format!("{:.1}mph", kmh_to_mph(v))
        } else {
            format!("{:.1}km/h", v)
        }
    };
    let rf = |v: f64| {
        if units.use_inches() {
            format!("{:.2}in", mm_to_in(v))
        } else {
            format!("{:.1}mm", v)
        }
    };

    let plain_ruler: Vec<char> = vec!['─'; chart_w];
    let r = &plain_ruler;
    let title_avg_tmax = format!("{} °{}", t!("chart.avg_tmax"), units.temp_label());
    print_one_chart(
        out,
        &title_avg_tmax,
        &max_t,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┬',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_avg_tmin = format!("{} °{}", t!("chart.avg_tmin"), units.temp_label());
    print_one_chart(
        out,
        &title_avg_tmin,
        &min_t,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_ext_tmax = format!("{} °{}", t!("chart.ext_tmax"), units.temp_label());
    print_one_chart(
        out,
        &title_ext_tmax,
        &ext_max,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_ext_tmin = format!("{} °{}", t!("chart.ext_tmin"), units.temp_label());
    print_one_chart(
        out,
        &title_ext_tmin,
        &ext_min,
        None,
        temp_min,
        temp_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| tf(v),
        &|v| temp_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_rain_mon = format!("{} {}", t!("chart.rain"), units.rain_label());
    print_one_chart(
        out,
        &title_rain_mon,
        &precip,
        None,
        0.0,
        rain_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| rf(v),
        &|v| palette((v / rain_max).clamp(0.0, 1.0), theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_wind_mon = format!("{} {}", t!("chart.wind"), units.wind_label());
    print_one_chart(
        out,
        &title_wind_mon,
        &winds,
        None,
        0.0,
        wind_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| wf(v),
        &|v| wind_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '├', mono)?;

    let title_gusts_mon = format!("{} {}", t!("chart.gusts"), units.wind_label());
    print_one_chart(
        out,
        &title_gusts_mon,
        &gusts,
        None,
        0.0,
        wind_max,
        chart_h,
        label_w,
        chart_w,
        term_w,
        &|v| wf(v),
        &|v| wind_color(v, theme),
        Color::White,
        r,
        '┼',
        false,
        mono,
    )?;
    write_monthly_axis(out, data, label_w, chart_w, '└', mono)?;

    writeln!(out)?;
    Ok(())
}

// ── Tables ────────────────────────────────────────────────────────────────────

/// (header, label_w, default_bar_w)
const HIST_DAILY_COLS: &[(&str, usize, usize)] =
    &[("TEMP °C", 11, 9), ("RAIN mm", 7, 8), ("WIND km/h", 11, 9)];

fn hist_daily_col_title(i: usize, units: Units) -> String {
    match i {
        0 => format!("{} °{}", t!("col.temp"), units.temp_label()),
        1 => format!("{} {}", t!("col.rain"), units.rain_label()),
        2 => format!("{} {}", t!("col.wind"), units.wind_label()),
        _ => HIST_DAILY_COLS[i].0.to_string(),
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
    if data.is_empty() {
        return Ok(());
    }

    let date_w: usize = 13;
    const MIN_BAR: usize = 3;
    let n_cols = HIST_DAILY_COLS.len();

    let fixed: usize = date_w
        + HIST_DAILY_COLS
            .iter()
            .map(|(_, lw, _)| 1 + lw + 1)
            .sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = HIST_DAILY_COLS.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = HIST_DAILY_COLS
        .iter()
        .map(|(_, _, bw)| ((bw * available) / default_total.max(1)).max(MIN_BAR))
        .collect();
    let used = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w {
        bar_ws[n_cols - 1] += term_w - used;
    }
    let sep_w = fixed + bar_ws.iter().sum::<usize>();

    let hdr = Style::default().add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let temp_min = data
        .iter()
        .map(|d| d.min_temp)
        .fold(f64::INFINITY, f64::min)
        - 2.0;
    let temp_max = data
        .iter()
        .map(|d| d.max_temp)
        .fold(f64::NEG_INFINITY, f64::max)
        + 2.0;
    let rain_max = data
        .iter()
        .map(|d| d.precip_sum)
        .fold(0.0_f64, f64::max)
        .max(0.1);
    let wind_max = data
        .iter()
        .map(|d| d.gust_max.max(d.wind_max))
        .fold(0.0_f64, f64::max)
        + 2.0;

    let hdr_col =
        |lw: usize, bw: usize, title: &str| format!(" {:<width$}", title, width = lw + 1 + bw);

    let mut lines: Vec<Line> = Vec::new();

    let mut hdr_spans = vec![Span::styled(format!("{:<date_w$}", t!("col.date")), hdr)];
    for (i, (_, lw, _)) in HIST_DAILY_COLS.iter().enumerate() {
        hdr_spans.push(Span::styled(
            hdr_col(*lw, bar_ws[i], &hist_daily_col_title(i, units)),
            hdr,
        ));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));

    let mut last_month = 0u32;
    for d in data {
        if d.date.month() != last_month {
            if last_month != 0 {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim)));
            }
            let label = format!(
                "{} {}",
                month_abbr(d.date.year(), d.date.month()),
                d.date.year()
            );
            lines.push(Line::from(vec![Span::styled(
                format!("{:<sep_w$}", label),
                Style::default()
                    .fg(palette(0.0, theme))
                    .add_modifier(Modifier::BOLD),
            )]));
            last_month = d.date.month();
        }

        let loc = crate::locale::chrono_locale(&rust_i18n::locale());
        let date_label = format!(
            "{} {:02} {}",
            d.date.format_localized("%a", loc),
            d.date.day(),
            month_abbr(d.date.year(), d.date.month())
        );
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
                    spans.push(Span::styled(
                        format!("{:>5.1}/{:>5.1}", max_d, min_d),
                        Style::default().fg(c),
                    ));
                    spans.push(Span::raw(" "));
                    // ● marks max, ◆ marks min
                    spans.extend(dual_bar(d.max_temp, d.min_temp, temp_min, temp_max, bw, c));
                }
                1 => {
                    let disp = if units.use_inches() {
                        mm_to_in(d.precip_sum)
                    } else {
                        d.precip_sum
                    };
                    let max_d = if units.use_inches() {
                        mm_to_in(rain_max)
                    } else {
                        rain_max
                    };
                    let c = palette((disp / max_d).clamp(0.0, 1.0), theme);
                    let label = if units.use_inches() {
                        format!("{:>7.3}", disp)
                    } else {
                        format!("{:>7.1}", disp)
                    };
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
                    spans.push(Span::styled(
                        format!("{:>5.1}/{:>5.1}", ws, wg),
                        Style::default().fg(c),
                    ));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(d.wind_max, d.gust_max, 0.0, wind_max, bw, c));
                }
            }
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

/// (header, label_w, default_bar_w)
const HIST_MON_COLS: &[(&str, usize, usize)] = &[
    ("AVG TEMP °C", 11, 9),
    ("EXT TEMP °C", 11, 9),
    ("RAIN mm", 7, 8),
    ("WIND km/h", 11, 9),
];

fn hist_mon_col_title(i: usize, units: Units) -> String {
    match i {
        0 => format!("{} °{}", t!("col.avg_temp"), units.temp_label()),
        1 => format!("{} °{}", t!("col.ext_temp"), units.temp_label()),
        2 => format!("{} {}", t!("col.rain"), units.rain_label()),
        3 => format!("{} {}", t!("col.wind"), units.wind_label()),
        _ => HIST_MON_COLS[i].0.to_string(),
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
    if data.is_empty() {
        return Ok(());
    }

    let month_w: usize = 10;
    const MIN_BAR: usize = 3;
    let n_cols = HIST_MON_COLS.len();

    let fixed: usize = month_w
        + HIST_MON_COLS
            .iter()
            .map(|(_, lw, _)| 1 + lw + 1)
            .sum::<usize>();
    let available = term_w.saturating_sub(fixed);
    let default_total: usize = HIST_MON_COLS.iter().map(|(_, _, bw)| bw).sum();
    let mut bar_ws: Vec<usize> = HIST_MON_COLS
        .iter()
        .map(|(_, _, bw)| ((bw * available) / default_total.max(1)).max(MIN_BAR))
        .collect();
    let used = fixed + bar_ws.iter().sum::<usize>();
    if used < term_w {
        bar_ws[n_cols - 1] += term_w - used;
    }
    let sep_w = fixed + bar_ws.iter().sum::<usize>();

    let hdr_sty = Style::default().add_modifier(Modifier::BOLD);
    let dim_sty = Style::default().fg(Color::DarkGray);

    let temp_min = data
        .iter()
        .map(|m| m.extreme_min_temp.min(m.avg_min_temp))
        .fold(f64::INFINITY, f64::min)
        - 2.0;
    let temp_max = data
        .iter()
        .map(|m| m.extreme_max_temp.max(m.avg_max_temp))
        .fold(f64::NEG_INFINITY, f64::max)
        + 2.0;
    let rain_max = data
        .iter()
        .map(|m| m.precip_sum)
        .fold(0.0_f64, f64::max)
        .max(0.1);
    let wind_max = data
        .iter()
        .map(|m| m.gust_max.max(m.wind_max))
        .fold(0.0_f64, f64::max)
        + 2.0;

    let hdr_col =
        |lw: usize, bw: usize, title: &str| format!(" {:<width$}", title, width = lw + 1 + bw);

    let mut lines: Vec<Line> = Vec::new();

    let mut hdr_spans = vec![Span::styled(
        format!("{:<month_w$}", t!("col.month")),
        hdr_sty,
    )];
    for (i, (_, lw, _)) in HIST_MON_COLS.iter().enumerate() {
        hdr_spans.push(Span::styled(
            hdr_col(*lw, bar_ws[i], &hist_mon_col_title(i, units)),
            hdr_sty,
        ));
    }
    lines.push(Line::from(hdr_spans));
    lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));

    let mut last_year = 0i32;
    for m in data {
        if m.year != last_year {
            if last_year != 0 {
                lines.push(Line::from(Span::styled("─".repeat(sep_w), dim_sty)));
            }
            lines.push(Line::from(vec![Span::styled(
                format!("{:<sep_w$}", m.year),
                Style::default()
                    .fg(palette(0.0, theme))
                    .add_modifier(Modifier::BOLD),
            )]));
            last_year = m.year;
        }

        let label = format!("{:<month_w$}", month_abbr(m.year, m.month));
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
                    spans.push(Span::styled(
                        format!("{:>5.1}/{:>5.1}", max_d, min_d),
                        Style::default().fg(c),
                    ));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(
                        m.avg_max_temp,
                        m.avg_min_temp,
                        temp_min,
                        temp_max,
                        bw,
                        c,
                    ));
                }
                1 => {
                    let (max_d, min_d) = if units.use_fahrenheit() {
                        (c_to_f(m.extreme_max_temp), c_to_f(m.extreme_min_temp))
                    } else {
                        (m.extreme_max_temp, m.extreme_min_temp)
                    };
                    let c = temp_color((m.extreme_max_temp + m.extreme_min_temp) / 2.0, theme);
                    spans.push(Span::styled(
                        format!("{:>5.1}/{:>5.1}", max_d, min_d),
                        Style::default().fg(c),
                    ));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(
                        m.extreme_max_temp,
                        m.extreme_min_temp,
                        temp_min,
                        temp_max,
                        bw,
                        c,
                    ));
                }
                2 => {
                    let disp = if units.use_inches() {
                        mm_to_in(m.precip_sum)
                    } else {
                        m.precip_sum
                    };
                    let max_d = if units.use_inches() {
                        mm_to_in(rain_max)
                    } else {
                        rain_max
                    };
                    let c = palette((disp / max_d).clamp(0.0, 1.0), theme);
                    let label = if units.use_inches() {
                        format!("{:>7.3}", disp)
                    } else {
                        format!("{:>7.1}", disp)
                    };
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
                    spans.push(Span::styled(
                        format!("{:>5.1}/{:>5.1}", ws, wg),
                        Style::default().fg(c),
                    ));
                    spans.push(Span::raw(" "));
                    spans.extend(dual_bar(m.wind_max, m.gust_max, 0.0, wind_max, bw, c));
                }
            }
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

// ── Footer ────────────────────────────────────────────────────────────────────

pub fn write_hist_footer(
    out: &mut impl IoWrite,
    api_url: &str,
    mono: bool,
    version: &str,
) -> io::Result<()> {
    let dim = if mono { "" } else { "\x1b[90m" };
    let reset = if mono { "" } else { "\x1b[0m" };
    let mods = t!("usage.modifiers_title");
    let indent = " ".repeat(mods.chars().count() + 1);
    writeln!(out)?;
    write!(out, "{dim}")?;
    writeln!(out, "{}", t!("footer.data_source_hist"))?;
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
    writeln!(out, "{}  v{version}", t!("footer.github"))?;
    write!(out, "{reset}")?;
    Ok(())
}

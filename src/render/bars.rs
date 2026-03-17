use ratatui::{style::{Color, Modifier, Style}, text::Span};

use crate::colors::{temp_color, wind_color};
use crate::types::Theme;

pub fn value_bar(value: f64, min: f64, max: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    if max <= min || width == 0 { return vec![Span::raw(" ".repeat(width))]; }
    let pos = (((value - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..pos { chars[i] = ('─', Style::default().fg(color)); }
    if pos < width { chars[pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    chars.into_iter().map(|(c, s)| Span::styled(c.to_string(), s)).collect()
}

/// Dual bar: primary=● with trail=─, secondary=◆, all in `color`.
pub fn dual_bar(primary: f64, secondary: f64, min: f64, max: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    if max <= min || width == 0 { return vec![Span::raw(" ".repeat(width))]; }
    let p_pos = (((primary  - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let s_pos = (((secondary - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..=p_pos { chars[i] = ('─', Style::default().fg(color)); }
    if s_pos < width { chars[s_pos] = ('◆', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    if p_pos < width { chars[p_pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD)); }
    chars.into_iter().map(|(c, s)| Span::styled(c.to_string(), s)).collect()
}

pub fn temp_bar(temp: f64, apparent: f64, min: f64, max: f64, width: usize, theme: Theme) -> Vec<Span<'static>> {
    dual_bar(temp, apparent, min, max, width, temp_color(temp, theme))
}

pub fn wind_bar(speed: f64, gust: f64, min: f64, max: f64, width: usize, theme: Theme) -> Vec<Span<'static>> {
    dual_bar(speed, gust, min, max, width, wind_color(speed, theme))
}

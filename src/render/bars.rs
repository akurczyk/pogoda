use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::colors::{temp_color, wind_color};
use crate::types::Theme;

pub fn value_bar(value: f64, min: f64, max: f64, width: usize, color: Color) -> Vec<Span<'static>> {
    if max <= min || width == 0 {
        return vec![Span::raw(" ".repeat(width))];
    }
    let pos =
        (((value - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..pos {
        chars[i] = ('─', Style::default().fg(color));
    }
    if pos < width {
        chars[pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD));
    }
    chars
        .into_iter()
        .map(|(c, s)| Span::styled(c.to_string(), s))
        .collect()
}

/// Dual bar: primary=● with trail=─, secondary=◆, all in `color`.
pub fn dual_bar(
    primary: f64,
    secondary: f64,
    min: f64,
    max: f64,
    width: usize,
    color: Color,
) -> Vec<Span<'static>> {
    if max <= min || width == 0 {
        return vec![Span::raw(" ".repeat(width))];
    }
    let p_pos =
        (((primary - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let s_pos =
        (((secondary - min) / (max - min)).clamp(0.0, 1.0) * (width as f64 - 1.0)).round() as usize;
    let mut chars: Vec<(char, Style)> = vec![(' ', Style::default()); width];
    for i in 0..=p_pos {
        chars[i] = ('─', Style::default().fg(color));
    }
    if s_pos > p_pos {
        for i in (p_pos + 1)..s_pos.min(width) {
            chars[i] = ('┈', Style::default().fg(color));
        }
    }
    if s_pos < width {
        chars[s_pos] = ('◆', Style::default().fg(color).add_modifier(Modifier::BOLD));
    }
    if p_pos < width {
        chars[p_pos] = ('●', Style::default().fg(color).add_modifier(Modifier::BOLD));
    }
    chars
        .into_iter()
        .map(|(c, s)| Span::styled(c.to_string(), s))
        .collect()
}

pub fn temp_bar(
    temp: f64,
    apparent: f64,
    min: f64,
    max: f64,
    width: usize,
    theme: Theme,
) -> Vec<Span<'static>> {
    dual_bar(temp, apparent, min, max, width, temp_color(temp, theme))
}

pub fn wind_bar(
    speed: f64,
    gust: f64,
    min: f64,
    max: f64,
    width: usize,
    theme: Theme,
) -> Vec<Span<'static>> {
    dual_bar(speed, gust, min, max, width, wind_color(speed, theme))
}

/// Block-fill cell: horizontal fill = prob%, block height character = mm intensity.
pub fn rain_block_cell(
    prob: f64,
    mm: f64,
    max_mm: f64,
    width: usize,
    color: Color,
) -> Vec<Span<'static>> {
    const BLOCKS: &[char] = &['▁', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let filled = ((prob / 100.0).clamp(0.0, 1.0) * width as f64).round() as usize;
    if filled == 0 {
        return vec![Span::raw(" ".repeat(width))];
    }
    let block_idx = ((mm / max_mm.max(0.001)).clamp(0.0, 1.0) * 8.0).round() as usize;
    let ch = BLOCKS[block_idx];
    let filled_str: String = std::iter::repeat(ch).take(filled).collect();
    let mut spans = vec![Span::styled(filled_str, Style::default().fg(color))];
    if filled < width {
        spans.push(Span::raw(" ".repeat(width - filled)));
    }
    spans
}

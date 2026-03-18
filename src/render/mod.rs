pub mod banner;
pub mod bars;
pub mod charts;
pub mod drone;
pub mod table;

use ratatui::{style::{Color, Modifier}, text::Span};
use std::io::{self, Write as IoWrite};

pub fn emit_span(out: &mut impl IoWrite, span: &Span, mono: bool) -> io::Result<()> {
    let style = span.style;
    let has_bold = style.add_modifier.contains(Modifier::BOLD);
    if mono {
        if has_bold { write!(out, "\x1b[1m")?; }
        write!(out, "{}", span.content)?;
        if has_bold { write!(out, "\x1b[0m")?; }
        return Ok(());
    }
    let has_blink = style.add_modifier.contains(Modifier::SLOW_BLINK);
    let has_style = style.fg.is_some() || !style.add_modifier.is_empty();
    if has_bold  { write!(out, "\x1b[1m")?; }
    if has_blink { write!(out, "\x1b[5m")?; }
    match style.bg {
        Some(Color::Rgb(r, g, b)) => write!(out, "\x1b[48;2;{r};{g};{b}m")?,
        _ => {}
    }
    match style.fg {
        Some(Color::Rgb(r, g, b)) => write!(out, "\x1b[38;2;{r};{g};{b}m")?,
        Some(Color::White)        => write!(out, "\x1b[97m")?,
        Some(Color::DarkGray)     => write!(out, "\x1b[90m")?,
        Some(Color::Cyan)         => write!(out, "\x1b[36m")?,
        Some(Color::Blue)         => write!(out, "\x1b[34m")?,
        _ => {}
    }
    write!(out, "{}", span.content)?;
    if has_style { write!(out, "\x1b[0m")?; }
    Ok(())
}

pub fn write_colored(out: &mut impl IoWrite, ch: &str, color: Color, mono: bool) -> io::Result<()> {
    if mono {
        write!(out, "{ch}")
    } else {
        match color {
            Color::Rgb(r, g, b) => write!(out, "\x1b[38;2;{r};{g};{b}m{ch}\x1b[0m"),
            Color::White        => write!(out, "\x1b[1;37m{ch}\x1b[0m"),
            Color::DarkGray     => write!(out, "\x1b[90m{ch}\x1b[0m"),
            Color::Cyan         => write!(out, "\x1b[36m{ch}\x1b[0m"),
            _                   => write!(out, "{ch}"),
        }
    }
}

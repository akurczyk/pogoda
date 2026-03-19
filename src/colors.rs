use ratatui::style::Color;

use crate::types::Theme;

pub fn oklch_to_rgb(l: f64, c: f64, h_deg: f64) -> Color {
    let h = h_deg.to_radians();
    let a = c * h.cos();
    let b = c * h.sin();
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;
    let rl =  4.0767416621 * l_.powi(3) - 3.3077115913 * m_.powi(3) + 0.2309699292 * s_.powi(3);
    let gl = -1.2684380046 * l_.powi(3) + 2.6097574011 * m_.powi(3) - 0.3413193965 * s_.powi(3);
    let bl = -0.0041960863 * l_.powi(3) - 0.7034186147 * m_.powi(3) + 1.7076147010 * s_.powi(3);
    let gamma = |x: f64| -> u8 {
        let x = x.clamp(0.0, 1.0);
        let s = if x <= 0.0031308 { 12.92 * x } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 };
        (s * 255.0).round() as u8
    };
    Color::Rgb(gamma(rl), gamma(gl), gamma(bl))
}

/// Map t ∈ [0,1] to a perceptually uniform palette color.
pub fn palette(t: f64, theme: Theme) -> Color {
    let t = t.clamp(0.0, 1.0);
    let h = match theme {
        Theme::Blue    => 200.0 + t *  80.0, // cyan(200°) → indigo(280°)
        Theme::Warm    => 280.0 + t * 120.0, // indigo(280°) → orange(400°)
        Theme::Rainbow => 200.0 + t * 200.0, // cyan(200°) → orange(400°)
        Theme::Classic => 264.0 - t * 237.0, // blue(264°) → red(27°)
        Theme::Rainforest      => 200.0 - t *  80.0, // cyan(200°) → lime(120°)
    };
    oklch_to_rgb(0.62, 0.14, h)
}

pub fn cloud_color(_pct: f64) -> Color                { Color::DarkGray }
pub fn temp_color(t: f64, theme: Theme) -> Color      { palette(((t + 15.0) / 45.0).clamp(0.0, 1.0), theme) }
pub fn wind_color(s: f64, theme: Theme) -> Color      { palette((s / 60.0).clamp(0.0, 1.0), theme) }
pub fn pressure_color(p: f64, theme: Theme) -> Color  { palette(1.0 - ((p - 985.0) / 55.0).clamp(0.0, 1.0), theme) }


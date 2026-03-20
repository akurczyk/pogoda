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

#[cfg(test)]
mod tests {
    use super::*;

    fn is_rgb(c: Color) -> bool { matches!(c, Color::Rgb(_, _, _)) }

    #[test]
    fn palette_returns_rgb_for_all_themes() {
        for theme in [Theme::Warm, Theme::Blue, Theme::Rainbow, Theme::Classic, Theme::Rainforest] {
            assert!(is_rgb(palette(0.0, theme)));
            assert!(is_rgb(palette(0.5, theme)));
            assert!(is_rgb(palette(1.0, theme)));
        }
    }

    #[test]
    fn palette_clamps_out_of_range() {
        // Values outside [0,1] must not panic and must return Rgb
        assert!(is_rgb(palette(-1.0, Theme::Warm)));
        assert!(is_rgb(palette(2.0,  Theme::Warm)));
    }

    #[test]
    fn oklch_to_rgb_pure_black_approx() {
        // L=0 should produce near-black
        let Color::Rgb(r, g, b) = oklch_to_rgb(0.0, 0.0, 0.0) else { panic!("not Rgb") };
        assert!(r < 10 && g < 10 && b < 10);
    }

    #[test]
    fn cloud_color_is_dark_gray() {
        assert_eq!(cloud_color(50.0), Color::DarkGray);
    }

    #[test]
    fn temp_color_returns_rgb() {
        // -15°C → t=0, 30°C → t=1
        assert!(is_rgb(temp_color(-15.0, Theme::Warm)));
        assert!(is_rgb(temp_color(30.0,  Theme::Warm)));
        assert!(is_rgb(temp_color(0.0,   Theme::Blue)));
    }

    #[test]
    fn wind_color_returns_rgb() {
        assert!(is_rgb(wind_color(0.0,  Theme::Warm)));
        assert!(is_rgb(wind_color(30.0, Theme::Classic)));
        assert!(is_rgb(wind_color(60.0, Theme::Rainforest)));
    }

    #[test]
    fn pressure_color_returns_rgb() {
        assert!(is_rgb(pressure_color(985.0,  Theme::Warm)));  // t=1 → low
        assert!(is_rgb(pressure_color(1040.0, Theme::Warm)));  // t=0 → high
        assert!(is_rgb(pressure_color(1013.0, Theme::Blue)));
    }
}


pub fn c_to_f(v: f64) -> f64 {
    v * 9.0 / 5.0 + 32.0
}
pub fn kmh_to_mph(v: f64) -> f64 {
    v * 0.621371
}
pub fn mm_to_in(v: f64) -> f64 {
    v / 25.4
}
pub fn hpa_to_inhg(v: f64) -> f64 {
    v * 0.02953
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_to_f_freezing() {
        assert_eq!(c_to_f(0.0), 32.0);
    }

    #[test]
    fn c_to_f_boiling() {
        assert_eq!(c_to_f(100.0), 212.0);
    }

    #[test]
    fn c_to_f_negative_40_crossover() {
        assert_eq!(c_to_f(-40.0), -40.0);
    }

    #[test]
    fn kmh_to_mph_hundred() {
        let result = kmh_to_mph(100.0);
        assert!((result - 62.1371).abs() < 0.0001);
    }

    #[test]
    fn mm_to_in_one_inch() {
        let result = mm_to_in(25.4);
        assert!((result - 1.0).abs() < 0.0001);
    }

    #[test]
    fn hpa_to_inhg_standard_pressure() {
        // 1013.25 hPa ≈ 29.92 inHg
        let result = hpa_to_inhg(1013.25);
        assert!((result - 29.921).abs() < 0.01);
    }
}

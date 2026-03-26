pub fn checksum_sum(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

pub fn checksum_xor(data: &[u8]) -> u8 {
    data.iter().fold(0u8, |acc, &b| acc ^ b)
}

pub fn parse_dms(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(&[':', '*', '°', '\'', '"'][..]).collect();
    match parts.len() {
        1 => s.parse().ok(),
        2 => {
            let deg: f64 = parts[0].parse().ok()?;
            let min: f64 = parts[1].trim_end_matches(&['\'', '"'][..]).parse().ok()?;
            let sign = if deg < 0.0 || s.starts_with('-') {
                -1.0
            } else {
                1.0
            };
            Some(sign * (deg.abs() + min / 60.0))
        }
        3 | 4 => {
            let deg: f64 = parts[0].parse().ok()?;
            let min: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].trim_end_matches(&['\'', '"'][..]).parse().ok()?;
            let sign = if deg < 0.0 || s.starts_with('-') {
                -1.0
            } else {
                1.0
            };
            Some(sign * (deg.abs() + min / 60.0 + sec / 3600.0))
        }
        _ => None,
    }
}

pub fn format_dms(deg: f64, precision: usize) -> String {
    let sign = if deg < 0.0 { "-" } else { "+" };
    let deg_abs = deg.abs();
    let d = deg_abs.floor() as i32;
    let m_full = (deg_abs - d as f64) * 60.0;
    let m = m_full.floor() as i32;
    let s = (m_full - m as f64) * 60.0;
    match precision {
        0 => format!("{}{:03}", sign, d),
        1 => format!("{}{}*{:02}", sign, d, m),
        _ => format!("{}{}*{:02}:{:02.0}", sign, d, m, s),
    }
}

pub fn format_angle_3digit(deg: f64) -> String {
    format!("{:03}", deg.round() as i32 % 1000)
}

pub fn normalize_azimuth(az: f64) -> f64 {
    let mut result = az % 360.0;
    if result < 0.0 {
        result += 360.0;
    }
    result
}

pub fn normalize_elevation(el: f64) -> f64 {
    el.clamp(-90.0, 90.0)
}

pub fn ra_to_hours(ra_deg: f64) -> f64 {
    ra_deg / 15.0
}

pub fn hours_to_ra(hours: f64) -> f64 {
    hours * 15.0
}

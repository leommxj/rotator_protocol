use std::f64::consts::PI;

use crate::model::Position;

const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;

#[derive(Debug, Clone, Copy)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

impl Location {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

pub struct CoordinateConverter {
    location: Location,
}

impl CoordinateConverter {
    pub fn new(location: Location) -> Self {
        Self { location }
    }

    fn julian_date() -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        2440587.5 + now / 86400.0
    }

    fn greenwich_mean_sidereal_time(jd: f64) -> f64 {
        let t = (jd - 2451545.0) / 36525.0;
        let gmst = 280.46061837 + 360.98564736629 * (jd - 2451545.0) + 0.000387933 * t * t
            - t * t * t / 38710000.0;
        gmst.rem_euclid(360.0)
    }

    fn local_sidereal_time(&self, jd: f64) -> f64 {
        let gmst = Self::greenwich_mean_sidereal_time(jd);
        (gmst + self.location.longitude).rem_euclid(360.0)
    }

    pub fn equatorial_to_horizontal(&self, pos: Position) -> Position {
        let jd = Self::julian_date();
        let lst = self.local_sidereal_time(jd);

        let ra_deg = pos.ra() * 15.0;
        let dec_deg = pos.dec();

        let ha_deg = (lst - ra_deg).rem_euclid(360.0);
        let ha = ha_deg * DEG_TO_RAD;
        let dec = dec_deg * DEG_TO_RAD;
        let lat = self.location.latitude * DEG_TO_RAD;

        let sin_alt = dec.sin() * lat.sin() + dec.cos() * lat.cos() * ha.cos();
        let alt = sin_alt.asin();

        let cos_az = (dec.sin() - alt.sin() * lat.sin()) / (alt.cos() * lat.cos());
        let cos_az = cos_az.clamp(-1.0, 1.0);
        let mut az = cos_az.acos();

        if ha.sin() > 0.0 {
            az = 2.0 * PI - az;
        }

        let az_deg = az * RAD_TO_DEG;
        let alt_deg = alt * RAD_TO_DEG;

        Position::new_altaz(az_deg, alt_deg)
    }

    pub fn horizontal_to_equatorial(&self, pos: Position) -> Position {
        let jd = Self::julian_date();
        let lst = self.local_sidereal_time(jd);

        let az = pos.azimuth() * DEG_TO_RAD;
        let alt = pos.elevation() * DEG_TO_RAD;
        let lat = self.location.latitude * DEG_TO_RAD;

        let sin_dec = alt.sin() * lat.sin() + alt.cos() * lat.cos() * az.cos();
        let dec = sin_dec.asin();

        let cos_ha = (alt.sin() - dec.sin() * lat.sin()) / (dec.cos() * lat.cos());
        let cos_ha = cos_ha.clamp(-1.0, 1.0);
        let mut ha = cos_ha.acos();

        if az.sin() > 0.0 {
            ha = 2.0 * PI - ha;
        }

        let ha_deg = ha * RAD_TO_DEG;
        let ra_deg = (lst - ha_deg).rem_euclid(360.0);
        let ra_hours = ra_deg / 15.0;
        let dec_deg = dec * RAD_TO_DEG;

        Position::new_equatorial(ra_hours, dec_deg)
    }
}

impl Default for CoordinateConverter {
    fn default() -> Self {
        Self::new(Location::default())
    }
}

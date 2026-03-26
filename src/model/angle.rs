use crate::config::{RotatorLimits, RotatorOffset};
use crate::error::{Error, Result};
use crate::model::Position;

#[derive(Debug, Clone)]
pub struct AngleConverter {
    limits: RotatorLimits,
    offset: RotatorOffset,
}

impl AngleConverter {
    pub fn new(limits: RotatorLimits, offset: RotatorOffset) -> Self {
        Self { limits, offset }
    }

    pub fn convert(&self, pos: Position) -> Result<Position> {
        let mut az = pos.azimuth();
        let mut el = pos.elevation();

        az = self.normalize_azimuth(az);
        az += self.offset.azimuth;
        az = self.normalize_azimuth(az);

        el += self.offset.elevation;

        self.validate_azimuth(az)?;
        self.validate_elevation(el)?;

        Ok(Position::new_altaz(az, el))
    }

    pub fn reverse(&self, pos: Position) -> Position {
        let mut az = pos.azimuth() - self.offset.azimuth;
        let el = pos.elevation() - self.offset.elevation;
        az = self.normalize_azimuth(az);
        Position::new_altaz(az, el)
    }

    fn normalize_azimuth(&self, mut az: f64) -> f64 {
        let range = self.limits.azimuth_max - self.limits.azimuth_min;
        if range <= 0.0 {
            return az;
        }

        while az < self.limits.azimuth_min {
            az += 360.0;
        }
        while az >= self.limits.azimuth_min + 360.0 {
            az -= 360.0;
        }

        az
    }

    fn validate_azimuth(&self, az: f64) -> Result<()> {
        if az < self.limits.azimuth_min || az > self.limits.azimuth_max {
            return Err(Error::OutOfRange(format!(
                "Azimuth {:.2} out of range [{:.2}, {:.2}]",
                az, self.limits.azimuth_min, self.limits.azimuth_max
            )));
        }
        Ok(())
    }

    fn validate_elevation(&self, el: f64) -> Result<()> {
        if el < self.limits.elevation_min || el > self.limits.elevation_max {
            return Err(Error::OutOfRange(format!(
                "Elevation {:.2} out of range [{:.2}, {:.2}]",
                el, self.limits.elevation_min, self.limits.elevation_max
            )));
        }
        Ok(())
    }

    pub fn limits(&self) -> &RotatorLimits {
        &self.limits
    }

    pub fn offset(&self) -> &RotatorOffset {
        &self.offset
    }
}

impl Default for AngleConverter {
    fn default() -> Self {
        Self::new(RotatorLimits::default(), RotatorOffset::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoordinateSystem {
    AltAz,
    Equatorial,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Position {
    pub axis1: f64,
    pub axis2: f64,
    pub coord_system: CoordinateSystem,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        CoordinateSystem::AltAz
    }
}

impl Position {
    pub fn new_altaz(azimuth: f64, elevation: f64) -> Self {
        Self {
            axis1: azimuth,
            axis2: elevation,
            coord_system: CoordinateSystem::AltAz,
        }
    }

    pub fn new_equatorial(ra: f64, dec: f64) -> Self {
        Self {
            axis1: ra,
            axis2: dec,
            coord_system: CoordinateSystem::Equatorial,
        }
    }

    pub fn azimuth(&self) -> f64 {
        self.axis1
    }

    pub fn elevation(&self) -> f64 {
        self.axis2
    }

    pub fn ra(&self) -> f64 {
        self.axis1
    }

    pub fn dec(&self) -> f64 {
        self.axis2
    }
}

use super::common::{format_angle_3digit, normalize_azimuth, normalize_elevation};
use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, MoveDirection, Position, Speed, UnifiedCommand};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Gs232Variant {
    A,
    #[default]
    B,
}

pub struct Gs232Codec {
    buffer: Vec<u8>,
    variant: Gs232Variant,
}

impl Gs232Codec {
    pub fn new() -> Self {
        Self::with_variant(Gs232Variant::B)
    }

    pub fn new_a() -> Self {
        Self::with_variant(Gs232Variant::A)
    }

    pub fn new_b() -> Self {
        Self::with_variant(Gs232Variant::B)
    }

    pub fn with_variant(variant: Gs232Variant) -> Self {
        Self {
            buffer: Vec::new(),
            variant,
        }
    }

    fn parse_position_response(&self, line: &str) -> Option<Position> {
        let line = line.trim();

        // GS-232B format: +0aaa+0eee
        if line.starts_with("+0") || line.starts_with("-0") {
            // Check if it has elevation part
            if let Some(second_plus) = line[1..].find('+') {
                let az_part = &line[2..second_plus + 1];
                let el_part = &line[second_plus + 3..];
                let az: f64 = az_part.parse().ok()?;
                let el: f64 = el_part.parse().ok()?;
                return Some(Position::new_altaz(az, el));
            } else {
                // GS-232A format: +0aaa (azimuth only)
                let az: f64 = line[2..].parse().ok()?;
                return Some(Position::new_altaz(az, 0.0));
            }
        }

        // AZ=aaa EL=eee format
        if line.contains("AZ=") {
            let az = line
                .split("AZ=")
                .nth(1)?
                .split_whitespace()
                .next()?
                .parse()
                .ok()?;
            let el = line
                .split("EL=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            return Some(Position::new_altaz(az, el));
        }

        // Simple numeric response for C command: aaa
        if line.len() >= 3 && line.len() <= 4 {
            if let Ok(az) = line.parse::<f64>() {
                return Some(Position::new_altaz(az, 0.0));
            }
        }

        None
    }
}

impl Default for Gs232Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Gs232Codec {
    fn name(&self) -> &'static str {
        match self.variant {
            Gs232Variant::A => "gs232a",
            Gs232Variant::B => "gs232b",
        }
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        let msg = match cmd {
            UnifiedCommand::GetPosition => match self.variant {
                Gs232Variant::A => "C\r".to_string(),
                Gs232Variant::B => "C2\r".to_string(),
            },

            UnifiedCommand::GotoPosition(pos) => {
                let az = normalize_azimuth(pos.azimuth());
                match self.variant {
                    Gs232Variant::A => {
                        format!("M{}\r", format_angle_3digit(az))
                    }
                    Gs232Variant::B => {
                        let el = normalize_elevation(pos.elevation());
                        format!("W{} {}\r", format_angle_3digit(az), format_angle_3digit(el))
                    }
                }
            }

            UnifiedCommand::Move { direction, .. } => match direction {
                MoveDirection::Right => "R\r".to_string(),
                MoveDirection::Left => "L\r".to_string(),
                MoveDirection::Up => {
                    if self.variant == Gs232Variant::A {
                        return Err(Error::NotSupported(
                            "Elevation control not supported in GS-232A".into(),
                        ));
                    }
                    "U\r".to_string()
                }
                MoveDirection::Down => {
                    if self.variant == Gs232Variant::A {
                        return Err(Error::NotSupported(
                            "Elevation control not supported in GS-232A".into(),
                        ));
                    }
                    "D\r".to_string()
                }
                _ => return Err(Error::NotSupported("Diagonal movement".into())),
            },

            UnifiedCommand::Stop => "S\r".to_string(),
            UnifiedCommand::StopAxis1 => "A\r".to_string(),
            UnifiedCommand::StopAxis2 => {
                if self.variant == Gs232Variant::A {
                    return Err(Error::NotSupported(
                        "Elevation control not supported in GS-232A".into(),
                    ));
                }
                "E\r".to_string()
            }

            UnifiedCommand::SetSpeed(speed) => {
                let val = match speed {
                    Speed::Normalized(n) => (*n * 4.0) as u8,
                    Speed::Absolute(d) => (*d / 10.0) as u8,
                };
                format!("X{}\r", val.min(4))
            }

            _ => return Err(Error::NotSupported(format!("{:?}", cmd))),
        };
        Ok(msg.into_bytes())
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        if let Some(pos) = self.buffer.iter().position(|&b| b == b'\r' || b == b'\n') {
            let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
            self.buffer.drain(..=pos);
            // Skip additional \n after \r
            if !self.buffer.is_empty() && self.buffer[0] == b'\n' {
                self.buffer.drain(..1);
            }

            let line = line.trim();
            if line.is_empty() {
                return Ok(DecodeResult::None);
            }

            // Try parsing as position response FIRST (before checking commands)
            if let Some(pos) = self.parse_position_response(line) {
                return Ok(DecodeResult::Status(DeviceStatus::with_position(pos)));
            }

            // Parse command from client
            match line.chars().next() {
                Some('C') => {
                    if line == "C" || line == "C2" || line == "B" {
                        return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                    }
                }
                Some('M') => {
                    if let Ok(az) = line[1..].trim().parse::<f64>() {
                        return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_altaz(az, 0.0),
                        )));
                    }
                }
                Some('W') => {
                    let parts: Vec<&str> = line[1..].split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let (Ok(az), Ok(el)) = (parts[0].parse(), parts[1].parse()) {
                            return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                                Position::new_altaz(az, el),
                            )));
                        }
                    }
                }
                Some('R') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_right(
                        Speed::default(),
                    )));
                }
                Some('L') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_left(
                        Speed::default(),
                    )));
                }
                Some('U') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_up(
                        Speed::default(),
                    )));
                }
                Some('D') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_down(
                        Speed::default(),
                    )));
                }
                Some('S') => return Ok(DecodeResult::Command(UnifiedCommand::Stop)),
                Some('A') => return Ok(DecodeResult::Command(UnifiedCommand::StopAxis1)),
                Some('E') => return Ok(DecodeResult::Command(UnifiedCommand::StopAxis2)),
                Some('B') => return Ok(DecodeResult::Command(UnifiedCommand::GetPosition)),
                _ => {}
            }

            Ok(DecodeResult::None)
        } else {
            Ok(DecodeResult::NeedMoreData)
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: true,
            has_goto: true,
            has_continuous_move: true,
            has_stop: true,
            has_park: false,
            has_presets: false,
            has_speed_control: true,
            supports_equatorial: false,
            supports_altaz: true,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            let az = pos.azimuth() as i32;
            let el = pos.elevation() as i32;
            match self.variant {
                Gs232Variant::A => Ok(format!("AZ={:03}\r\n", az).into_bytes()),
                Gs232Variant::B => Ok(format!("AZ={:03}  EL={:03}\r\n", az, el).into_bytes()),
            }
        } else {
            Ok(b"?\r\n".to_vec())
        }
    }
}

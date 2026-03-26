use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, MoveDirection, Position, Speed, UnifiedCommand};

pub struct EasycommCodec {
    buffer: Vec<u8>,
    version: u8,
}

impl EasycommCodec {
    pub fn new(version: u8) -> Self {
        Self {
            buffer: Vec::new(),
            version: version.clamp(1, 3),
        }
    }
}

impl Default for EasycommCodec {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Codec for EasycommCodec {
    fn name(&self) -> &'static str {
        "easycomm"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        let msg = match cmd {
            UnifiedCommand::GotoPosition(pos) => {
                format!("AZ{:.1} EL{:.1}\n", pos.azimuth(), pos.elevation())
            }

            UnifiedCommand::Move { direction, speed } if self.version >= 2 => {
                let spd = match speed {
                    Speed::Normalized(n) => (n * 1000.0) as i32,
                    Speed::Absolute(d) => (*d * 1000.0) as i32,
                };
                match direction {
                    MoveDirection::Left => format!("VL{}\n", spd),
                    MoveDirection::Right => format!("VR{}\n", spd),
                    MoveDirection::Up => format!("VU{}\n", spd),
                    MoveDirection::Down => format!("VD{}\n", spd),
                    _ => return Err(Error::NotSupported("Diagonal movement".into())),
                }
            }

            UnifiedCommand::Move { direction, .. } => match direction {
                MoveDirection::Left => "ML\n".to_string(),
                MoveDirection::Right => "MR\n".to_string(),
                MoveDirection::Up => "MU\n".to_string(),
                MoveDirection::Down => "MD\n".to_string(),
                _ => return Err(Error::NotSupported("Diagonal movement".into())),
            },

            UnifiedCommand::Stop => "SA\nSE\n".to_string(),
            UnifiedCommand::StopAxis1 => "SA\n".to_string(),
            UnifiedCommand::StopAxis2 => "SE\n".to_string(),

            UnifiedCommand::GetInfo => "VE\n".to_string(),

            _ => return Err(Error::NotSupported(format!("{:?}", cmd))),
        };
        Ok(msg.into_bytes())
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        if let Some(pos) = self
            .buffer
            .iter()
            .position(|&b| b == b'\n' || b == b'\r' || b == b' ')
        {
            let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
            self.buffer.drain(..=pos);

            let line = line.trim();
            if line.is_empty() {
                return Ok(DecodeResult::None);
            }

            // Parse AZxxx.x ELyyy.y format
            if line.starts_with("AZ") {
                let az: f64 = line[2..].parse().unwrap_or(0.0);

                // Check for EL in buffer
                if let Some(el_pos) = self.buffer.windows(2).position(|w| w == b"EL") {
                    if let Some(end) = self.buffer[el_pos..]
                        .iter()
                        .position(|&b| b == b'\n' || b == b'\r' || b == b' ')
                    {
                        let el_str =
                            String::from_utf8_lossy(&self.buffer[el_pos + 2..el_pos + end]);
                        let el: f64 = el_str.parse().unwrap_or(0.0);
                        self.buffer.drain(..el_pos + end + 1);
                        return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_altaz(az, el),
                        )));
                    }
                }
                return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                    Position::new_altaz(az, 0.0),
                )));
            }

            if line.starts_with("EL") {
                // EL without AZ - position update
                let el: f64 = line[2..].parse().unwrap_or(0.0);
                return Ok(DecodeResult::Status(DeviceStatus::with_position(
                    Position::new_altaz(0.0, el),
                )));
            }

            match line {
                "ML" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_left(
                        Speed::default(),
                    )));
                }
                "MR" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_right(
                        Speed::default(),
                    )));
                }
                "MU" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_up(
                        Speed::default(),
                    )));
                }
                "MD" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::move_down(
                        Speed::default(),
                    )));
                }
                "SA" => return Ok(DecodeResult::Command(UnifiedCommand::StopAxis1)),
                "SE" => return Ok(DecodeResult::Command(UnifiedCommand::StopAxis2)),
                "VE" => return Ok(DecodeResult::Command(UnifiedCommand::GetInfo)),
                _ => {}
            }

            // VL/VR/VU/VD with speed
            if line.len() > 2 {
                let prefix = &line[..2];
                let value: i32 = line[2..].parse().unwrap_or(0);
                let speed = Speed::Absolute(value as f64 / 1000.0);

                match prefix {
                    "VL" => {
                        return Ok(DecodeResult::Command(UnifiedCommand::Move {
                            direction: MoveDirection::Left,
                            speed,
                        }));
                    }
                    "VR" => {
                        return Ok(DecodeResult::Command(UnifiedCommand::Move {
                            direction: MoveDirection::Right,
                            speed,
                        }));
                    }
                    "VU" => {
                        return Ok(DecodeResult::Command(UnifiedCommand::Move {
                            direction: MoveDirection::Up,
                            speed,
                        }));
                    }
                    "VD" => {
                        return Ok(DecodeResult::Command(UnifiedCommand::Move {
                            direction: MoveDirection::Down,
                            speed,
                        }));
                    }
                    _ => {}
                }
            }

            Ok(DecodeResult::None)
        } else {
            Ok(DecodeResult::NeedMoreData)
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: false,
            has_goto: true,
            has_continuous_move: true,
            has_stop: true,
            has_park: false,
            has_presets: false,
            has_speed_control: self.version >= 3,
            supports_equatorial: false,
            supports_altaz: true,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            Ok(format!("AZ{:.1} EL{:.1}\n", pos.azimuth(), pos.elevation()).into_bytes())
        } else {
            Ok(Vec::new())
        }
    }
}

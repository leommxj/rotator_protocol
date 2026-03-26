use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, MoveDirection, Position, Speed, UnifiedCommand};

pub struct RotctldCodec {
    buffer: Vec<u8>,
}

impl RotctldCodec {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn parse_direction(s: &str) -> Option<MoveDirection> {
        match s.to_uppercase().as_str() {
            "2" | "UP" => Some(MoveDirection::Up),
            "4" | "DOWN" => Some(MoveDirection::Down),
            "8" | "LEFT" | "CCW" => Some(MoveDirection::Left),
            "16" | "RIGHT" | "CW" => Some(MoveDirection::Right),
            _ => None,
        }
    }
}

impl Default for RotctldCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for RotctldCodec {
    fn name(&self) -> &'static str {
        "rotctld"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        let msg = match cmd {
            UnifiedCommand::GetPosition => "p\n".to_string(),

            UnifiedCommand::GotoPosition(pos) => {
                format!("P {} {}\n", pos.azimuth(), pos.elevation())
            }

            UnifiedCommand::Move { direction, speed } => {
                let dir = match direction {
                    MoveDirection::Up => 2,
                    MoveDirection::Down => 4,
                    MoveDirection::Left => 8,
                    MoveDirection::Right => 16,
                    MoveDirection::UpLeft => 32,
                    MoveDirection::UpRight => 64,
                    MoveDirection::DownLeft => 128,
                    MoveDirection::DownRight => 256,
                };
                let spd = match speed {
                    Speed::Normalized(n) => (n * 100.0) as i32,
                    Speed::Absolute(d) => *d as i32,
                };
                format!("M {} {}\n", dir, spd)
            }

            UnifiedCommand::Stop => "S\n".to_string(),
            UnifiedCommand::Park => "K\n".to_string(),
            UnifiedCommand::Reset => "R 1\n".to_string(),
            UnifiedCommand::GetInfo => "_\n".to_string(),

            _ => return Err(Error::NotSupported(format!("{:?}", cmd))),
        };
        Ok(msg.into_bytes())
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        // Check for line ending (\n or \r)
        let line_end = self.buffer.iter().position(|&b| b == b'\n' || b == b'\r');

        // If no line ending, try to parse commands without line ending
        if line_end.is_none() && !self.buffer.is_empty() {
            let text = String::from_utf8_lossy(&self.buffer).to_string();
            let text = text.trim();

            // Single character commands
            if text.len() == 1 {
                let first = self.buffer[0];
                match first {
                    b'p' => {
                        self.buffer.clear();
                        return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                    }
                    b'S' => {
                        self.buffer.clear();
                        return Ok(DecodeResult::Command(UnifiedCommand::Stop));
                    }
                    b'K' => {
                        self.buffer.clear();
                        return Ok(DecodeResult::Command(UnifiedCommand::Park));
                    }
                    b'_' => {
                        self.buffer.clear();
                        return Ok(DecodeResult::Command(UnifiedCommand::GetInfo));
                    }
                    b'q' => {
                        self.buffer.clear();
                        return Ok(DecodeResult::None);
                    }
                    _ => {}
                }
            }

            // Try to parse multi-arg commands without newline
            let parts: Vec<&str> = text.split_whitespace().collect();
            if !parts.is_empty() {
                match parts[0] {
                    "P" if parts.len() >= 3 => {
                        if let (Ok(az), Ok(el)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>())
                        {
                            self.buffer.clear();
                            return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                                Position::new_altaz(az, el),
                            )));
                        }
                    }
                    "M" if parts.len() >= 3 => {
                        if let Some(dir) = Self::parse_direction(parts[1]) {
                            let speed = parts[2].parse().unwrap_or(50);
                            self.buffer.clear();
                            return Ok(DecodeResult::Command(UnifiedCommand::Move {
                                direction: dir,
                                speed: Speed::Normalized(speed as f64 / 100.0),
                            }));
                        }
                    }
                    _ => {}
                }
            }

            return Ok(DecodeResult::NeedMoreData);
        }

        if let Some(pos) = line_end {
            let line = String::from_utf8_lossy(&self.buffer[..pos]).to_string();
            // Skip the line ending character(s)
            let mut end = pos + 1;
            if end < self.buffer.len() && self.buffer[end] == b'\n' {
                end += 1; // Handle \r\n
            }
            self.buffer.drain(..end);

            let line = line.trim();
            if line.is_empty() {
                return Ok(DecodeResult::None);
            }

            // Check for RPRT response (success/error)
            if line.starts_with("RPRT") {
                let code: i32 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                if code == 0 {
                    return Ok(DecodeResult::Status(DeviceStatus::ok()));
                } else {
                    return Ok(DecodeResult::Status(DeviceStatus::error(
                        code,
                        format!("Error code: {}", code),
                    )));
                }
            }

            // Try parsing as position (two lines: az then el)
            if let Ok(value) = line.parse::<f64>() {
                // This might be azimuth, check for next line
                if let Some(pos2) = self.buffer.iter().position(|&b| b == b'\n' || b == b'\r') {
                    let line2 = String::from_utf8_lossy(&self.buffer[..pos2]).to_string();
                    let mut end2 = pos2 + 1;
                    if end2 < self.buffer.len() && self.buffer[end2] == b'\n' {
                        end2 += 1;
                    }
                    self.buffer.drain(..end2);
                    if let Ok(el) = line2.trim().parse::<f64>() {
                        return Ok(DecodeResult::Status(DeviceStatus::with_position(
                            Position::new_altaz(value, el),
                        )));
                    }
                }
                // Single value - might be just azimuth
                return Ok(DecodeResult::Status(DeviceStatus::with_position(
                    Position::new_altaz(value, 0.0),
                )));
            }

            // Parse commands from client
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                return Ok(DecodeResult::None);
            }

            match parts[0] {
                "P" | "\\set_pos" if parts.len() >= 3 => {
                    if let (Ok(az), Ok(el)) = (parts[1].parse(), parts[2].parse()) {
                        return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                            Position::new_altaz(az, el),
                        )));
                    }
                }
                "p" | "\\get_pos" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                }
                "M" | "\\move" if parts.len() >= 3 => {
                    if let Some(dir) = Self::parse_direction(parts[1]) {
                        let speed = parts[2].parse().unwrap_or(50);
                        return Ok(DecodeResult::Command(UnifiedCommand::Move {
                            direction: dir,
                            speed: Speed::Normalized(speed as f64 / 100.0),
                        }));
                    }
                }
                "S" | "\\stop" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::Stop));
                }
                "K" | "\\park" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::Park));
                }
                "R" | "\\reset" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::Reset));
                }
                "_" | "\\get_info" => {
                    return Ok(DecodeResult::Command(UnifiedCommand::GetInfo));
                }
                "q" | "\\quit" => {
                    return Ok(DecodeResult::None);
                }
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
            has_park: true,
            has_presets: false,
            has_speed_control: true,
            supports_equatorial: false,
            supports_altaz: true,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            Ok(format!("{:.6}\n{:.6}\n", pos.azimuth(), pos.elevation()).into_bytes())
        } else if status.error_code.is_some() {
            Ok(format!("RPRT {}\n", status.error_code.unwrap_or(-1)).into_bytes())
        } else {
            Ok(b"RPRT 0\n".to_vec())
        }
    }
}

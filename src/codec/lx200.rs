use super::common::parse_dms;
use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, MoveDirection, Position, Speed, UnifiedCommand};

pub struct Lx200Codec {
    buffer: Vec<u8>,
}

impl Lx200Codec {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn format_ra(hours: f64) -> String {
        let h = hours.floor() as i32;
        let m_full = (hours - h as f64) * 60.0;
        let m = m_full.floor() as i32;
        let s = (m_full - m as f64) * 60.0;
        format!("{:02}:{:02}:{:02}", h % 24, m, s as i32)
    }

    fn format_dec(deg: f64) -> String {
        let sign = if deg < 0.0 { "-" } else { "+" };
        let d = deg.abs().floor() as i32;
        let m_full = (deg.abs() - d as f64) * 60.0;
        let m = m_full.floor() as i32;
        let s = (m_full - m as f64) * 60.0;
        format!("{}{}*{:02}:{:02}", sign, d, m, s as i32)
    }

    fn parse_ra(s: &str) -> Option<f64> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() >= 2 {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            return Some(h + m / 60.0 + s / 3600.0);
        }
        None
    }
}

impl Default for Lx200Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for Lx200Codec {
    fn name(&self) -> &'static str {
        "lx200"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        let msg = match cmd {
            UnifiedCommand::GetPosition => ":GR#:GD#".to_string(),

            UnifiedCommand::GotoPosition(pos) => {
                format!(
                    ":Sr {}#:Sd {}#:MS#",
                    Self::format_ra(pos.ra()),
                    Self::format_dec(pos.dec())
                )
            }

            UnifiedCommand::Move { direction, .. } => match direction {
                MoveDirection::Up => ":Mn#".to_string(),
                MoveDirection::Down => ":Ms#".to_string(),
                MoveDirection::Left => ":Mw#".to_string(),
                MoveDirection::Right => ":Me#".to_string(),
                _ => return Err(Error::NotSupported("Diagonal movement".into())),
            },

            UnifiedCommand::Stop => ":Q#".to_string(),

            UnifiedCommand::SetSpeed(speed) => {
                let cmd = match speed {
                    Speed::Normalized(n) if *n < 0.25 => ":RG#",
                    Speed::Normalized(n) if *n < 0.5 => ":RC#",
                    Speed::Normalized(n) if *n < 0.75 => ":RM#",
                    _ => ":RS#",
                };
                cmd.to_string()
            }

            _ => return Err(Error::NotSupported(format!("{:?}", cmd))),
        };
        Ok(msg.into_bytes())
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        // LX200 commands start with : and end with #
        if let Some(start) = self.buffer.iter().position(|&b| b == b':') {
            if let Some(end) = self.buffer[start..].iter().position(|&b| b == b'#') {
                let cmd_bytes = self.buffer[start + 1..start + end].to_vec();
                self.buffer.drain(..start + end + 1);

                let cmd_str = String::from_utf8_lossy(&cmd_bytes);

                // Parse command
                if cmd_str.starts_with("Sr ") {
                    // Set RA target
                    return Ok(DecodeResult::None);
                }
                if cmd_str.starts_with("Sd ") {
                    // Set Dec target
                    return Ok(DecodeResult::None);
                }
                if cmd_str == "MS" {
                    return Ok(DecodeResult::None); // GOTO execute
                }
                if cmd_str == "GR" || cmd_str == "GD" {
                    return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                }
                if cmd_str == "Q" {
                    return Ok(DecodeResult::Command(UnifiedCommand::Stop));
                }
                if cmd_str.starts_with('M') && cmd_str.len() == 2 {
                    let dir = match cmd_str.chars().nth(1) {
                        Some('n') => MoveDirection::Up,
                        Some('s') => MoveDirection::Down,
                        Some('e') => MoveDirection::Right,
                        Some('w') => MoveDirection::Left,
                        _ => return Ok(DecodeResult::None),
                    };
                    return Ok(DecodeResult::Command(UnifiedCommand::Move {
                        direction: dir,
                        speed: Speed::default(),
                    }));
                }

                return Ok(DecodeResult::None);
            }
        }

        // Check for response without colon (RA/Dec values)
        if let Some(end) = self.buffer.iter().position(|&b| b == b'#') {
            let resp = String::from_utf8_lossy(&self.buffer[..end]).to_string();
            self.buffer.drain(..end + 1);

            // Try parse as RA
            if let Some(ra) = Self::parse_ra(&resp) {
                // Check for next value (Dec)
                if let Some(end2) = self.buffer.iter().position(|&b| b == b'#') {
                    let resp2 = String::from_utf8_lossy(&self.buffer[..end2]).to_string();
                    self.buffer.drain(..end2 + 1);
                    if let Some(dec) = parse_dms(&resp2) {
                        return Ok(DecodeResult::Status(DeviceStatus::with_position(
                            Position::new_equatorial(ra, dec),
                        )));
                    }
                }
            }
        }

        Ok(DecodeResult::NeedMoreData)
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
            supports_equatorial: true,
            supports_altaz: true,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            Ok(format!(
                "{}#{}#",
                Self::format_ra(pos.ra()),
                Self::format_dec(pos.dec())
            )
            .into_bytes())
        } else {
            Ok(Vec::new())
        }
    }
}

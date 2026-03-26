use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, Position, UnifiedCommand};

pub struct NexstarCodec {
    buffer: Vec<u8>,
}

impl NexstarCodec {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    #[allow(dead_code)]
    fn encode_position(value: f64, max: f64) -> String {
        let normalized = value / max;
        let encoded = (normalized * 65536.0) as u32;
        format!("{:04X}", encoded & 0xFFFF)
    }

    fn encode_position_precise(value: f64, max: f64) -> String {
        let normalized = value / max;
        let encoded = (normalized * (0x100000000u64 as f64)) as u32;
        format!("{:08X}", encoded)
    }

    fn decode_position(hex: &str, max: f64) -> Option<f64> {
        let value = u32::from_str_radix(hex, 16).ok()?;
        Some((value as f64 / 65536.0) * max)
    }

    fn decode_position_precise(hex: &str, max: f64) -> Option<f64> {
        let value = u32::from_str_radix(hex, 16).ok()?;
        Some((value as f64 / (0x100000000u64 as f64)) * max)
    }
}

impl Default for NexstarCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for NexstarCodec {
    fn name(&self) -> &'static str {
        "nexstar"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        let msg = match cmd {
            UnifiedCommand::GetPosition => "e".to_string(), // precise RA/Dec

            UnifiedCommand::GotoPosition(pos) => {
                let ra_hex = Self::encode_position_precise(pos.ra(), 24.0);
                let dec_hex = Self::encode_position_precise(pos.dec() + 90.0, 180.0);
                format!("r{},{}", ra_hex, dec_hex)
            }

            _ => return Err(Error::NotSupported(format!("{:?}", cmd))),
        };
        Ok(msg.into_bytes())
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        if let Some(end) = self.buffer.iter().position(|&b| b == b'#') {
            let msg = String::from_utf8_lossy(&self.buffer[..end]).to_string();
            self.buffer.drain(..end + 1);

            // Commands from client
            match msg.chars().next() {
                Some('E') | Some('e') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                }
                Some('Z') | Some('z') => {
                    return Ok(DecodeResult::Command(UnifiedCommand::GetPosition));
                }
                Some('R') | Some('r') | Some('B') | Some('b') => {
                    // GOTO command
                    let parts: Vec<&str> = msg[1..].split(',').collect();
                    if parts.len() >= 2 {
                        let precise = msg.chars().next().unwrap().is_lowercase();
                        let (ra, dec) = if precise {
                            (
                                Self::decode_position_precise(parts[0], 24.0),
                                Self::decode_position_precise(parts[1], 180.0).map(|d| d - 90.0),
                            )
                        } else {
                            (
                                Self::decode_position(parts[0], 24.0),
                                Self::decode_position(parts[1], 180.0).map(|d| d - 90.0),
                            )
                        };
                        if let (Some(ra), Some(dec)) = (ra, dec) {
                            return Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                                Position::new_equatorial(ra, dec),
                            )));
                        }
                    }
                }
                _ => {}
            }

            // Response parsing (RA,Dec#)
            let parts: Vec<&str> = msg.split(',').collect();
            if parts.len() >= 2 {
                let precise = parts[0].len() == 8;
                let (ra, dec) = if precise {
                    (
                        Self::decode_position_precise(parts[0], 24.0),
                        Self::decode_position_precise(parts[1], 180.0).map(|d| d - 90.0),
                    )
                } else {
                    (
                        Self::decode_position(parts[0], 24.0),
                        Self::decode_position(parts[1], 180.0).map(|d| d - 90.0),
                    )
                };
                if let (Some(ra), Some(dec)) = (ra, dec) {
                    return Ok(DecodeResult::Status(DeviceStatus::with_position(
                        Position::new_equatorial(ra, dec),
                    )));
                }
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
            has_continuous_move: false,
            has_stop: false,
            has_park: false,
            has_presets: false,
            has_speed_control: false,
            supports_equatorial: true,
            supports_altaz: true,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            let ra_hex = Self::encode_position_precise(pos.ra(), 24.0);
            let dec_hex = Self::encode_position_precise(pos.dec() + 90.0, 180.0);
            Ok(format!("{},{}#", ra_hex, dec_hex).into_bytes())
        } else {
            Ok(b"#".to_vec())
        }
    }
}

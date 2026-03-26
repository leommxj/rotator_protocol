use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, Position, UnifiedCommand};

const MSG_GOTO: u16 = 0;
const MSG_CURRENT_POS: u16 = 0;

pub struct StellariumCodec {
    buffer: Vec<u8>,
}

impl StellariumCodec {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn encode_ra(hours: f64) -> u32 {
        let normalized = hours / 24.0;
        (normalized * (0x100000000u64 as f64)) as u32
    }

    fn encode_dec(degrees: f64) -> i32 {
        (degrees / 90.0 * (0x40000000 as f64)) as i32
    }

    fn decode_ra(value: u32) -> f64 {
        (value as f64) * 24.0 / (0x100000000u64 as f64)
    }

    fn decode_dec(value: i32) -> f64 {
        (value as f64) * 90.0 / (0x40000000 as f64)
    }

    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0)
    }
}

impl Default for StellariumCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for StellariumCodec {
    fn name(&self) -> &'static str {
        "stellarium"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        match cmd {
            UnifiedCommand::GotoPosition(pos) => {
                // MessageGoto: length=20, type=0
                let mut msg = Vec::with_capacity(20);
                msg.extend_from_slice(&20u16.to_le_bytes());
                msg.extend_from_slice(&MSG_GOTO.to_le_bytes());
                msg.extend_from_slice(&Self::current_timestamp().to_le_bytes());
                msg.extend_from_slice(&Self::encode_ra(pos.ra()).to_le_bytes());
                msg.extend_from_slice(&Self::encode_dec(pos.dec()).to_le_bytes());
                Ok(msg)
            }

            _ => Err(Error::NotSupported(format!("{:?}", cmd))),
        }
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        if self.buffer.len() < 4 {
            return Ok(DecodeResult::NeedMoreData);
        }

        let length = u16::from_le_bytes([self.buffer[0], self.buffer[1]]) as usize;
        let msg_type = u16::from_le_bytes([self.buffer[2], self.buffer[3]]);

        if self.buffer.len() < length {
            return Ok(DecodeResult::NeedMoreData);
        }

        let packet = self.buffer.drain(..length).collect::<Vec<_>>();

        match msg_type {
            0 if length == 20 => {
                // MessageGoto from client
                let ra = u32::from_le_bytes([packet[12], packet[13], packet[14], packet[15]]);
                let dec = i32::from_le_bytes([packet[16], packet[17], packet[18], packet[19]]);

                let ra_hours = Self::decode_ra(ra);
                let dec_deg = Self::decode_dec(dec);

                Ok(DecodeResult::Command(UnifiedCommand::GotoPosition(
                    Position::new_equatorial(ra_hours, dec_deg),
                )))
            }
            _ => Ok(DecodeResult::None),
        }
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: false,
            has_goto: true,
            has_continuous_move: false,
            has_stop: false,
            has_park: false,
            has_presets: false,
            has_speed_control: false,
            supports_equatorial: true,
            supports_altaz: false,
        }
    }

    fn encode_status(&self, status: &DeviceStatus) -> Result<Vec<u8>> {
        if let Some(pos) = &status.position {
            // MessageCurrentPosition: length=24, type=0
            let mut msg = Vec::with_capacity(24);
            msg.extend_from_slice(&24u16.to_le_bytes());
            msg.extend_from_slice(&MSG_CURRENT_POS.to_le_bytes());
            msg.extend_from_slice(&Self::current_timestamp().to_le_bytes());
            msg.extend_from_slice(&Self::encode_ra(pos.ra()).to_le_bytes());
            msg.extend_from_slice(&Self::encode_dec(pos.dec()).to_le_bytes());
            msg.extend_from_slice(&0i32.to_le_bytes()); // status OK
            Ok(msg)
        } else {
            Ok(Vec::new())
        }
    }
}

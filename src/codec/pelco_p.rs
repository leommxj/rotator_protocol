use super::common::checksum_xor;
use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{MoveDirection, Speed, UnifiedCommand};

const STX: u8 = 0xA0;
const ETX: u8 = 0xAF;
const PACKET_LEN: usize = 8;

pub struct PelcoPCodec {
    address: u8,
    buffer: Vec<u8>,
}

impl PelcoPCodec {
    pub fn new(address: u8) -> Self {
        Self {
            address: address & 0x1F,
            buffer: Vec::new(),
        }
    }

    fn speed_to_byte(speed: &Speed) -> u8 {
        match speed {
            Speed::Normalized(n) => ((n * 63.0) as u8).min(0x3F),
            Speed::Absolute(d) => ((*d / 2.0) as u8).min(0x3F),
        }
    }

    fn build_packet(&self, data1: u8, data2: u8, data3: u8, data4: u8) -> Vec<u8> {
        let checksum = checksum_xor(&[STX, self.address, data1, data2, data3, data4, ETX]);
        vec![STX, self.address, data1, data2, data3, data4, ETX, checksum]
    }
}

impl Default for PelcoPCodec {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Codec for PelcoPCodec {
    fn name(&self) -> &'static str {
        "pelco_p"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        match cmd {
            UnifiedCommand::Stop => Ok(self.build_packet(0x00, 0x00, 0x00, 0x00)),

            UnifiedCommand::Move { direction, speed } => {
                let pan_speed = Self::speed_to_byte(speed);
                let tilt_speed = Self::speed_to_byte(speed);

                let (data2, d3, d4) = match direction {
                    MoveDirection::Up => (0x08, 0x00, tilt_speed),
                    MoveDirection::Down => (0x10, 0x00, tilt_speed),
                    MoveDirection::Left => (0x04, pan_speed, 0x00),
                    MoveDirection::Right => (0x02, pan_speed, 0x00),
                    MoveDirection::UpLeft => (0x0C, pan_speed, tilt_speed),
                    MoveDirection::UpRight => (0x0A, pan_speed, tilt_speed),
                    MoveDirection::DownLeft => (0x14, pan_speed, tilt_speed),
                    MoveDirection::DownRight => (0x12, pan_speed, tilt_speed),
                };
                Ok(self.build_packet(0x00, data2, d3, d4))
            }

            _ => Err(Error::NotSupported(format!("{:?}", cmd))),
        }
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        // Find STX
        while !self.buffer.is_empty() && self.buffer[0] != STX {
            self.buffer.remove(0);
        }

        if self.buffer.len() < PACKET_LEN {
            return Ok(DecodeResult::NeedMoreData);
        }

        let packet = &self.buffer[..PACKET_LEN];

        if packet[6] != ETX {
            self.buffer.remove(0);
            return Err(Error::InvalidData("Invalid ETX".into()));
        }

        let calculated_checksum = checksum_xor(&packet[..7]);
        if packet[7] != calculated_checksum {
            self.buffer.remove(0);
            return Err(Error::InvalidData("Checksum mismatch".into()));
        }

        let _addr = packet[1];
        let _data1 = packet[2];
        let data2 = packet[3];
        let data3 = packet[4];
        let data4 = packet[5];

        self.buffer.drain(..PACKET_LEN);

        let result = if data2 == 0x00 && data3 == 0x00 && data4 == 0x00 {
            DecodeResult::Command(UnifiedCommand::Stop)
        } else if data2 & 0x08 != 0 {
            let direction = if data2 & 0x04 != 0 {
                MoveDirection::UpLeft
            } else if data2 & 0x02 != 0 {
                MoveDirection::UpRight
            } else {
                MoveDirection::Up
            };
            DecodeResult::Command(UnifiedCommand::Move {
                direction,
                speed: Speed::Normalized(data4 as f64 / 63.0),
            })
        } else if data2 & 0x10 != 0 {
            let direction = if data2 & 0x04 != 0 {
                MoveDirection::DownLeft
            } else if data2 & 0x02 != 0 {
                MoveDirection::DownRight
            } else {
                MoveDirection::Down
            };
            DecodeResult::Command(UnifiedCommand::Move {
                direction,
                speed: Speed::Normalized(data4 as f64 / 63.0),
            })
        } else if data2 & 0x04 != 0 {
            DecodeResult::Command(UnifiedCommand::Move {
                direction: MoveDirection::Left,
                speed: Speed::Normalized(data3 as f64 / 63.0),
            })
        } else if data2 & 0x02 != 0 {
            DecodeResult::Command(UnifiedCommand::Move {
                direction: MoveDirection::Right,
                speed: Speed::Normalized(data3 as f64 / 63.0),
            })
        } else {
            DecodeResult::None
        };

        Ok(result)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: false,
            has_goto: false,
            has_continuous_move: true,
            has_stop: true,
            has_park: false,
            has_presets: false,
            has_speed_control: true,
            supports_equatorial: false,
            supports_altaz: true,
        }
    }
}

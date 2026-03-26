use super::common::checksum_sum;
use super::traits::{Capabilities, Codec, DecodeResult};
use crate::error::{Error, Result};
use crate::model::{MoveDirection, Speed, UnifiedCommand};

const SYNC_BYTE: u8 = 0xFF;
const PACKET_LEN: usize = 7;

pub struct PelcoDCodec {
    address: u8,
    buffer: Vec<u8>,
}

impl PelcoDCodec {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            buffer: Vec::new(),
        }
    }

    fn speed_to_byte(speed: &Speed) -> u8 {
        match speed {
            Speed::Normalized(n) => ((n * 63.0) as u8).min(0x3F),
            Speed::Absolute(d) => ((*d / 2.0) as u8).min(0x3F),
        }
    }

    fn build_packet(&self, cmd1: u8, cmd2: u8, data1: u8, data2: u8) -> Vec<u8> {
        let checksum = checksum_sum(&[self.address, cmd1, cmd2, data1, data2]);
        vec![SYNC_BYTE, self.address, cmd1, cmd2, data1, data2, checksum]
    }
}

impl Default for PelcoDCodec {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Codec for PelcoDCodec {
    fn name(&self) -> &'static str {
        "pelco_d"
    }

    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>> {
        match cmd {
            UnifiedCommand::Stop => Ok(self.build_packet(0x00, 0x00, 0x00, 0x00)),

            UnifiedCommand::Move { direction, speed } => {
                let pan_speed = Self::speed_to_byte(speed);
                let tilt_speed = Self::speed_to_byte(speed);

                let (cmd2, d1, d2) = match direction {
                    MoveDirection::Up => (0x08, 0x00, tilt_speed),
                    MoveDirection::Down => (0x10, 0x00, tilt_speed),
                    MoveDirection::Left => (0x04, pan_speed, 0x00),
                    MoveDirection::Right => (0x02, pan_speed, 0x00),
                    MoveDirection::UpLeft => (0x0C, pan_speed, tilt_speed),
                    MoveDirection::UpRight => (0x0A, pan_speed, tilt_speed),
                    MoveDirection::DownLeft => (0x14, pan_speed, tilt_speed),
                    MoveDirection::DownRight => (0x12, pan_speed, tilt_speed),
                };
                Ok(self.build_packet(0x00, cmd2, d1, d2))
            }

            UnifiedCommand::SetPreset(num) => Ok(self.build_packet(0x00, 0x03, 0x00, *num)),

            UnifiedCommand::GotoPreset(num) => Ok(self.build_packet(0x00, 0x07, 0x00, *num)),

            UnifiedCommand::ClearPreset(num) => Ok(self.build_packet(0x00, 0x05, 0x00, *num)),

            UnifiedCommand::GetPosition => Ok(self.build_packet(0x00, 0x51, 0x00, 0x00)),

            _ => Err(Error::NotSupported(format!("{:?}", cmd))),
        }
    }

    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult> {
        self.buffer.extend_from_slice(data);

        // Find sync byte
        while !self.buffer.is_empty() && self.buffer[0] != SYNC_BYTE {
            self.buffer.remove(0);
        }

        if self.buffer.len() < PACKET_LEN {
            return Ok(DecodeResult::NeedMoreData);
        }

        let packet = &self.buffer[..PACKET_LEN];
        let calculated_checksum = checksum_sum(&packet[1..6]);

        if packet[6] != calculated_checksum {
            self.buffer.remove(0);
            return Err(Error::InvalidData("Checksum mismatch".into()));
        }

        let _addr = packet[1];
        let cmd1 = packet[2];
        let cmd2 = packet[3];
        let data1 = packet[4];
        let data2 = packet[5];

        self.buffer.drain(..PACKET_LEN);

        // Decode command
        let result = if cmd1 == 0x00 && cmd2 == 0x00 {
            DecodeResult::Command(UnifiedCommand::Stop)
        } else if cmd2 & 0x08 != 0 && cmd2 & 0x10 == 0 {
            // Up
            let direction = if cmd2 & 0x04 != 0 {
                MoveDirection::UpLeft
            } else if cmd2 & 0x02 != 0 {
                MoveDirection::UpRight
            } else {
                MoveDirection::Up
            };
            DecodeResult::Command(UnifiedCommand::Move {
                direction,
                speed: Speed::Normalized(data2 as f64 / 63.0),
            })
        } else if cmd2 & 0x10 != 0 {
            // Down
            let direction = if cmd2 & 0x04 != 0 {
                MoveDirection::DownLeft
            } else if cmd2 & 0x02 != 0 {
                MoveDirection::DownRight
            } else {
                MoveDirection::Down
            };
            DecodeResult::Command(UnifiedCommand::Move {
                direction,
                speed: Speed::Normalized(data2 as f64 / 63.0),
            })
        } else if cmd2 & 0x04 != 0 {
            DecodeResult::Command(UnifiedCommand::Move {
                direction: MoveDirection::Left,
                speed: Speed::Normalized(data1 as f64 / 63.0),
            })
        } else if cmd2 & 0x02 != 0 {
            DecodeResult::Command(UnifiedCommand::Move {
                direction: MoveDirection::Right,
                speed: Speed::Normalized(data1 as f64 / 63.0),
            })
        } else if cmd2 == 0x03 {
            DecodeResult::Command(UnifiedCommand::SetPreset(data2))
        } else if cmd2 == 0x07 {
            DecodeResult::Command(UnifiedCommand::GotoPreset(data2))
        } else if cmd2 == 0x05 {
            DecodeResult::Command(UnifiedCommand::ClearPreset(data2))
        } else if cmd2 == 0x51 {
            DecodeResult::Command(UnifiedCommand::GetPosition)
        } else {
            DecodeResult::None
        };

        Ok(result)
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            has_position_query: true,
            has_goto: false,
            has_continuous_move: true,
            has_stop: true,
            has_park: false,
            has_presets: true,
            has_speed_control: true,
            supports_equatorial: false,
            supports_altaz: true,
        }
    }
}

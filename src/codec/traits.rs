use crate::error::Result;
use crate::model::{DeviceStatus, UnifiedCommand};

#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    pub has_position_query: bool,
    pub has_goto: bool,
    pub has_continuous_move: bool,
    pub has_stop: bool,
    pub has_park: bool,
    pub has_presets: bool,
    pub has_speed_control: bool,
    pub supports_equatorial: bool,
    pub supports_altaz: bool,
}

#[derive(Debug, Clone)]
pub enum DecodeResult {
    Command(UnifiedCommand),
    Status(DeviceStatus),
    NeedMoreData,
    None,
}

pub trait Codec: Send + Sync {
    fn name(&self) -> &'static str;
    fn encode(&self, cmd: &UnifiedCommand) -> Result<Vec<u8>>;
    fn decode(&mut self, data: &[u8]) -> Result<DecodeResult>;
    fn capabilities(&self) -> Capabilities;

    fn encode_status(&self, _status: &DeviceStatus) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

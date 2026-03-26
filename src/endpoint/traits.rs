use async_trait::async_trait;

use crate::error::Result;
use crate::model::{DeviceStatus, UnifiedCommand};

#[async_trait]
pub trait Endpoint: Send {
    fn name(&self) -> &str {
        "unknown"
    }
    async fn send_command(&mut self, cmd: &UnifiedCommand) -> Result<()>;
    async fn recv_command(&mut self) -> Result<UnifiedCommand>;
    async fn send_status(&mut self, status: &DeviceStatus) -> Result<()>;
    async fn recv_status(&mut self) -> Result<DeviceStatus>;
    fn is_connected(&self) -> bool;
    async fn close(&mut self) -> Result<()>;
}

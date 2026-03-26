use async_trait::async_trait;
use std::time::Duration;

use super::traits::Endpoint;
use crate::codec::{Codec, DecodeResult};
use crate::error::Result;
use crate::model::{DeviceStatus, UnifiedCommand};
use crate::transport::{is_debug_enabled, Transport};

pub struct TargetEndpoint {
    transport: Box<dyn Transport>,
    codec: Box<dyn Codec>,
    timeout: Duration,
    name: String,
}

impl TargetEndpoint {
    pub fn new(transport: Box<dyn Transport>, codec: Box<dyn Codec>, timeout: Duration) -> Self {
        let name = codec.name().to_string();
        Self {
            transport,
            codec,
            timeout,
            name,
        }
    }
}

#[async_trait]
impl Endpoint for TargetEndpoint {
    fn name(&self) -> &str {
        &self.name
    }

    async fn send_command(&mut self, cmd: &UnifiedCommand) -> Result<()> {
        if is_debug_enabled() {
            println!("  ENCODE CMD: {:?}", cmd);
        }
        let data = self.codec.encode(cmd)?;
        self.transport.send(&data).await
    }

    async fn recv_command(&mut self) -> Result<UnifiedCommand> {
        loop {
            let data = self.transport.recv_timeout(self.timeout).await?;
            match self.codec.decode(&data)? {
                DecodeResult::Command(cmd) => return Ok(cmd),
                DecodeResult::Status(_) => continue,
                DecodeResult::NeedMoreData => continue,
                DecodeResult::None => continue,
            }
        }
    }

    async fn send_status(&mut self, status: &DeviceStatus) -> Result<()> {
        let data = self.codec.encode_status(status)?;
        if !data.is_empty() {
            self.transport.send(&data).await?;
        }
        Ok(())
    }

    async fn recv_status(&mut self) -> Result<DeviceStatus> {
        loop {
            let data = self.transport.recv_timeout(self.timeout).await?;
            let result = self.codec.decode(&data)?;
            if is_debug_enabled() {
                println!("  DECODE STATUS: {:?}", result);
            }
            match result {
                DecodeResult::Status(status) => return Ok(status),
                DecodeResult::Command(_) => continue,
                DecodeResult::NeedMoreData => continue,
                DecodeResult::None => continue,
            }
        }
    }

    fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    async fn close(&mut self) -> Result<()> {
        self.transport.close().await
    }
}

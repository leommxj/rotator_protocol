use async_trait::async_trait;
use std::time::Duration;

use super::traits::Endpoint;
use crate::codec::{Codec, DecodeResult};
use crate::error::Result;
use crate::model::{DeviceStatus, UnifiedCommand};
use crate::transport::{DebugTransport, Transport};

pub struct SourceEndpoint {
    transport: Box<dyn Transport>,
    codec: Box<dyn Codec>,
    timeout: Duration,
    name: String,
}

impl SourceEndpoint {
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
impl Endpoint for SourceEndpoint {
    fn name(&self) -> &str {
        &self.name
    }

    async fn send_command(&mut self, cmd: &UnifiedCommand) -> Result<()> {
        let data = self.codec.encode(cmd)?;
        self.transport.send(&data).await
    }

    async fn recv_command(&mut self) -> Result<UnifiedCommand> {
        loop {
            let data = self.transport.recv_timeout(self.timeout).await?;
            let result = self.codec.decode(&data)?;
            if crate::transport::is_debug_enabled() {
                println!("  DECODE: {:?}", result);
            }
            match result {
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
            match self.codec.decode(&data)? {
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

pub struct SourceServer<S: crate::transport::TransportServer>
where
    S::Connection: 'static,
{
    server: S,
    codec_factory: Box<dyn Fn() -> Box<dyn Codec> + Send + Sync>,
    timeout: Duration,
}

impl<S: crate::transport::TransportServer> SourceServer<S>
where
    S::Connection: 'static,
{
    pub fn new(
        server: S,
        codec_factory: Box<dyn Fn() -> Box<dyn Codec> + Send + Sync>,
        timeout: Duration,
    ) -> Self {
        Self {
            server,
            codec_factory,
            timeout,
        }
    }

    pub async fn accept(&mut self) -> Result<SourceEndpoint> {
        let conn = self.server.accept().await?;
        let transport: Box<dyn Transport> = Box::new(DebugTransport::new(conn, "CLIENT"));
        Ok(SourceEndpoint::new(
            transport,
            (self.codec_factory)(),
            self.timeout,
        ))
    }
}

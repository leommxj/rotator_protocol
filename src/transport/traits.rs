use async_trait::async_trait;
use std::time::Duration;

use crate::error::Result;

#[async_trait]
pub trait Transport: Send {
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self) -> Result<Vec<u8>>;
    async fn recv_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>>;
    fn is_connected(&self) -> bool;
    async fn close(&mut self) -> Result<()>;
}

#[async_trait]
pub trait TransportServer: Send {
    type Connection: Transport;
    async fn accept(&mut self) -> Result<Self::Connection>;
    async fn close(&mut self) -> Result<()>;
}

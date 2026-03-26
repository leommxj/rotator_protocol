use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPort, SerialPortBuilderExt, SerialStream};

use super::traits::Transport;
use crate::error::{Error, Result};

pub struct SerialTransport {
    port: SerialStream,
    buffer: Vec<u8>,
}

impl SerialTransport {
    pub fn new(port_name: &str, baud_rate: u32) -> Result<Self> {
        let port = tokio_serial::new(port_name, baud_rate)
            .open_native_async()
            .map_err(|e| Error::Transport(format!("Failed to open serial port: {}", e)))?;

        Ok(Self {
            port,
            buffer: Vec::with_capacity(256),
        })
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> Result<()> {
        self.port
            .set_timeout(timeout)
            .map_err(|e| Error::Transport(format!("Failed to set timeout: {}", e)))
    }
}

#[async_trait]
impl Transport for SerialTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.port.write_all(data).await?;
        self.port.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        self.buffer.clear();
        let mut buf = [0u8; 256];
        let n = self.port.read(&mut buf).await?;
        if n == 0 {
            return Err(Error::ConnectionClosed);
        }
        self.buffer.extend_from_slice(&buf[..n]);
        Ok(self.buffer.clone())
    }

    async fn recv_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        match tokio::time::timeout(timeout, self.recv()).await {
            Ok(result) => result,
            Err(_) => Err(Error::Timeout),
        }
    }

    fn is_connected(&self) -> bool {
        true
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

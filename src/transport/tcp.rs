use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::traits::{Transport, TransportServer};
use crate::error::{Error, Result};

pub struct TcpTransport {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl TcpTransport {
    pub async fn connect(addr: &str) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self {
            stream,
            buffer: Vec::with_capacity(1024),
        })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: Vec::with_capacity(1024),
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        self.buffer.clear();
        let mut buf = [0u8; 1024];
        let n = self.stream.read(&mut buf).await?;
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
        self.stream.peer_addr().is_ok()
    }

    async fn close(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

pub struct TcpServer {
    listener: TcpListener,
}

impl TcpServer {
    pub async fn bind(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.listener.local_addr().map_err(Error::from)
    }
}

#[async_trait]
impl TransportServer for TcpServer {
    type Connection = TcpTransport;

    async fn accept(&mut self) -> Result<Self::Connection> {
        let (stream, _addr) = self.listener.accept().await?;
        Ok(TcpTransport::from_stream(stream))
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

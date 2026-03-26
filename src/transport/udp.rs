use async_trait::async_trait;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{lookup_host, UdpSocket};
use tokio::sync::Mutex;

use super::traits::{Transport, TransportServer};
use crate::error::{Error, Result};

struct UdpShared {
    socket: Arc<UdpSocket>,
    backlog: Mutex<VecDeque<(SocketAddr, Vec<u8>)>>,
}

impl UdpShared {
    fn new(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            backlog: Mutex::new(VecDeque::new()),
        }
    }

    async fn push_backlog(&self, addr: SocketAddr, data: Vec<u8>) {
        self.backlog.lock().await.push_back((addr, data));
    }

    async fn pop_next_backlog(&self) -> Option<(SocketAddr, Vec<u8>)> {
        self.backlog.lock().await.pop_front()
    }

    async fn pop_backlog_for_peer(&self, peer: SocketAddr) -> Option<Vec<u8>> {
        let mut backlog = self.backlog.lock().await;
        let index = backlog.iter().position(|(addr, _)| *addr == peer)?;
        let (_, data) = backlog.remove(index)?;
        Some(data)
    }
}

async fn resolve_socket_addr(addr: &str) -> Result<SocketAddr> {
    lookup_host(addr)
        .await?
        .next()
        .ok_or_else(|| Error::Transport(format!("Unable to resolve address: {addr}")))
}

pub struct UdpTransport {
    shared: Arc<UdpShared>,
    peer_addr: Option<SocketAddr>,
    buffer: Vec<u8>,
    pending_packet: Option<Vec<u8>>,
    close_on_timeout: bool,
}

impl UdpTransport {
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            shared: Arc::new(UdpShared::new(socket)),
            peer_addr: None,
            buffer: Vec::with_capacity(1024),
            pending_packet: None,
            close_on_timeout: false,
        })
    }

    pub async fn connect(peer_addr: &str) -> Result<Self> {
        let peer = resolve_socket_addr(peer_addr).await?;
        let bind_addr = if peer.is_ipv6() {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        let socket = UdpSocket::bind(bind_addr).await?;
        Ok(Self {
            shared: Arc::new(UdpShared::new(socket)),
            peer_addr: Some(peer),
            buffer: Vec::with_capacity(1024),
            pending_packet: None,
            close_on_timeout: false,
        })
    }

    pub fn set_peer(&mut self, addr: SocketAddr) {
        self.peer_addr = Some(addr);
    }

    fn from_shared(
        shared: Arc<UdpShared>,
        peer_addr: SocketAddr,
        pending_packet: Option<Vec<u8>>,
    ) -> Self {
        Self {
            shared,
            peer_addr: Some(peer_addr),
            buffer: Vec::with_capacity(1024),
            pending_packet,
            close_on_timeout: true,
        }
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        if let Some(peer) = self.peer_addr {
            self.shared.socket.send_to(data, peer).await?;
        } else {
            return Err(Error::Transport("No peer address set".into()));
        }
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        if let Some(data) = self.pending_packet.take() {
            return Ok(data);
        }

        if let Some(peer) = self.peer_addr {
            if let Some(data) = self.shared.pop_backlog_for_peer(peer).await {
                return Ok(data);
            }
        }

        self.buffer.clear();
        loop {
            let mut buf = [0u8; 1024];
            let (n, addr) = self.shared.socket.recv_from(&mut buf).await?;

            match self.peer_addr {
                Some(peer) if peer != addr => {
                    self.shared.push_backlog(addr, buf[..n].to_vec()).await;
                    continue;
                }
                None => self.peer_addr = Some(addr),
                _ => {}
            }

            self.buffer.extend_from_slice(&buf[..n]);
            return Ok(self.buffer.clone());
        }
    }

    async fn recv_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        match tokio::time::timeout(timeout, self.recv()).await {
            Ok(result) => result,
            Err(_) if self.close_on_timeout => Err(Error::ConnectionClosed),
            Err(_) => Err(Error::Timeout),
        }
    }

    fn is_connected(&self) -> bool {
        self.peer_addr.is_some()
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct UdpServer {
    shared: Arc<UdpShared>,
}

impl UdpServer {
    pub async fn bind(addr: &str) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            shared: Arc::new(UdpShared::new(socket)),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.shared.socket.local_addr().map_err(Error::from)
    }
}

#[async_trait]
impl TransportServer for UdpServer {
    type Connection = UdpTransport;

    async fn accept(&mut self) -> Result<Self::Connection> {
        if let Some((addr, data)) = self.shared.pop_next_backlog().await {
            return Ok(UdpTransport::from_shared(
                self.shared.clone(),
                addr,
                Some(data),
            ));
        }

        let mut buf = [0u8; 1024];
        let (n, addr) = self.shared.socket.recv_from(&mut buf).await?;
        Ok(UdpTransport::from_shared(
            self.shared.clone(),
            addr,
            Some(buf[..n].to_vec()),
        ))
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn udp_server_accept_preserves_first_packet() -> Result<()> {
        let mut server = UdpServer::bind("127.0.0.1:0").await?;
        let server_addr = server.local_addr()?;
        let client = UdpSocket::bind("127.0.0.1:0").await?;

        client.send_to(b"first", server_addr).await?;

        let mut conn = server.accept().await?;
        assert_eq!(conn.recv().await?, b"first");

        client.send_to(b"second", server_addr).await?;
        assert_eq!(conn.recv().await?, b"second");

        Ok(())
    }

    #[tokio::test]
    async fn udp_transport_connect_sends_and_receives() -> Result<()> {
        let peer = UdpSocket::bind("127.0.0.1:0").await?;
        let peer_addr = peer.local_addr()?;
        let mut transport = UdpTransport::connect(&peer_addr.to_string()).await?;

        transport.send(b"ping").await?;
        let mut buf = [0u8; 1024];
        let (n, transport_addr) = peer.recv_from(&mut buf).await?;
        assert_eq!(&buf[..n], b"ping");

        peer.send_to(b"pong", transport_addr).await?;
        assert_eq!(transport.recv().await?, b"pong");

        Ok(())
    }
}

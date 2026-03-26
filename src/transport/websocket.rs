use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};

use super::traits::{Transport, TransportServer};
use crate::error::{Error, Result};

enum WsStream {
    Client(WebSocketStream<MaybeTlsStream<TcpStream>>),
    Server(WebSocketStream<TcpStream>),
}

pub struct WebSocketTransport {
    ws: WsStream,
}

impl WebSocketTransport {
    pub async fn connect(url: &str) -> Result<Self> {
        let (ws, _) = connect_async(url)
            .await
            .map_err(|e| Error::Transport(format!("WebSocket connect failed: {}", e)))?;
        Ok(Self {
            ws: WsStream::Client(ws),
        })
    }

    pub async fn from_stream(stream: TcpStream) -> Result<Self> {
        let ws = accept_async(stream)
            .await
            .map_err(|e| Error::Transport(format!("WebSocket accept failed: {}", e)))?;
        Ok(Self {
            ws: WsStream::Server(ws),
        })
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        // Send as Text if data is valid UTF-8, otherwise Binary
        let msg = match std::str::from_utf8(data) {
            Ok(text) => Message::Text(text.to_string()),
            Err(_) => Message::Binary(data.to_vec()),
        };
        match &mut self.ws {
            WsStream::Client(ws) => ws.send(msg).await,
            WsStream::Server(ws) => ws.send(msg).await,
        }
        .map_err(|e| Error::Transport(format!("WebSocket send failed: {}", e)))?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        let next = match &mut self.ws {
            WsStream::Client(ws) => ws.next().await,
            WsStream::Server(ws) => ws.next().await,
        };
        match next {
            Some(Ok(Message::Binary(data))) => Ok(data),
            Some(Ok(Message::Text(text))) => Ok(text.into_bytes()),
            Some(Ok(Message::Close(_))) => Err(Error::ConnectionClosed),
            Some(Err(e)) => Err(Error::Transport(format!("WebSocket recv failed: {}", e))),
            None => Err(Error::ConnectionClosed),
            _ => self.recv().await,
        }
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
        match &mut self.ws {
            WsStream::Client(ws) => ws.close(None).await,
            WsStream::Server(ws) => ws.close(None).await,
        }
        .map_err(|e| Error::Transport(format!("WebSocket close failed: {}", e)))?;
        Ok(())
    }
}

pub struct WebSocketServer {
    listener: TcpListener,
}

impl WebSocketServer {
    pub async fn bind(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { listener })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.listener.local_addr().map_err(Error::from)
    }
}

#[async_trait]
impl TransportServer for WebSocketServer {
    type Connection = WebSocketTransport;

    async fn accept(&mut self) -> Result<Self::Connection> {
        let (stream, _addr) = self.listener.accept().await?;
        WebSocketTransport::from_stream(stream).await
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

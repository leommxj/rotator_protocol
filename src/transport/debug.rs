use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::traits::Transport;
use crate::error::Result;

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_debug_enabled(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::SeqCst)
}

pub fn format_data(data: &[u8], direction: &str, label: &str) {
    if !is_debug_enabled() {
        return;
    }

    let timestamp = chrono_lite();

    // Build hex string
    let hex: String = data
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ");

    // Try to interpret as text, escape control chars
    let text: String = data
        .iter()
        .map(|&b| {
            if b == b'\r' {
                "\\r".to_string()
            } else if b == b'\n' {
                "\\n".to_string()
            } else if b == b'\t' {
                "\\t".to_string()
            } else if b.is_ascii_graphic() || b == b' ' {
                (b as char).to_string()
            } else {
                format!("\\x{:02X}", b)
            }
        })
        .collect();

    println!(
        "[{}] {} {} ({} bytes)\n  TXT: {}\n  HEX: {}",
        timestamp,
        direction,
        label,
        data.len(),
        text,
        hex
    );
}

fn chrono_lite() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();

    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

pub struct DebugTransport<T: Transport> {
    inner: T,
    label: String,
}

impl<T: Transport> DebugTransport<T> {
    pub fn new(inner: T, label: &str) -> Self {
        Self {
            inner,
            label: label.to_string(),
        }
    }
}

#[async_trait]
impl<T: Transport> Transport for DebugTransport<T> {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        format_data(data, ">>>", &self.label);
        self.inner.send(data).await
    }

    async fn recv(&mut self) -> Result<Vec<u8>> {
        let data = self.inner.recv().await?;
        format_data(&data, "<<<", &self.label);
        Ok(data)
    }

    async fn recv_timeout(&mut self, timeout: Duration) -> Result<Vec<u8>> {
        let data = self.inner.recv_timeout(timeout).await?;
        format_data(&data, "<<<", &self.label);
        Ok(data)
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    async fn close(&mut self) -> Result<()> {
        self.inner.close().await
    }
}

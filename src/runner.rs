use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{error, info};

use crate::bridge::Bridge;
use crate::codec::{create_codec, is_supported_protocol};
use crate::config::Config;
use crate::endpoint::{AlpacaDeviceType, AlpacaEndpoint, Endpoint, SourceServer, TargetEndpoint};
use crate::error::{Error, Result};
use crate::model::{AngleConverter, CoordinateConverter, Location};
use crate::transport::{
    DebugTransport, SerialTransport, TcpServer, TcpTransport, TransportServer, UdpServer,
    UdpTransport, WebSocketServer, WebSocketTransport,
};

pub fn is_serial_port(addr: &str) -> bool {
    addr.starts_with("/dev/")
        || addr.starts_with("COM")
        || addr.starts_with("com")
        || addr.starts_with("\\\\.\\COM")
}

pub fn normalize_address(addr: &str) -> String {
    if addr.starts_with(':') {
        format!("0.0.0.0{}", addr)
    } else {
        addr.to_string()
    }
}

pub fn strip_ws_prefix(addr: &str) -> &str {
    addr.strip_prefix("ws://")
        .or_else(|| addr.strip_prefix("wss://"))
        .unwrap_or(addr)
}

pub fn detect_transport(addr: &str) -> &'static str {
    if is_serial_port(addr) {
        "serial"
    } else if addr.starts_with("ws://") || addr.starts_with("wss://") {
        "websocket"
    } else if addr.starts_with("http://") || addr.starts_with("https://") {
        "http"
    } else {
        "tcp"
    }
}

pub fn is_alpaca_protocol(protocol: &str) -> bool {
    matches!(
        protocol.to_lowercase().as_str(),
        "alpaca" | "alpaca-rotator" | "alpaca-telescope" | "ascom"
    )
}

pub fn validate_config(config: &Config) -> Result<()> {
    if config.rotator.protocol.is_empty() {
        return Err(Error::Config("Rotator protocol required".into()));
    }
    if !is_supported_protocol(&config.rotator.protocol) {
        return Err(Error::Config(format!(
            "Unknown rotator protocol: '{}'",
            config.rotator.protocol
        )));
    }
    if config.client.protocol.is_empty() {
        return Err(Error::Config("Client protocol required".into()));
    }
    if !is_supported_protocol(&config.client.protocol) {
        return Err(Error::Config(format!(
            "Unknown client protocol: '{}'",
            config.client.protocol
        )));
    }

    if is_alpaca_protocol(&config.client.protocol) {
        return Err(Error::Config(
            "Alpaca is only supported on the rotator/target side".into(),
        ));
    }

    if let Some(transport) = config.client.transport.as_deref() {
        if !matches!(transport, "tcp" | "udp" | "websocket" | "ws") {
            return Err(Error::Config(format!(
                "Unsupported client transport '{}'; supported client transports are tcp, udp, websocket",
                transport
            )));
        }
    }

    if matches!(config.rotator.transport.as_deref(), Some("http"))
        && !is_alpaca_protocol(&config.rotator.protocol)
    {
        return Err(Error::Config(
            "HTTP transport is only supported for Alpaca rotator targets".into(),
        ));
    }

    Ok(())
}

pub fn prepare_config(config: &mut Config) {
    config.client.address = normalize_address(&config.client.address);

    if config.rotator.transport.is_none() {
        if let Some(ref addr) = config.rotator.address {
            config.rotator.transport = Some(detect_transport(addr).to_string());
        }
    }
    if config.client.transport.is_none() {
        config.client.transport = Some(detect_transport(&config.client.address).to_string());
    }
}

async fn create_rotator(config: &Config) -> Result<Box<dyn Endpoint>> {
    let protocol = config.rotator.protocol.to_lowercase();

    if is_alpaca_protocol(&protocol) {
        let addr = config
            .rotator
            .address
            .as_ref()
            .ok_or_else(|| Error::Config("Rotator address required for Alpaca".into()))?;

        let base_url = if addr.starts_with("http://") || addr.starts_with("https://") {
            addr.clone()
        } else {
            format!("http://{}", addr)
        };

        let device_type = if protocol.contains("telescope") {
            AlpacaDeviceType::Telescope
        } else {
            AlpacaDeviceType::Rotator
        };

        return Ok(Box::new(AlpacaEndpoint::new(&base_url, device_type, 0)));
    }

    let timeout = Duration::from_millis(config.options.timeout_ms);
    let codec = create_codec(&config.rotator.protocol)?;

    let addr = config
        .rotator
        .address
        .as_ref()
        .ok_or_else(|| Error::Config("Rotator address required".into()))?;

    let transport_type = config
        .rotator
        .transport
        .as_deref()
        .unwrap_or_else(|| detect_transport(addr));

    let transport: Box<dyn crate::transport::Transport> = match transport_type {
        "tcp" => {
            let normalized = normalize_address(addr);
            let tcp = TcpTransport::connect(&normalized).await?;
            Box::new(DebugTransport::new(tcp, "ROTATOR-TCP"))
        }
        "serial" => {
            let serial = SerialTransport::new(addr, config.rotator.baudrate)?;
            Box::new(DebugTransport::new(serial, "ROTATOR-SERIAL"))
        }
        "websocket" | "ws" => {
            let ws = WebSocketTransport::connect(addr).await?;
            Box::new(DebugTransport::new(ws, "ROTATOR-WS"))
        }
        "udp" => {
            let udp = UdpTransport::connect(addr).await?;
            Box::new(DebugTransport::new(udp, "ROTATOR-UDP"))
        }
        other => return Err(Error::Config(format!("Unknown transport: {}", other))),
    };

    Ok(Box::new(TargetEndpoint::new(transport, codec, timeout)))
}

pub struct Runner {
    config: Config,
    stop_flag: Arc<AtomicBool>,
    log_tx: Option<mpsc::UnboundedSender<String>>,
}

impl Runner {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            stop_flag: Arc::new(AtomicBool::new(false)),
            log_tx: None,
        }
    }

    pub fn with_log_sender(mut self, tx: mpsc::UnboundedSender<String>) -> Self {
        self.log_tx = Some(tx);
        self
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    fn log(&self, msg: String) {
        if let Some(ref tx) = self.log_tx {
            let _ = tx.send(msg.clone());
        }
        info!("{}", msg);
    }

    pub async fn run(&self) -> Result<()> {
        let timeout = Duration::from_millis(self.config.options.timeout_ms);
        let client_transport = self.config.client.transport.as_deref().unwrap_or("tcp");
        let bind_addr = strip_ws_prefix(&self.config.client.address);

        self.log(format!(
            "Listening on {} (protocol: {}, transport: {})",
            bind_addr, self.config.client.protocol, client_transport
        ));

        match client_transport {
            "websocket" | "ws" => {
                let server = WebSocketServer::bind(bind_addr).await?;
                let client_protocol = self.config.client.protocol.clone();
                let codec_factory = Box::new(move || create_codec(&client_protocol).unwrap());
                let mut client_server = SourceServer::new(server, codec_factory, timeout);
                self.run_server_loop(&mut client_server).await
            }
            "udp" => {
                let server = UdpServer::bind(bind_addr).await?;
                let client_protocol = self.config.client.protocol.clone();
                let codec_factory = Box::new(move || create_codec(&client_protocol).unwrap());
                let mut client_server = SourceServer::new(server, codec_factory, timeout);
                self.run_server_loop(&mut client_server).await
            }
            _ => {
                let server = TcpServer::bind(bind_addr).await?;
                let client_protocol = self.config.client.protocol.clone();
                let codec_factory = Box::new(move || create_codec(&client_protocol).unwrap());
                let mut client_server = SourceServer::new(server, codec_factory, timeout);
                self.run_server_loop(&mut client_server).await
            }
        }
    }

    async fn run_server_loop<S>(&self, client_server: &mut SourceServer<S>) -> Result<()>
    where
        S: TransportServer,
        S::Connection: 'static,
    {
        while !self.stop_flag.load(Ordering::Relaxed) {
            self.log("Waiting for connection...".to_string());

            // Use timeout to check stop flag periodically
            let accept_result =
                tokio::time::timeout(Duration::from_secs(1), client_server.accept()).await;

            let client_endpoint = match accept_result {
                Ok(Ok(ep)) => ep,
                Ok(Err(e)) => {
                    error!("Accept error: {}", e);
                    continue;
                }
                Err(_) => continue, // Timeout, check stop flag
            };

            self.log("Client connected".to_string());

            let rotator = match create_rotator(&self.config).await {
                Ok(t) => t,
                Err(e) => {
                    self.log(format!("Failed to connect to rotator: {}", e));
                    continue;
                }
            };

            self.log(format!(
                "Connected to rotator (protocol: {})",
                self.config.rotator.protocol
            ));

            let angle_converter = AngleConverter::new(
                self.config.rotator.limits.clone(),
                self.config.rotator.offset.clone(),
            );
            let mut bridge = Bridge::new(Box::new(client_endpoint), rotator)
                .with_angle_converter(angle_converter);

            let loc = &self.config.options.location;
            if loc.latitude != 0.0 || loc.longitude != 0.0 {
                let location = Location::new(loc.latitude, loc.longitude);
                bridge = bridge.with_coord_converter(CoordinateConverter::new(location));
                self.log(format!(
                    "Coordinate conversion enabled: lat {:.4}, lon {:.4}",
                    loc.latitude, loc.longitude
                ));
            }

            if let Err(e) = bridge.run().await {
                self.log(format!("Bridge error: {}", e));
            }
            let _ = bridge.close().await;

            self.log("Client disconnected".to_string());
        }

        self.log("Server stopped".to_string());
        Ok(())
    }
}

pub fn list_protocols() -> String {
    let mut s = String::new();
    s.push_str("Supported protocols:\n\n");
    s.push_str("  Rotator/Antenna Control:\n");
    s.push_str("    gs232a, gs-232a  - Yaesu GS-232A (azimuth only)\n");
    s.push_str("    gs232b, gs-232b  - Yaesu GS-232B (azimuth + elevation)\n");
    s.push_str("    gs232, gs-232    - Alias for gs232b\n");
    s.push_str("    rotctld          - Hamlib rotctld network protocol\n");
    s.push_str("    easycomm1        - EasyComm I (basic)\n");
    s.push_str("    easycomm, easycomm2 - EasyComm II (extended)\n");
    s.push_str("    easycomm3        - EasyComm III (with speed control)\n");
    s.push_str("\n  PTZ/Security:\n");
    s.push_str("    pelco_d, pelco-d - Pelco-D PTZ protocol\n");
    s.push_str("    pelco_p, pelco-p - Pelco-P PTZ protocol\n");
    s.push_str("\n  Telescope Control:\n");
    s.push_str("    lx200, meade     - Meade LX200 protocol\n");
    s.push_str("    nexstar          - Celestron NexStar protocol\n");
    s.push_str("    stellarium       - Stellarium telescope protocol\n");
    s.push_str("\n  ASCOM Alpaca (HTTP REST API):\n");
    s.push_str("    alpaca           - ASCOM Alpaca rotator (target only)\n");
    s.push_str("    alpaca-rotator   - ASCOM Alpaca rotator (target only)\n");
    s.push_str("    alpaca-telescope - ASCOM Alpaca telescope (target only)\n");
    s.push_str("\n  INDI (XML over TCP, port 7624):\n");
    s.push_str("    indi, indilib    - INDI protocol (Linux astronomy)\n");
    s.push_str("\nSupported client/source transports:\n");
    s.push_str("    tcp              - TCP/IP listener\n");
    s.push_str("    udp              - UDP listener\n");
    s.push_str("    websocket        - WebSocket listener\n");
    s.push_str("\nSupported rotator/target transports:\n");
    s.push_str("    serial           - Serial port (RS-232/RS-485)\n");
    s.push_str("    tcp              - TCP/IP client\n");
    s.push_str("    udp              - UDP peer\n");
    s.push_str("    websocket        - WebSocket client\n");
    s.push_str("    http             - HTTP (Alpaca target only)\n");
    s.push_str("\nNotes:\n");
    s.push_str("    - UDP is supported on both sides but must be selected explicitly.\n");
    s.push_str("    - Serial is rotator/target only.\n");
    s.push_str("    - Alpaca is rotator/target only.\n");
    s
}

use iced::widget::{column, container, pick_list, row, text, text_input, Column};
use iced::Element;

#[derive(Debug, Clone)]
pub struct ConfigPanel {
    pub rotator_protocol: String,
    pub rotator_transport: String,
    pub rotator_address: String,
    pub client_protocol: String,
    pub client_transport: String,
    pub client_address: String,
}

#[derive(Debug, Clone)]
pub enum ConfigMessage {
    RotatorProtocolChanged(String),
    RotatorTransportChanged(String),
    RotatorAddressChanged(String),
    ClientProtocolChanged(String),
    ClientTransportChanged(String),
    ClientAddressChanged(String),
}

impl Default for ConfigPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigPanel {
    pub fn new() -> Self {
        Self {
            rotator_protocol: "gs232b".to_string(),
            rotator_transport: "tcp".to_string(),
            rotator_address: "127.0.0.1:4000".to_string(),
            client_protocol: "gs232b".to_string(),
            client_transport: "tcp".to_string(),
            client_address: "0.0.0.0:4533".to_string(),
        }
    }

    pub fn update(&mut self, message: ConfigMessage) {
        match message {
            ConfigMessage::RotatorProtocolChanged(v) => self.rotator_protocol = v,
            ConfigMessage::RotatorTransportChanged(v) => self.rotator_transport = v,
            ConfigMessage::RotatorAddressChanged(v) => self.rotator_address = v,
            ConfigMessage::ClientProtocolChanged(v) => self.client_protocol = v,
            ConfigMessage::ClientTransportChanged(v) => self.client_transport = v,
            ConfigMessage::ClientAddressChanged(v) => self.client_address = v,
        }
    }

    pub fn view(&self) -> Element<ConfigMessage> {
        let protocols: Vec<&str> = vec![
            "gs232a",
            "gs232b",
            "rotctld",
            "easycomm",
            "easycomm2",
            "easycomm3",
            "pelco_d",
            "pelco_p",
            "lx200",
            "nexstar",
            "stellarium",
            "alpaca",
            "indi",
        ];
        let transports: Vec<&str> = vec!["tcp", "serial", "websocket"];

        let rotator_section = column![
            text("Rotator (Device)").size(16),
            row![
                text("Protocol:").width(80),
                pick_list(
                    protocols.clone(),
                    Some(self.rotator_protocol.as_str()),
                    |s| ConfigMessage::RotatorProtocolChanged(s.to_string())
                )
                .width(120),
            ]
            .spacing(10),
            row![
                text("Transport:").width(80),
                pick_list(
                    transports.clone(),
                    Some(self.rotator_transport.as_str()),
                    |s| ConfigMessage::RotatorTransportChanged(s.to_string())
                )
                .width(120),
            ]
            .spacing(10),
            row![
                text("Address:").width(80),
                text_input("host:port", &self.rotator_address)
                    .on_input(ConfigMessage::RotatorAddressChanged)
                    .width(200),
            ]
            .spacing(10),
        ]
        .spacing(5);

        let client_section = column![
            text("Client (Listen)").size(16),
            row![
                text("Protocol:").width(80),
                pick_list(protocols, Some(self.client_protocol.as_str()), |s| {
                    ConfigMessage::ClientProtocolChanged(s.to_string())
                })
                .width(120),
            ]
            .spacing(10),
            row![
                text("Transport:").width(80),
                pick_list(transports, Some(self.client_transport.as_str()), |s| {
                    ConfigMessage::ClientTransportChanged(s.to_string())
                })
                .width(120),
            ]
            .spacing(10),
            row![
                text("Address:").width(80),
                text_input("host:port", &self.client_address)
                    .on_input(ConfigMessage::ClientAddressChanged)
                    .width(200),
            ]
            .spacing(10),
        ]
        .spacing(5);

        container(
            Column::new()
                .push(rotator_section)
                .push(text(""))
                .push(client_section)
                .spacing(10),
        )
        .padding(10)
        .into()
    }
}

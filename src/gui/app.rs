use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use iced::widget::{button, container, row, text, Column};
use iced::{Element, Length, Subscription, Task};
use tokio::sync::mpsc;

use crate::config::{
    ClientConfig, Config, LocationConfig, Options, RotatorConfig, RotatorLimits, RotatorOffset,
};
use crate::runner::{prepare_config, validate_config, Runner};
use crate::transport::set_debug_enabled;

use super::config_panel::{ConfigMessage, ConfigPanel};
use super::logger::{Logger, LoggerMessage};

pub struct App {
    config_panel: ConfigPanel,
    logger: Logger,
    running: bool,
    stop_flag: Option<Arc<AtomicBool>>,
    log_rx: Option<mpsc::UnboundedReceiver<String>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Config(ConfigMessage),
    Logger(LoggerMessage),
    Start,
    Stop,
    Tick,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                config_panel: ConfigPanel::new(),
                logger: Logger::new(),
                running: false,
                stop_flag: None,
                log_rx: None,
            },
            Task::none(),
        )
    }

    fn build_config(&self) -> Result<Config, String> {
        let panel = &self.config_panel;

        let mut config = Config {
            rotator: RotatorConfig {
                protocol: panel.rotator_protocol.clone(),
                transport: Some(panel.rotator_transport.clone()),
                address: Some(panel.rotator_address.clone()),
                baudrate: 9600,
                limits: RotatorLimits::default(),
                offset: RotatorOffset::default(),
            },
            client: ClientConfig {
                protocol: panel.client_protocol.clone(),
                transport: Some(panel.client_transport.clone()),
                address: panel.client_address.clone(),
            },
            options: Options {
                coordinate_system: "altaz".to_string(),
                timeout_ms: 5000,
                location: LocationConfig::default(),
            },
        };

        prepare_config(&mut config);
        validate_config(&config).map_err(|e| e.to_string())?;

        Ok(config)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Config(msg) => {
                if !self.running {
                    self.config_panel.update(msg);
                }
            }
            Message::Logger(msg) => {
                self.logger.update(msg);
            }
            Message::Start => {
                if self.running {
                    return Task::none();
                }

                self.logger.update(LoggerMessage::Clear);

                let config = match self.build_config() {
                    Ok(c) => c,
                    Err(e) => {
                        self.logger
                            .update(LoggerMessage::AddLine(format!("Config error: {}", e)));
                        return Task::none();
                    }
                };

                set_debug_enabled(true);

                let (log_tx, log_rx) = mpsc::unbounded_channel();
                self.log_rx = Some(log_rx);

                let runner = Runner::new(config).with_log_sender(log_tx);
                let stop_flag = runner.stop_handle();
                self.stop_flag = Some(stop_flag);

                self.running = true;
                self.logger
                    .update(LoggerMessage::AddLine("Starting server...".to_string()));

                tokio::spawn(async move {
                    if let Err(e) = runner.run().await {
                        tracing::error!("Runner error: {}", e);
                    }
                });
            }
            Message::Stop => {
                if let Some(ref flag) = self.stop_flag {
                    flag.store(true, Ordering::Relaxed);
                }
                self.running = false;
                self.stop_flag = None;
                self.log_rx = None;
                self.logger
                    .update(LoggerMessage::AddLine("Stopping...".to_string()));
            }
            Message::Tick => {
                if let Some(ref mut rx) = self.log_rx {
                    while let Ok(line) = rx.try_recv() {
                        self.logger.update(LoggerMessage::AddLine(line));
                    }
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let config_panel = self.config_panel.view().map(Message::Config);

        let control_button = if self.running {
            button(text("Stop")).on_press(Message::Stop).width(80)
        } else {
            button(text("Start")).on_press(Message::Start).width(80)
        };

        let status = if self.running {
            text("Running").style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.0, 0.8, 0.0)),
            })
        } else {
            text("Stopped").style(|_| text::Style {
                color: Some(iced::Color::from_rgb(0.8, 0.0, 0.0)),
            })
        };

        let control_row = row![control_button, status].spacing(20).padding(10);

        let log_panel = container(self.logger.view().map(Message::Logger))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.1, 0.1, 0.1,
                ))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.3, 0.3, 0.3),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let content = Column::new()
            .push(config_panel)
            .push(control_row)
            .push(text("Log:").size(14))
            .push(log_panel)
            .spacing(5)
            .padding(10);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.running {
            iced::time::every(std::time::Duration::from_millis(100)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }
}

pub fn run_gui() -> iced::Result {
    iced::application("Rotator Protocol", App::update, App::view)
        .subscription(App::subscription)
        .run_with(App::new)
}

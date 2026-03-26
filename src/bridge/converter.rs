use tracing::{debug, error, info, warn};

use crate::endpoint::Endpoint;
use crate::error::{Error, Result};
use crate::model::{
    AngleConverter, CoordinateConverter, CoordinateSystem, DeviceStatus, Position, UnifiedCommand,
};

pub struct Bridge {
    source: Box<dyn Endpoint>,
    target: Box<dyn Endpoint>,
    current_status: DeviceStatus,
    angle_converter: Option<AngleConverter>,
    coord_converter: Option<CoordinateConverter>,
}

impl Bridge {
    pub fn new(source: Box<dyn Endpoint>, target: Box<dyn Endpoint>) -> Self {
        Self {
            source,
            target,
            current_status: DeviceStatus::default(),
            angle_converter: None,
            coord_converter: None,
        }
    }

    pub fn with_angle_converter(mut self, converter: AngleConverter) -> Self {
        self.angle_converter = Some(converter);
        self
    }

    pub fn with_coord_converter(mut self, converter: CoordinateConverter) -> Self {
        self.coord_converter = Some(converter);
        self
    }

    fn to_horizontal(&self, pos: Position) -> Position {
        if pos.coord_system == CoordinateSystem::Equatorial {
            if let Some(ref converter) = self.coord_converter {
                let converted = converter.equatorial_to_horizontal(pos);
                debug!(
                    "Coord convert: RA {:.4}h Dec {:.2}° -> Az {:.2}° El {:.2}°",
                    pos.ra(),
                    pos.dec(),
                    converted.azimuth(),
                    converted.elevation()
                );
                return converted;
            }
        }
        pos
    }

    fn to_equatorial(&self, pos: Position) -> Position {
        if pos.coord_system == CoordinateSystem::AltAz {
            if let Some(ref converter) = self.coord_converter {
                return converter.horizontal_to_equatorial(pos);
            }
        }
        pos
    }

    fn convert_position(&self, pos: Position) -> Result<Position> {
        let pos = self.to_horizontal(pos);
        if let Some(ref converter) = self.angle_converter {
            converter.convert(pos)
        } else {
            Ok(pos)
        }
    }

    fn reverse_position(&self, pos: Position) -> Position {
        let pos = if let Some(ref converter) = self.angle_converter {
            converter.reverse(pos)
        } else {
            pos
        };
        self.to_equatorial(pos)
    }

    fn convert_command(&self, cmd: UnifiedCommand) -> Result<UnifiedCommand> {
        match cmd {
            UnifiedCommand::GotoPosition(pos) => {
                let converted = self.convert_position(pos)?;
                Ok(UnifiedCommand::GotoPosition(converted))
            }
            other => Ok(other),
        }
    }

    fn convert_status(&self, mut status: DeviceStatus) -> DeviceStatus {
        if let Some(ref pos) = status.position {
            status.position = Some(self.reverse_position(*pos));
        }
        status
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(
            "Bridge started: {} <-> {}",
            self.source.name(),
            self.target.name()
        );

        if let Some(ref converter) = self.angle_converter {
            let limits = converter.limits();
            let offset = converter.offset();
            info!(
                "Angle limits: Az [{:.1}, {:.1}], El [{:.1}, {:.1}], Offset: Az {:.1}, El {:.1}",
                limits.azimuth_min,
                limits.azimuth_max,
                limits.elevation_min,
                limits.elevation_max,
                offset.azimuth,
                offset.elevation
            );
        }

        if self.coord_converter.is_some() {
            info!("Coordinate conversion enabled (Equatorial <-> Horizontal)");
        }

        loop {
            if !self.source.is_connected() {
                warn!("Source disconnected");
                return Err(Error::ConnectionClosed);
            }

            let cmd = match self.source.recv_command().await {
                Ok(cmd) => cmd,
                Err(Error::Timeout) => continue,
                Err(Error::ConnectionClosed) => {
                    info!("Source connection closed");
                    return Ok(());
                }
                Err(e) => {
                    error!("Source recv error: {}", e);
                    continue;
                }
            };

            debug!("Received command: {:?}", cmd);

            match self.handle_command(cmd).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Command handling error: {}", e);
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: UnifiedCommand) -> Result<()> {
        match &cmd {
            UnifiedCommand::GetPosition => {
                match self.target.send_command(&cmd).await {
                    Ok(()) => {}
                    Err(Error::NotSupported(_)) => {}
                    Err(e) => return Err(e),
                }

                match self.target.recv_status().await {
                    Ok(status) => {
                        let status = self.convert_status(status);
                        self.current_status = status.clone();
                        self.source.send_status(&status).await?;
                    }
                    Err(e) => {
                        warn!("Failed to get position: {}", e);
                        self.source.send_status(&self.current_status).await?;
                    }
                }
            }

            UnifiedCommand::GotoPosition(_) => match self.convert_command(cmd) {
                Ok(converted_cmd) => {
                    debug!("Converted command: {:?}", converted_cmd);
                    self.target.send_command(&converted_cmd).await?;
                    self.source.send_status(&DeviceStatus::ok()).await?;
                }
                Err(e) => {
                    warn!("Position conversion failed: {}", e);
                    self.source
                        .send_status(&DeviceStatus::error(-1, &e.to_string()))
                        .await?;
                }
            },

            UnifiedCommand::Move { .. } | UnifiedCommand::Stop | UnifiedCommand::Park => {
                self.target.send_command(&cmd).await?;
                self.source.send_status(&DeviceStatus::ok()).await?;
            }

            _ => match self.target.send_command(&cmd).await {
                Ok(()) => {
                    self.source.send_status(&DeviceStatus::ok()).await?;
                }
                Err(e) => {
                    warn!("Unsupported command: {:?}, error: {}", cmd, e);
                    self.source
                        .send_status(&DeviceStatus::error(-1, "Unsupported"))
                        .await?;
                }
            },
        }

        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        let _ = self.source.close().await;
        let _ = self.target.close().await;
        Ok(())
    }
}

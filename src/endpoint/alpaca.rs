use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use crate::endpoint::Endpoint;
use crate::error::{Error, Result};
use crate::model::{DeviceStatus, MotionState, MoveDirection, Position, Speed, UnifiedCommand};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
struct AlpacaResponse<T> {
    value: Option<T>,
    client_transaction_id: u32,
    server_transaction_id: u32,
    error_number: i32,
    error_message: String,
}

#[derive(Debug, Clone)]
pub enum AlpacaDeviceType {
    Rotator,
    Telescope,
}

pub struct AlpacaEndpoint {
    client: Client,
    base_url: String,
    device_type: AlpacaDeviceType,
    device_number: u32,
    client_id: u32,
    transaction_id: AtomicU32,
}

impl AlpacaEndpoint {
    pub fn new(base_url: &str, device_type: AlpacaDeviceType, device_number: u32) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            base_url: base_url.trim_end_matches('/').to_string(),
            device_type,
            device_number,
            client_id: 1,
            transaction_id: AtomicU32::new(1),
        }
    }

    pub fn rotator(base_url: &str, device_number: u32) -> Self {
        Self::new(base_url, AlpacaDeviceType::Rotator, device_number)
    }

    pub fn telescope(base_url: &str, device_number: u32) -> Self {
        Self::new(base_url, AlpacaDeviceType::Telescope, device_number)
    }

    fn next_transaction_id(&self) -> u32 {
        self.transaction_id.fetch_add(1, Ordering::SeqCst)
    }

    fn device_path(&self) -> String {
        let type_name = match self.device_type {
            AlpacaDeviceType::Rotator => "rotator",
            AlpacaDeviceType::Telescope => "telescope",
        };
        format!(
            "{}/api/v1/{}/{}",
            self.base_url, type_name, self.device_number
        )
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, endpoint: &str) -> Result<T> {
        let url = format!(
            "{}{}?ClientID={}&ClientTransactionID={}",
            self.device_path(),
            endpoint,
            self.client_id,
            self.next_transaction_id()
        );

        let resp: AlpacaResponse<T> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("HTTP request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| Error::Codec(format!("JSON parse failed: {}", e)))?;

        if resp.error_number != 0 {
            return Err(Error::Protocol(format!(
                "Alpaca error {}: {}",
                resp.error_number, resp.error_message
            )));
        }

        resp.value
            .ok_or_else(|| Error::Protocol("No value in response".into()))
    }

    async fn put(&self, endpoint: &str, params: &[(&str, String)]) -> Result<()> {
        let url = format!("{}{}", self.device_path(), endpoint);

        let mut form = vec![
            ("ClientID", self.client_id.to_string()),
            (
                "ClientTransactionID",
                self.next_transaction_id().to_string(),
            ),
        ];
        for (k, v) in params {
            form.push((k, v.clone()));
        }

        let resp: AlpacaResponse<()> = self
            .client
            .put(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("HTTP request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| Error::Codec(format!("JSON parse failed: {}", e)))?;

        if resp.error_number != 0 {
            return Err(Error::Protocol(format!(
                "Alpaca error {}: {}",
                resp.error_number, resp.error_message
            )));
        }

        Ok(())
    }

    // Rotator methods
    async fn get_rotator_position(&self) -> Result<f64> {
        self.get("/position").await
    }

    async fn rotator_move_absolute(&self, position: f64) -> Result<()> {
        self.put("/moveabsolute", &[("Position", position.to_string())])
            .await
    }

    async fn rotator_halt(&self) -> Result<()> {
        self.put("/halt", &[]).await
    }

    async fn is_rotator_moving(&self) -> Result<bool> {
        self.get("/ismoving").await
    }

    // Telescope methods
    async fn get_telescope_position(&self) -> Result<Position> {
        let az: f64 = self.get("/azimuth").await?;
        let alt: f64 = self.get("/altitude").await?;
        Ok(Position::new_altaz(az, alt))
    }

    #[allow(dead_code)]
    async fn get_telescope_radec(&self) -> Result<Position> {
        let ra: f64 = self.get("/rightascension").await?;
        let dec: f64 = self.get("/declination").await?;
        Ok(Position::new_equatorial(ra, dec))
    }

    async fn telescope_slew_altaz(&self, alt: f64, az: f64) -> Result<()> {
        self.put(
            "/slewtoaltazasync",
            &[("Altitude", alt.to_string()), ("Azimuth", az.to_string())],
        )
        .await
    }

    async fn telescope_slew_radec(&self, ra: f64, dec: f64) -> Result<()> {
        self.put(
            "/slewtocoordinatesasync",
            &[
                ("RightAscension", ra.to_string()),
                ("Declination", dec.to_string()),
            ],
        )
        .await
    }

    async fn telescope_abort(&self) -> Result<()> {
        self.put("/abortslew", &[]).await
    }

    async fn telescope_move_axis(&self, axis: i32, rate: f64) -> Result<()> {
        self.put(
            "/moveaxis",
            &[("Axis", axis.to_string()), ("Rate", rate.to_string())],
        )
        .await
    }

    async fn is_telescope_slewing(&self) -> Result<bool> {
        self.get("/slewing").await
    }
}

#[async_trait]
impl Endpoint for AlpacaEndpoint {
    fn name(&self) -> &str {
        match self.device_type {
            AlpacaDeviceType::Rotator => "alpaca-rotator",
            AlpacaDeviceType::Telescope => "alpaca-telescope",
        }
    }

    async fn send_command(&mut self, cmd: &UnifiedCommand) -> Result<()> {
        match &self.device_type {
            AlpacaDeviceType::Rotator => match cmd {
                UnifiedCommand::GotoPosition(pos) => {
                    self.rotator_move_absolute(pos.azimuth()).await
                }
                UnifiedCommand::Stop => self.rotator_halt().await,
                _ => Err(Error::NotSupported(format!("{:?}", cmd))),
            },
            AlpacaDeviceType::Telescope => match cmd {
                UnifiedCommand::GotoPosition(pos) => {
                    if pos.coord_system == crate::model::CoordinateSystem::Equatorial {
                        self.telescope_slew_radec(pos.ra(), pos.dec()).await
                    } else {
                        self.telescope_slew_altaz(pos.elevation(), pos.azimuth())
                            .await
                    }
                }
                UnifiedCommand::Stop => self.telescope_abort().await,
                UnifiedCommand::Move { direction, speed } => {
                    let rate = match speed {
                        Speed::Normalized(n) => n * 5.0,
                        Speed::Absolute(d) => *d,
                    };
                    match direction {
                        MoveDirection::Left => self.telescope_move_axis(0, -rate).await,
                        MoveDirection::Right => self.telescope_move_axis(0, rate).await,
                        MoveDirection::Up => self.telescope_move_axis(1, rate).await,
                        MoveDirection::Down => self.telescope_move_axis(1, -rate).await,
                        _ => Err(Error::NotSupported("Diagonal movement".into())),
                    }
                }
                _ => Err(Error::NotSupported(format!("{:?}", cmd))),
            },
        }
    }

    async fn recv_command(&mut self) -> Result<UnifiedCommand> {
        // Alpaca is client-only, cannot receive commands
        Err(Error::NotSupported(
            "Alpaca endpoint cannot receive commands".into(),
        ))
    }

    async fn send_status(&mut self, _status: &DeviceStatus) -> Result<()> {
        // Alpaca is client-only
        Err(Error::NotSupported(
            "Alpaca endpoint cannot send status".into(),
        ))
    }

    async fn recv_status(&mut self) -> Result<DeviceStatus> {
        match &self.device_type {
            AlpacaDeviceType::Rotator => {
                let position = self.get_rotator_position().await?;
                let is_moving = self.is_rotator_moving().await.unwrap_or(false);
                Ok(DeviceStatus {
                    position: Some(Position::new_altaz(position, 0.0)),
                    target_position: None,
                    motion_state: if is_moving {
                        MotionState::Moving
                    } else {
                        MotionState::Idle
                    },
                    error_code: None,
                    error_message: None,
                })
            }
            AlpacaDeviceType::Telescope => {
                let position = self.get_telescope_position().await?;
                let is_slewing = self.is_telescope_slewing().await.unwrap_or(false);
                Ok(DeviceStatus {
                    position: Some(position),
                    target_position: None,
                    motion_state: if is_slewing {
                        MotionState::Moving
                    } else {
                        MotionState::Idle
                    },
                    error_code: None,
                    error_message: None,
                })
            }
        }
    }

    fn is_connected(&self) -> bool {
        true
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

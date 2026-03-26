use serde::Deserialize;
use std::path::Path;

use crate::error::{Error, Result};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub rotator: RotatorConfig,
    pub client: ClientConfig,
    #[serde(default)]
    pub options: Options,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RotatorConfig {
    pub protocol: String,
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default = "default_baudrate")]
    pub baudrate: u32,
    #[serde(default)]
    pub limits: RotatorLimits,
    #[serde(default)]
    pub offset: RotatorOffset,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RotatorLimits {
    #[serde(default = "default_az_min")]
    pub azimuth_min: f64,
    #[serde(default = "default_az_max")]
    pub azimuth_max: f64,
    #[serde(default = "default_el_min")]
    pub elevation_min: f64,
    #[serde(default = "default_el_max")]
    pub elevation_max: f64,
}

impl Default for RotatorLimits {
    fn default() -> Self {
        Self {
            azimuth_min: default_az_min(),
            azimuth_max: default_az_max(),
            elevation_min: default_el_min(),
            elevation_max: default_el_max(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct RotatorOffset {
    #[serde(default)]
    pub azimuth: f64,
    #[serde(default)]
    pub elevation: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ClientConfig {
    pub protocol: String,
    #[serde(default)]
    pub transport: Option<String>,
    #[serde(default = "default_listen")]
    pub address: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Options {
    #[serde(default = "default_coordinate_system")]
    pub coordinate_system: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub location: LocationConfig,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LocationConfig {
    #[serde(default)]
    pub latitude: f64,
    #[serde(default)]
    pub longitude: f64,
}

fn default_listen() -> String {
    "0.0.0.0:4533".to_string()
}

fn default_baudrate() -> u32 {
    9600
}

fn default_coordinate_system() -> String {
    "altaz".to_string()
}

fn default_timeout() -> u64 {
    5000
}

fn default_az_min() -> f64 {
    0.0
}

fn default_az_max() -> f64 {
    360.0
}

fn default_el_min() -> f64 {
    0.0
}

fn default_el_max() -> f64 {
    90.0
}

pub fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Config(format!("Failed to read config: {}", e)))?;

    toml::from_str(&content).map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))
}

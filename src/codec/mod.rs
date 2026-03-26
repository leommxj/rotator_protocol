mod common;
mod easycomm;
mod gs232;
mod indi;
mod lx200;
mod nexstar;
mod pelco_d;
mod pelco_p;
mod rotctld;
mod stellarium;
mod traits;

pub use common::*;
pub use easycomm::EasycommCodec;
pub use gs232::{Gs232Codec, Gs232Variant};
pub use indi::IndiCodec;
pub use lx200::Lx200Codec;
pub use nexstar::NexstarCodec;
pub use pelco_d::PelcoDCodec;
pub use pelco_p::PelcoPCodec;
pub use rotctld::RotctldCodec;
pub use stellarium::StellariumCodec;
pub use traits::{Capabilities, Codec, DecodeResult};

use crate::error::{Error, Result};

const SUPPORTED_PROTOCOLS: &[&str] = &[
    "gs232",
    "gs-232",
    "gs232a",
    "gs-232a",
    "gs232b",
    "gs-232b",
    "yaesu",
    "rotctld",
    "rotctl",
    "hamlib",
    "pelco_d",
    "pelco-d",
    "pelcod",
    "pelco_p",
    "pelco-p",
    "pelcop",
    "easycomm",
    "easycomm1",
    "easycomm2",
    "easycomm3",
    "stellarium",
    "lx200",
    "meade",
    "nexstar",
    "celestron",
    "indi",
    "indilib",
    "alpaca",
    "alpaca-rotator",
    "alpaca-telescope",
    "ascom",
];

pub fn is_supported_protocol(name: &str) -> bool {
    SUPPORTED_PROTOCOLS.contains(&name.to_lowercase().as_str())
}

pub fn create_codec(name: &str) -> Result<Box<dyn Codec>> {
    match name.to_lowercase().as_str() {
        // GS-232 variants
        "gs232" | "gs-232" | "gs232b" | "gs-232b" | "yaesu" => Ok(Box::new(Gs232Codec::new_b())),
        "gs232a" | "gs-232a" => Ok(Box::new(Gs232Codec::new_a())),
        // Hamlib
        "rotctld" | "rotctl" | "hamlib" => Ok(Box::new(RotctldCodec::new())),
        // Pelco
        "pelco_d" | "pelco-d" | "pelcod" => Ok(Box::new(PelcoDCodec::new(1))),
        "pelco_p" | "pelco-p" | "pelcop" => Ok(Box::new(PelcoPCodec::new(0))),
        // EasyComm variants
        "easycomm" | "easycomm2" => Ok(Box::new(EasycommCodec::new(2))),
        "easycomm1" => Ok(Box::new(EasycommCodec::new(1))),
        "easycomm3" => Ok(Box::new(EasycommCodec::new(3))),
        // Telescope protocols
        "stellarium" => Ok(Box::new(StellariumCodec::new())),
        "lx200" | "meade" => Ok(Box::new(Lx200Codec::new())),
        "nexstar" | "celestron" => Ok(Box::new(NexstarCodec::new())),
        // INDI
        "indi" | "indilib" => Ok(Box::new(IndiCodec::new())),
        _ => Err(Error::Config(format!("Unknown codec: {}", name))),
    }
}

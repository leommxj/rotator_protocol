use std::path::PathBuf;

use clap::Parser;
use tracing::info;

use rotator_protocol::config::{
    load_config, ClientConfig, Config, LocationConfig, Options, RotatorConfig, RotatorLimits,
    RotatorOffset,
};
use rotator_protocol::runner::{
    is_alpaca_protocol, list_protocols, prepare_config, validate_config, Runner,
};
use rotator_protocol::transport::set_debug_enabled;
use rotator_protocol::{Error, Result};

#[derive(Parser, Debug)]
#[command(author, version, about = "Rotator Protocol Converter", long_about = None)]
pub struct Args {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short = 'R', long, visible_alias = "rp", value_name = "PROTOCOL")]
    rotator_protocol: Option<String>,

    #[arg(long, visible_alias = "rt", value_name = "TYPE")]
    rotator_transport: Option<String>,

    #[arg(long, visible_alias = "ra", value_name = "ADDR")]
    rotator_address: Option<String>,

    #[arg(short, long, default_value = "9600")]
    baudrate: u32,

    #[arg(long, value_name = "MIN:MAX")]
    az_range: Option<String>,

    #[arg(long, value_name = "MIN:MAX")]
    el_range: Option<String>,

    #[arg(long, default_value = "0")]
    az_offset: f64,

    #[arg(long, default_value = "0")]
    el_offset: f64,

    #[arg(long = "lat", value_name = "DEG")]
    latitude: Option<f64>,

    #[arg(long = "lon", value_name = "DEG")]
    longitude: Option<f64>,

    #[arg(short = 'C', long, visible_alias = "cp", value_name = "PROTOCOL")]
    client_protocol: Option<String>,

    #[arg(long, visible_alias = "ct", value_name = "TYPE")]
    client_transport: Option<String>,

    #[arg(long, visible_alias = "ca", value_name = "ADDR")]
    client_address: Option<String>,

    #[arg(short, long, default_value = "5000")]
    timeout: u64,

    #[arg(long, visible_alias = "list")]
    list_protocols: bool,

    #[arg(short = 'D', long)]
    debug: bool,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable GUI (CLI mode only, requires --rp and --cp)
    #[cfg(feature = "gui")]
    #[arg(long)]
    no_gui: bool,
}

fn setup_logging(verbose: u8) {
    let level = match verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(level.into()),
        )
        .init();
}

fn parse_range(s: &str) -> Result<(f64, f64)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(Error::Config(format!(
            "Invalid range format '{}', expected 'min:max'",
            s
        )));
    }
    let min = parts[0]
        .parse()
        .map_err(|_| Error::Config(format!("Invalid min value: {}", parts[0])))?;
    let max = parts[1]
        .parse()
        .map_err(|_| Error::Config(format!("Invalid max value: {}", parts[1])))?;
    Ok((min, max))
}

pub fn build_config(args: &Args) -> Result<Config> {
    let mut config = if let Some(ref path) = args.config {
        load_config(path)?
    } else if PathBuf::from("config.toml").exists() {
        load_config(&PathBuf::from("config.toml")).unwrap_or_else(|_| default_config())
    } else {
        default_config()
    };

    if let Some(ref proto) = args.rotator_protocol {
        config.rotator.protocol = proto.clone();
    }
    if let Some(ref addr) = args.rotator_address {
        config.rotator.address = Some(addr.clone());
    }
    if let Some(ref transport) = args.rotator_transport {
        config.rotator.transport = Some(transport.clone());
    }
    if args.baudrate != 9600 {
        config.rotator.baudrate = args.baudrate;
    }

    if let Some(ref az_range) = args.az_range {
        let (min, max) = parse_range(az_range)?;
        config.rotator.limits.azimuth_min = min;
        config.rotator.limits.azimuth_max = max;
    }
    if let Some(ref el_range) = args.el_range {
        let (min, max) = parse_range(el_range)?;
        config.rotator.limits.elevation_min = min;
        config.rotator.limits.elevation_max = max;
    }
    if args.az_offset != 0.0 {
        config.rotator.offset.azimuth = args.az_offset;
    }
    if args.el_offset != 0.0 {
        config.rotator.offset.elevation = args.el_offset;
    }

    if let Some(ref proto) = args.client_protocol {
        config.client.protocol = proto.clone();
    }
    if let Some(ref addr) = args.client_address {
        config.client.address = addr.clone();
    }
    if let Some(ref transport) = args.client_transport {
        config.client.transport = Some(transport.clone());
    }

    if args.timeout != 5000 {
        config.options.timeout_ms = args.timeout;
    }

    if let Some(lat) = args.latitude {
        config.options.location.latitude = lat;
    }
    if let Some(lon) = args.longitude {
        config.options.location.longitude = lon;
    }

    prepare_config(&mut config);
    validate_config(&config)?;

    Ok(config)
}

fn default_config() -> Config {
    Config {
        rotator: RotatorConfig {
            protocol: String::new(),
            transport: None,
            address: None,
            baudrate: 9600,
            limits: RotatorLimits::default(),
            offset: RotatorOffset::default(),
        },
        client: ClientConfig {
            protocol: String::new(),
            transport: None,
            address: "0.0.0.0:4533".to_string(),
        },
        options: Options {
            coordinate_system: "altaz".to_string(),
            timeout_ms: 5000,
            location: LocationConfig::default(),
        },
    }
}

#[cfg(feature = "gui")]
fn should_run_gui(args: &Args) -> bool {
    !args.no_gui && args.rotator_protocol.is_none() && args.client_protocol.is_none()
}

fn main() {
    let args = Args::parse();

    #[cfg(feature = "gui")]
    {
        if should_run_gui(&args) {
            if let Err(e) = rotator_protocol::gui::run_gui() {
                eprintln!("GUI error: {}", e);
                std::process::exit(1);
            }
            return;
        }
    }

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    if let Err(e) = rt.block_on(run_cli(args)) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_cli(args: Args) -> Result<()> {
    setup_logging(args.verbose);

    if args.debug {
        set_debug_enabled(true);
        info!("Debug mode enabled");
    }

    if args.list_protocols {
        print!("{}", list_protocols());
        return Ok(());
    }

    let config = build_config(&args)?;

    info!(
        "Rotator: {} over {}",
        config.rotator.protocol,
        if is_alpaca_protocol(&config.rotator.protocol) {
            "http".to_string()
        } else {
            config
                .rotator
                .transport
                .clone()
                .unwrap_or_else(|| "auto".to_string())
        }
    );
    info!(
        "Client: {} over {} @ {}",
        config.client.protocol,
        config.client.transport.as_deref().unwrap_or("tcp"),
        config.client.address
    );

    let runner = Runner::new(config);
    runner.run().await
}

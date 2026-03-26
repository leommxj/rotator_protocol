# Rotator Protocol Converter

## Project Structure

```
src/
├── lib.rs / main.rs / error.rs
├── model/          # Position, UnifiedCommand, DeviceStatus, AngleConverter, CoordinateConverter
├── transport/      # TCP, Serial, WebSocket, UDP, DebugTransport
├── codec/          # GS-232, Pelco-D, rotctld, Stellarium, EasyComm, LX200, NexStar, INDI, Alpaca
├── endpoint/       # Transport + Codec, AlpacaEndpoint
├── bridge/         # Protocol bridge with coordinate/angle conversion
└── config/         # TOML config loader
```

## Architecture

```
Client Endpoint ←→ Bridge ←→ Rotator Endpoint
      ↓               ↓              ↓
   Codec      AngleConverter      Codec
      ↓       CoordConverter         ↓
  Transport                     Transport
```

## Key Components

- **Transport**: `send()`, `recv()`, `recv_timeout()`
- **Codec**: `encode()`, `decode()`, `capabilities()`
- **UnifiedCommand**: Intermediate representation for all protocols
- **AngleConverter**: Limits validation, offset compensation, angle normalization
- **CoordinateConverter**: Equatorial (RA/Dec) ↔ Horizontal (Az/El) conversion

## CLI Usage

```bash
# Basic: Stellarium -> GS-232B over TCP
./rotator_protocol --cp stellarium --rp gs232b --ra 192.168.1.100:4000

# With serial port
./rotator_protocol --cp rotctld --rp gs232b --ra /dev/ttyUSB0 -b 9600

# With angle limits and offset
./rotator_protocol --cp rotctld --rp gs232b --ra :4000 \
  --az-range 0:360 --el-range 0:90 --az-offset 5.0

# With coordinate conversion (for Stellarium equatorial coords)
./rotator_protocol --cp stellarium --rp gs232b --ra :4000 \
  --lat 40.7128 --lon -74.0060

# List supported protocols
./rotator_protocol --list
```

## Supported Protocols

| Protocol | Type | Description |
|----------|------|-------------|
| gs232a/gs232b | Rotator | Yaesu GS-232 |
| rotctld | Rotator | Hamlib rotctld |
| easycomm1/2/3 | Rotator | EasyComm |
| pelco_d/pelco_p | PTZ | Pelco-D/P |
| lx200 | Telescope | Meade LX200 |
| nexstar | Telescope | Celestron NexStar |
| stellarium | Telescope | Stellarium |
| alpaca | ASCOM | Alpaca REST API |
| indi | Astronomy | INDI XML |

## Build

```bash
make release          # Linux x86_64
make windows          # Windows x86_64
make all              # All platforms
make clean            # Clean build
```

## Code Style

- Minimal comments, only when necessary
- `thiserror` for errors
- `tokio` for async
- `tokio-serial` for serial ports

# Rotator Protocol Converter

Rust 编写的旋转器 / 云台 / 望远镜协议桥接器。

它把不同应用层协议统一映射到一套内部命令模型，再通过不同 transport 把 client 侧请求转发到 rotator 侧设备。

## 项目能力

- 在不同协议之间做桥接，例如 `rotctld -> GS-232`、`Stellarium -> rotctld`
- 在不同 transport 之间做桥接，例如 `TCP -> Serial`、`UDP -> WebSocket`
- 对目标 rotator 做角度限制、偏移修正
- 在提供经纬度时进行赤道坐标 / 地平坐标转换
- 可选 GUI（feature `gui`）

## 支持的协议

- GS-232A / GS-232B
- rotctld / Hamlib
- EasyComm 1 / 2 / 3
- Pelco-D / Pelco-P
- LX200
- NexStar
- Stellarium Telescope Protocol
- INDI
- Alpaca Rotator / Alpaca Telescope

精确的双向实现状态、命令覆盖和限制请看 [Development.md](./Development.md)。

## 支持的 transport

- client/source 侧：`tcp`、`udp`、`websocket`
- rotator/target 侧：`serial`、`tcp`、`udp`、`websocket`
- `http` 仅用于 Alpaca rotator/target

## 使用前注意

- `Alpaca` 只能作为 rotator/target 侧协议使用。
- `Serial` 只支持 rotator/target 侧。
- `UDP` 已实现双边支持，但无法通过地址自动识别，必须显式指定 `transport = "udp"`。

## 配置格式

程序默认会在当前目录查找 `config.toml`；也可以通过 `--config` 指定。

示例：

```toml
[rotator]
protocol = "gs232"
transport = "serial"
address = "/dev/ttyUSB0"
baudrate = 9600

[rotator.limits]
azimuth_min = 0
azimuth_max = 360
elevation_min = 0
elevation_max = 90

[rotator.offset]
azimuth = 0
elevation = 0

[client]
protocol = "rotctld"
transport = "tcp"
address = "0.0.0.0:4533"

[options]
coordinate_system = "altaz"
timeout_ms = 5000

[options.location]
latitude = 31.2304
longitude = 121.4737
```

### 配置项说明

- `rotator.protocol`: 目标设备协议
- `rotator.transport`: 目标设备 transport，支持 `serial` / `tcp` / `udp` / `websocket`，Alpaca 隐式使用 HTTP
- `rotator.address`: 目标设备地址；serial 侧是串口名，network 侧是主机地址
- `rotator.limits`: 目标角度范围，会在 `GotoPosition` 时做校验
- `rotator.offset`: 角度偏移，会在下发前和回传后做转换
- `client.protocol`: incoming client 协议
- `client.transport`: incoming transport，仅支持 `tcp` / `udp` / `websocket`
- `client.address`: listener 地址
- `options.timeout_ms`: source/target 收包超时
- `options.location`: 提供非零经纬度后，桥接器才会启用赤道坐标和地平坐标之间的换算

## CLI 用法

查看支持列表：

```bash
cargo run -- --list-protocols
```

最小示例，监听 `rotctld`，转发到串口 `GS-232`：

```bash
cargo run -- \
  -R gs232 \
  --rotator-transport serial \
  --rotator-address /dev/ttyUSB0 \
  -C rotctld \
  --client-transport tcp \
  --client-address :4533
```

使用配置文件：

```bash
cargo run -- --config config.toml
```

启用调试收发日志：

```bash
cargo run -- --config config.toml --debug -v
```

启用 GUI（可选）：

```bash
cargo run --features gui
```

如果启用了 GUI，但又显式传入 `--rotator-protocol` 和 `--client-protocol`，程序会按 CLI 模式运行。

## 相关文档

- [Development.md](./Development.md): 架构、实现矩阵、开发说明
- [protocols_documentation.md](./protocols_documentation.md): 协议规范参考文档

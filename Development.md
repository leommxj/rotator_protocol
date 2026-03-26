# Development

本文件只放开发者相关内容：架构、实现状态、内部限制、目录结构与扩展方式。

用户使用说明请看 [README.md](./README.md)。

## 架构概览

```text
┌────────────────────────────────────────────────────────────┐
│                      Bridge (桥接层)                        │
│         连接 Source 和 Target，处理协议转换                  │
└────────────────────────────────────────────────────────────┘
        ▲                                       │
        │ UnifiedCommand / DeviceStatus         ▼
┌───────┴───────┐                       ┌───────┴───────┐
│    Source     │                       │    Target     │
│  Endpoint     │                       │   Endpoint    │
└───────┬───────┘                       └───────┬───────┘
        │                                       │
┌───────┴───────┐                       ┌───────┴───────┐
│    Codec      │                       │    Codec      │
│  (协议编解码)  │                       │  (协议编解码)  │
└───────┬───────┘                       └───────┬───────┘
        │                                       │
┌───────┴───────┐                       ┌───────┴───────┐
│   Transport   │                       │   Transport   │
│   (传输层)     │                       │   (传输层)     │
└───────────────┘                       └───────────────┘
```

桥接时的关键步骤：

1. `SourceEndpoint` 把 client 请求解码为统一 `UnifiedCommand`
2. `Bridge` 负责角度限制、偏移修正、必要时做坐标转换
3. `TargetEndpoint` 把统一命令编码成目标设备协议并发出
4. 如果目标协议支持状态读取，再把 `DeviceStatus` 编码回 source 协议

## 实现状态

以下状态针对当前代码，不等同于协议规范本身的完整覆盖。

### Transport 支持矩阵

| Transport | client/source 侧 | rotator/target 侧 | 说明 |
|---|---|---|---|
| TCP | 已实现 | 已实现 | 默认网络 transport |
| UDP | 已实现 | 已实现 | 已接入运行路径，但必须显式指定 `transport = "udp"` |
| WebSocket | 已实现 | 已实现 | client 侧是 listener，rotator 侧是 client |
| Serial | 未实现 | 已实现 | 仅 rotator/target 侧支持 |
| HTTP | 未实现 | 仅 Alpaca | 不是通用 transport，只用于 Alpaca target endpoint |

### 协议实现矩阵

| 协议 | 别名 | client/source 侧 | rotator/target 侧 | 坐标 | 备注 |
|---|---|---|---|---|---|
| GS-232A | `gs232a`, `gs-232a` | 可接收查询、方位 goto、连续移动、停止，并回传方位状态 | 可发送查询、方位 goto、方位移动、停止、速度设置 | AltAz | 目标侧是方位角单轴实现 |
| GS-232B | `gs232`, `gs-232`, `gs232b`, `gs-232b`, `yaesu` | 可接收查询、方位/仰角 goto、连续移动、停止，并回传位置状态 | 可发送查询、方位/仰角 goto、连续移动、停止、速度设置，并解析位置状态 | AltAz | 当前最完整的串口 rotator 协议之一 |
| rotctld | `rotctld`, `rotctl`, `hamlib` | 可接收查询、goto、连续移动、停止、park、reset、info，并回传位置 / `RPRT` | 同左 | AltAz | 当前网络 client/source 路径最完整 |
| EasyComm 1 | `easycomm1` | 可接收 goto、连续移动、轴停、info，并可回传 `AZ ... EL ...` 状态 | 可发送 goto、连续移动、轴停、info | AltAz | 当前没有可靠的端到端位置查询 / 状态解析路径 |
| EasyComm 2/3 | `easycomm`, `easycomm2`, `easycomm3` | 与 EasyComm1 类似，但 move 编码走带速度数值的变体 | 与 EasyComm1 类似 | AltAz | 代码当前对 v2/v3 走同一套 speed move 编码 |
| Pelco-D | `pelco_d`, `pelco-d`, `pelcod` | 可接收 move、stop、preset、get-position 指令 | 可发送 move、stop、preset、get-position 报文 | PTZ / AltAz 风格 | 没有状态回报码编码，也没有位置状态解析，无法形成端到端位置回读 |
| Pelco-P | `pelco_p`, `pelco-p`, `pelcop` | 可接收 move、stop | 可发送 move、stop | PTZ / AltAz 风格 | 无 preset、无状态回读 |
| LX200 | `lx200`, `meade` | 可接收查询、连续移动、停止，并可回传 RA/Dec 状态 | 可发送查询、goto、连续移动、停止、速度设置，并解析 RA/Dec 状态 | 当前实现按 Equatorial 使用 | 当前 decode 不会把 `:Sr` / `:Sd` / `:MS#` 组合还原成 client 侧 goto |
| NexStar | `nexstar`, `celestron` | 可接收查询、goto，并可回传位置状态 | 可发送查询、goto，并解析位置状态 | 当前实现按 Equatorial 使用 | 只实现精确 RA/Dec 路径，没有连续移动 / stop |
| Stellarium | `stellarium` | 可接收 goto，并回传 current-position packet | 可发送 goto | Equatorial | target 侧没有状态解析 / 查询路径 |
| INDI | `indi`, `indilib` | 可接收 `getProperties`、goto、move、stop、park，并可回传 number-vector 状态 | 可发送查询、goto、move、stop、park，并解析状态 | Equatorial + AltAz + rotator angle | 依赖 `EQUATORIAL_EOD_COORD`、`HORIZONTAL_COORD`、`ABS_ROTATOR_ANGLE`、`TELESCOPE_*` 等 property 名称 |
| Alpaca Rotator | `alpaca`, `alpaca-rotator`, `ascom` | 未实现 | 可发送 goto、stop，并通过 HTTP 读当前位置 / moving 状态 | Rotator angle | 仅 target 侧，非 codec |
| Alpaca Telescope | `alpaca-telescope` | 未实现 | 可发送 AltAz / RA-Dec goto、axis move、stop，并通过 HTTP 读状态 | Equatorial + AltAz | 仅 target 侧，当前未实现 park |

### 重要限制

- `Alpaca` 只能作为 rotator/target 侧协议使用，不能作为 incoming client 协议。
- `HTTP` 不是通用 transport，只给 Alpaca target endpoint 使用。
- `Serial` 只支持 rotator/target 侧，不支持 client/source listener。
- `UDP` 已实现双边支持，但无法通过地址自动识别，必须显式指定 `transport = "udp"`。
- `options.coordinate_system` 当前只是配置字段，运行时未直接消费。

## 配置与运行时行为

### Transport 自动识别规则

rotator 侧如果没有显式写 `transport`，当前逻辑会这样判断：

- 地址以 `/dev/`、`COM`、`com`、`\\\\.\\COM` 开头：`serial`
- 地址以 `ws://` 或 `wss://` 开头：`websocket`
- 地址以 `http://` 或 `https://` 开头：`http`
- 其他情况：`tcp`

注意：

- `UDP` 不会自动识别，因为它和 TCP 都是 `host:port` 形式。
- client/source 侧如果被自动识别成 `http`，会在配置校验阶段报错。

### 坐标与转换

统一位置模型支持两种坐标：

- `AltAz`
- `Equatorial`

当前桥接器的转换逻辑是：

- 目标协议如果需要 AltAz，而 source 给的是 RA/Dec，会在提供 `options.location` 后做天文坐标换算
- 目标协议如果需要 RA/Dec，而 rotator 返回的是 AltAz，也会做反向换算
- 如果未提供非零经纬度，则不会启用坐标换算，source/target 协议的坐标系必须自行对齐

## 目录结构

```text
src/
├── bridge/        # Bridge 与命令转换
├── codec/         # 协议编解码
├── config/        # TOML 配置加载
├── endpoint/      # Source/Target/Alpaca endpoint
├── gui/           # 可选 GUI（feature = "gui"）
├── model/         # UnifiedCommand / Position / DeviceStatus / converter
├── transport/     # TCP / UDP / Serial / WebSocket / Debug
├── error.rs       # 统一错误类型
├── lib.rs         # 库入口
├── main.rs        # CLI 入口
└── runner.rs      # 配置校验、transport 选择、运行循环
```

## 数据流示例

场景：`GPredict (rotctld) -> 本程序 -> GS-232`

```text
GPredict                    本程序                         旋转器
   │                          │                              │
   │──"P 180.0 45.0\n"───────▶│                              │
   │                          │ decode -> GotoPosition       │
   │                          │ encode -> "W180 045\r"       │
   │                          │─────────────────────────────▶│
   │                          │                              │
   │                          │◀─────────────────────────────│
   │◀──────位置状态 / RPRT────│                              │
```

## 开发

常用命令：

```bash
cargo check
cargo test
cargo build --release
```

也可以使用 `Makefile`：

```bash
make check
make test
make release
make windows
```

## 扩展方式

### 添加新协议

1. 在 `src/codec/` 下新增协议实现
2. 实现 `Codec` trait
3. 在 `src/codec/mod.rs` 注册名字和别名
4. 明确它在 `client/source` 与 `rotator/target` 两个方向上的真实可用性

### 添加新 transport

1. 在 `src/transport/` 下新增实现
2. 实现 `Transport` trait
3. 如果需要 listener，再实现 `TransportServer`
4. 在 `src/transport/mod.rs` 和 `src/runner.rs` 接入运行路径

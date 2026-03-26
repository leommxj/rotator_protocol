# Rotator/云台/望远镜控制协议文档

本文档整理了常见的旋转器控制协议规范，用于实现Rust协议转换项目的参考。

> 注意：本文首先是协议规范参考，不等同于当前仓库的完整实现范围。项目的实现矩阵、transport 限制和开发说明请以 [Development.md](./Development.md) 为准。

---

## 目录

1. [Pelco-D 协议](#1-pelco-d-协议)
2. [Pelco-P 协议](#2-pelco-p-协议)
3. [Yaesu GS-232A/B 协议](#3-yaesu-gs-232ab-协议)
4. [Meade LX200 协议](#4-meade-lx200-协议)
5. [Celestron NexStar 协议](#5-celestron-nexstar-协议)
6. [EasyComm 协议](#6-easycomm-协议)
7. [Hamlib rotctld 协议](#7-hamlib-rotctld-协议)
8. [Stellarium Telescope Protocol](#8-stellarium-telescope-protocol)
9. [ASCOM Alpaca 协议](#9-ascom-alpaca-协议)
10. [INDI 协议](#10-indi-协议)
11. [INDIGO 协议](#11-indigo-协议)
12. [协议对比总结](#12-协议对比总结)

---

## 1. Pelco-D 协议

### 概述
Pelco-D是安防监控行业广泛使用的PTZ(Pan/Tilt/Zoom)控制协议，用于控制云台摄像机。

### 传输层
- **物理层**: RS-485 (半双工)
- **波特率**: 2400 bps (标准), 可选4800/9600
- **数据格式**: 8数据位, 1停止位, 无校验

### 报文格式 (7字节)

| 字节 | 名称 | 说明 |
|------|------|------|
| 1 | Sync | 同步字节，固定为 `0xFF` |
| 2 | Address | 设备地址 (0x01-0xFF) |
| 3 | Command1 | 命令字节1 |
| 4 | Command2 | 命令字节2 |
| 5 | Data1 | 数据1 (水平速度) |
| 6 | Data2 | 数据2 (垂直速度) |
| 7 | Checksum | 校验和 |

### Command1 位定义

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
|-------|-------|-------|-------|-------|-------|-------|-------|
| Sense | Reserved | Reserved | Auto/Manual Scan | Camera On/Off | Iris Close | Iris Open | Focus Near |

### Command2 位定义

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
|-------|-------|-------|-------|-------|-------|-------|-------|
| Focus Far | Zoom Wide | Zoom Tele | Tilt Down | Tilt Up | Pan Left | Pan Right | 固定0 |

### 速度范围
- **Pan速度 (Data1)**: 0x00 (停止) - 0x3F (高速), 0xFF (Turbo最高速)
- **Tilt速度 (Data2)**: 0x00 (停止) - 0x3F (最高速)

### 校验和计算
```
Checksum = (Address + Command1 + Command2 + Data1 + Data2) mod 256
```

### 命令示例

| 操作 | 报文 (Hex) |
|------|------------|
| 向左转动(高速) | FF 01 00 04 3F 00 44 |
| 向右转动(中速) | FF 01 00 02 20 00 23 |
| 向上抬升(高速) | FF 01 00 08 00 3F 48 |
| 向下俯冲(中速) | FF 01 00 10 20 00 31 |
| 停止所有动作 | FF 01 00 00 00 00 01 |
| 调用预置位07 | FF 01 00 07 00 07 0F |
| 设置预置位07 | FF 01 00 03 00 07 0B |

### 扩展命令

| 功能 | Command1 | Command2 | Data1 | Data2 |
|------|----------|----------|-------|-------|
| 设置预置位 | 0x00 | 0x03 | 0x00 | 预置位号 |
| 调用预置位 | 0x00 | 0x07 | 0x00 | 预置位号 |
| 清除预置位 | 0x00 | 0x05 | 0x00 | 预置位号 |
| 查询位置 | 0x00 | 0x51 | 0x00 | 0x00 |

### 注意事项
- 支持最多255个从设备
- 运动命令有15秒超时保护，需每5秒发送保持命令
- 主从模式，从设备不会主动发送数据

---

## 2. Pelco-P 协议

### 概述
Pelco-P是Pelco-D的前身，格式稍有不同，最多支持32个设备。

### 传输层
- **物理层**: RS-485
- **波特率**: 4800 bps (标准)
- **数据格式**: 8数据位, 1停止位, 无校验

### 报文格式 (8字节)

| 字节 | 名称 | 值 |
|------|------|-----|
| 1 | STX | 起始字节，固定为 `0xA0` |
| 2 | Address | 设备地址 (0x00-0x1F) |
| 3 | Data1 | 命令字节1 |
| 4 | Data2 | 命令字节2 |
| 5 | Data3 | Pan速度 |
| 6 | Data4 | Tilt速度 |
| 7 | ETX | 结束字节，固定为 `0xAF` |
| 8 | Checksum | XOR校验和 |

### 校验和计算
```
Checksum = STX XOR Address XOR Data1 XOR Data2 XOR Data3 XOR Data4 XOR ETX
```

### Data1 位定义

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
|-------|-------|-------|-------|-------|-------|-------|-------|
| - | Camera On | Auto Scan | Camera On/Off | Iris Close | Iris Open | Focus Near | Focus Far |

### Data2 位定义

| Bit 7 | Bit 6 | Bit 5 | Bit 4 | Bit 3 | Bit 2 | Bit 1 | Bit 0 |
|-------|-------|-------|-------|-------|-------|-------|-------|
| - | - | Zoom Wide | Zoom Tele | Tilt Down | Tilt Up | Pan Left | Pan Right |

### 速度范围
- **Pan速度 (Data3)**: 0x00-0x3F (正常), 0x40 (Turbo)
- **Tilt速度 (Data4)**: 0x00-0x3F

### 命令示例

| 操作 | 报文 (Hex) |
|------|------------|
| 向左转动(正常速度) | A0 00 00 04 20 00 AF 2B |
| 向右转动(正常速度) | A0 00 00 02 20 00 AF 2D |
| 向上抬升(正常速度) | A0 00 00 08 00 20 AF 27 |
| 向下俯冲(正常速度) | A0 00 00 10 00 20 AF 3F |
| 停止所有动作 | A0 00 00 00 00 00 AF 0F |

---

## 3. Yaesu GS-232A/B 协议

### 概述
Yaesu GS-232是业余无线电天线旋转器的标准控制接口，广泛用于卫星追踪和无线电通信。

### 传输层
- **物理层**: RS-232 (DB-9)
- **波特率**: 150-9600 bps (可配置，通常4800或9600)
- **数据格式**: 8数据位, 1停止位, 无校验
- **行结束符**: CR (0x0D)

### 命令格式
所有命令为ASCII文本，以回车(CR)结尾。数值参数为3位数字。

### 基本命令列表

| 命令 | 格式 | 说明 |
|------|------|------|
| C | `C` | 查询当前方位角 |
| B | `B` | 查询当前仰角 |
| C2 | `C2` | 查询方位角和仰角 |
| M | `Maaa` | 转到指定方位角 (aaa=000-450) |
| W | `Waaa eee` | 转到指定方位角和仰角 |
| R | `R` | 顺时针转动(右转) |
| L | `L` | 逆时针转动(左转) |
| U | `U` | 仰角上升 |
| D | `D` | 仰角下降 |
| S | `S` | 停止所有转动 |
| A | `A` | 停止方位角转动 |
| E | `E` | 停止仰角转动 |

### 扩展命令

| 命令 | 格式 | 说明 |
|------|------|------|
| P36 | `P36` | 设置360度模式 |
| P45 | `P45` | 设置450度模式 |
| O | `Oaaa` | 设置偏移校准 |
| F | `Faaa` | 设置全刻度校准 |
| H | `H` | 显示帮助信息 |
| H2 | `H2` | 显示仰角命令帮助 |

### 响应格式

| 命令 | 响应格式 | 示例 |
|------|----------|------|
| C | `+0aaa` 或 `AZ=aaa` | `+0180` 或 `AZ=180` |
| B | `+0eee` 或 `EL=eee` | `+0045` 或 `EL=045` |
| C2 | `+0aaa+0eee` 或 `AZ=aaa EL=eee` | `+0180+0045` |

### 命令示例
```
M180<CR>      -- 转到方位角180度
W180 045<CR>  -- 转到方位角180度，仰角45度
C<CR>         -- 查询当前方位角
S<CR>         -- 停止转动
```

### 与Hamlib兼容性
GS-232协议被Hamlib库广泛支持，rotctld可以作为网络代理。

---

## 4. Meade LX200 协议

### 概述
LX200是Meade望远镜的串行控制协议，已成为事实上的天文望远镜控制标准，被许多其他品牌支持。

### 传输层
- **物理层**: RS-232
- **波特率**: 9600 bps
- **数据格式**: 8数据位, 1停止位, 无校验 (8N1)
- **命令结束符**: `#`

### 命令格式
所有命令以冒号 `:` 开头，以 `#` 结尾。

### 坐标格式

| 类型 | 格式 | 范围 |
|------|------|------|
| 赤经 (RA) | HH:MM:SS 或 HH:MM.T | 00:00:00 - 23:59:59 |
| 赤纬 (Dec) | sDD*MM:SS 或 sDD*MM | -90°00 - +90°00 |
| 方位角 (Az) | DDD*MM:SS | 000°00 - 359°59 |
| 高度角 (Alt) | sDD*MM:SS | -90°00 - +90°00 |

### 查询命令

| 命令 | 说明 | 响应 |
|------|------|------|
| `:GR#` | 获取赤经 | HH:MM:SS# |
| `:GD#` | 获取赤纬 | sDD*MM:SS# |
| `:GA#` | 获取高度角 | sDD*MM:SS# |
| `:GZ#` | 获取方位角 | DDD*MM:SS# |
| `:GS#` | 获取恒星时 | HH:MM:SS# |
| `:GC#` | 获取日期 | MM/DD/YY# |
| `:GL#` | 获取本地时间 | HH:MM:SS# |
| `:Gt#` | 获取纬度 | sDD*MM# |
| `:Gg#` | 获取经度 | DDD*MM# |

### 设置命令

| 命令 | 说明 | 响应 |
|------|------|------|
| `:Sr HH:MM:SS#` | 设置目标赤经 | 1=成功, 0=失败 |
| `:Sd sDD*MM:SS#` | 设置目标赤纬 | 1=成功, 0=失败 |
| `:Sa sDD*MM#` | 设置目标高度 | 1=成功 |
| `:Sz DDD*MM#` | 设置目标方位 | 1=成功 |
| `:SL HH:MM:SS#` | 设置本地时间 | 1=成功 |
| `:SC MM/DD/YY#` | 设置日期 | 1=成功 |

### 运动命令

| 命令 | 说明 |
|------|------|
| `:MS#` | GOTO到目标坐标 (返回0=成功, 1=目标在地平线下, 2=低于限位) |
| `:Mn#` | 向北移动 |
| `:Ms#` | 向南移动 |
| `:Me#` | 向东移动 |
| `:Mw#` | 向西移动 |
| `:Qn#` | 停止向北 |
| `:Qs#` | 停止向南 |
| `:Qe#` | 停止向东 |
| `:Qw#` | 停止向西 |
| `:Q#` | 停止所有运动 |

### 速度控制

| 命令 | 说明 |
|------|------|
| `:RG#` | 导星速度 (最慢) |
| `:RC#` | 定心速度 |
| `:RM#` | 寻找速度 |
| `:RS#` | 高速移动 (最快) |

### 同步命令

| 命令 | 说明 |
|------|------|
| `:CM#` | 同步到当前目标坐标 |

### 对象选择

| 命令 | 说明 |
|------|------|
| `:LM NNNN#` | 选择梅西耶天体 |
| `:LC NNNN#` | 选择NGC天体 |
| `:LS NNNN#` | 选择恒星 (901-909为行星) |

---

## 5. Celestron NexStar 协议

### 概述
NexStar协议用于Celestron望远镜和SkyWatcher/Orion SynScan赤道仪。

### 传输层
- **物理层**: RS-232
- **波特率**: 9600 bps
- **数据格式**: 8数据位, 1停止位, 无校验

### 坐标编码
位置以16进制表示，值为旋转角度的分数 (值/65536 或 值/0x100000000)。

### 基本命令

| 命令 | 格式 | 说明 | 响应 |
|------|------|------|------|
| Get RA/Dec | `E` | 获取赤道坐标 | AAAA,DDDD# |
| Get Precise RA/Dec | `e` | 获取精确坐标 | AAAAAAAA,DDDDDDDD# |
| Get Az/Alt | `Z` | 获取地平坐标 | AAAA,EEEE# |
| Get Precise Az/Alt | `z` | 获取精确地平坐标 | AAAAAAAA,EEEEEEEE# |
| Goto RA/Dec | `R` | GOTO赤道坐标 | # |
| Goto Precise RA/Dec | `r` | 精确GOTO | # |
| Goto Az/Alt | `B` | GOTO地平坐标 | # |
| Goto Precise Az/Alt | `b` | 精确GOTO | # |

### 命令示例
```
E           -> 34AB,12CE#    (获取RA/Dec)
R34AB,12CE  -> #             (GOTO到RA=34AB, Dec=12CE)
Z           -> 12AB,4000#    (获取Az/Alt)
B12AB,4000  -> #             (GOTO到Az=12AB, Alt=4000)
```

### AUX低级协议
NexStar还有一个底层AUX协议，用于直接与电机控制器通信：

| 字段 | 说明 |
|------|------|
| 0x3B | 包起始标志 |
| Length | 包长度 |
| Source | 源设备ID |
| Destination | 目标设备ID |
| Command | 命令字节 |
| Data | 数据字节 |
| Checksum | 校验和 |

---

## 6. EasyComm 协议

### 概述
EasyComm是卫星追踪软件和天线旋转器之间的标准协议，有三个版本。

### 传输层
- **物理层**: RS-232 或 TCP
- **波特率**: 9600-19200 bps
- **数据格式**: 8数据位, 1停止位, 无校验
- **命令分隔符**: 空格、CR 或 LF

### EasyComm I

完整格式:
```
AZaaa.a ELeee.e UPuuuuuuuuu UUU DNddddddddd DDD
```

| 字段 | 说明 | 范围 |
|------|------|------|
| AZ | 方位角 (度) | 0.0-360.0 |
| EL | 仰角 (度) | 0.0-180.0 |
| UP | 上行频率 (Hz) | - |
| DN | 下行频率 (Hz) | - |
| UUU/DDD | 上行/下行模式 | FM, SSB等 |

### EasyComm II

| 命令 | 说明 |
|------|------|
| AZaaa.a | 设置目标方位角 |
| ELeee.e | 设置目标仰角 |
| UP | 上行频率 |
| DN | 下行频率 |
| UM | 上行模式 |
| DM | 下行模式 |
| ML | 向左移动 |
| MR | 向右移动 |
| MU | 向上移动 |
| MD | 向下移动 |
| SA | 停止方位角移动 |
| SE | 停止仰角移动 |
| AO | AOS (卫星可见) |
| LO | LOS (卫星消失) |
| IP | 读取输入端口 |
| OP | 设置输出端口 |
| AN | 读取模拟输入 |
| ST | 设置时间 |
| VE | 获取版本 |

### EasyComm III

扩展了速度控制和配置寄存器:

| 命令 | 说明 |
|------|------|
| VL/VR/VU/VD | 速度控制 (mdeg/s) |
| CR | 读取配置寄存器 |
| CW | 写入配置寄存器 |
| GS | 获取状态寄存器 |
| GE | 获取错误寄存器 |

配置寄存器包括: MaxSpeed, Overshoot, Jamming, Endpoints, Unstick

状态位: Idle, Moving, Pointing, Error

### 示例
```
AZ180.5      -- 设置方位角180.5度
EL45.0       -- 设置仰角45度
AZ180.5 EL45.0  -- 同时设置
VE           -- 获取版本
```

---

## 7. Hamlib rotctld 协议

### 概述
rotctld是Hamlib项目提供的TCP旋转器控制守护进程。它将各种旋转器的控制抽象为统一的网络接口，是业余无线电和卫星追踪领域重要的协议网关。应用程序可以使用rotator model 2 ("NET rotctl") 通过网络连接到rotctld。

### 传输层
- **协议**: TCP/IP
- **默认端口**: 4533
- **数据格式**: ASCII文本
- **行结束符**: `\n` (LF)

### 协议模式

rotctld支持两种协议模式：

#### 默认协议 (Default Protocol)
主要用于Hamlib库函数与rotctld之间的通信。命令简洁，每个命令一行。

#### 扩展响应协议 (Extended Response Protocol)
用于脚本和程序直接与rotctld交互，提供更详细的反馈。通过在命令前加标点符号激活：
- `+` - 使用换行符分隔响应
- 其他标点 (`;`, `|`, `,` 等) - 使用该字符作为分隔符

### 命令格式

命令可以使用单字符或长命令名。大写字母用于设置命令，小写字母用于查询命令。长命令名需要前置反斜杠 `\`。

### 核心命令列表

#### 位置控制

| 短命令 | 长命令 | 参数 | 说明 |
|--------|--------|------|------|
| `P` | `set_pos` | Azimuth Elevation | 设置位置 (浮点数) |
| `p` | `get_pos` | - | 获取当前位置 |
| `M` | `move` | Direction Speed | 连续移动 |
| `S` | `stop` | - | 停止旋转器 |
| `K` | `park` | - | 归位 |
| `R` | `reset` | Reset | 重置 (1=全部重置) |

#### 配置与信息

| 短命令 | 长命令 | 参数 | 说明 |
|--------|--------|------|------|
| `C` | `set_conf` | Token Value | 设置配置参数 |
| `_` | `get_info` | - | 获取设备信息 |
| `1` | `dump_caps` | - | 导出设备能力 |
| - | `dump_state` | - | 导出状态信息 |
| `w` | `send_cmd` | Cmd | 发送原始命令 |

#### 定位器辅助命令

| 短命令 | 长命令 | 说明 |
|--------|--------|------|
| `L` | `lonlat2loc` | 经纬度转Maidenhead网格 |
| `l` | `loc2lonlat` | Maidenhead网格转经纬度 |
| `D` | `dms2dec` | 度分秒转十进制 |
| `d` | `dec2dms` | 十进制转度分秒 |
| `B` | `qrb` | 计算距离和方位角 |

### Move命令方向值

`move` 命令的Direction参数支持整数值或关键字：

| 值 | 关键字 | 说明 |
|----|--------|------|
| 2 | UP | 向上 |
| 4 | DOWN | 向下 |
| 8 | LEFT, CCW | 向左/逆时针 |
| 16 | RIGHT, CW | 向右/顺时针 |
| 32 | UP_LEFT, UP_CCW | 左上 |
| 64 | UP_RIGHT, UP_CW | 右上 |
| 128 | DOWN_LEFT, DOWN_CCW | 左下 |
| 256 | DOWN_RIGHT, DOWN_CW | 右下 |

Speed参数范围: 1-100 (不是所有后端都使用此值)

### 位置值范围
- **方位角 (Azimuth)**: -180.0 到 540.0 度 (取决于旋转器类型)
- **仰角 (Elevation)**: -20.0 到 210.0 度 (取决于旋转器类型)
- 如果旋转器不支持仰角，使用 "0.0"

### 响应格式

#### 默认协议响应

**设置命令响应:**
```
RPRT x\n
```
其中 x 为Hamlib错误码，0表示成功。

**查询命令响应:**
每个值单独一行：
```
<value1>\n
<value2>\n
```

#### 扩展协议响应

使用 `+` 前缀时：
```
<long_command_name>: <value>\n
Azimuth: 180.000000\n
Elevation: 45.000000\n
RPRT 0\n
```

使用其他分隔符 (如 `;`) 时：
```
get_pos:;Azimuth: 180.000000;Elevation: 45.000000;RPRT 0
```

### 命令示例

**默认协议:**
```bash
# 设置位置到 Az=135.0, El=30.0
echo "P 135.0 30.0" | nc -w 1 localhost 4533
# 响应: RPRT 0

# 获取当前位置
echo "p" | nc -w 1 localhost 4533
# 响应:
# 135.000000
# 30.000000

# 停止旋转器
echo "S" | nc -w 1 localhost 4533

# 向右移动，速度50
echo "M 16 50" | nc -w 1 localhost 4533
# 或使用关键字
echo "M RIGHT 50" | nc -w 1 localhost 4533

# 归位
echo "K" | nc -w 1 localhost 4533
```

**扩展协议:**
```bash
# 使用长命令名和扩展响应
echo "+\get_pos" | nc -w 1 localhost 4533
# 响应:
# get_pos:
# Azimuth: 135.000000
# Elevation: 30.000000
# RPRT 0

# 使用分号分隔的响应
echo ";\get_pos" | nc -w 1 localhost 4533
# 响应: get_pos:;Azimuth: 135.000000;Elevation: 30.000000;RPRT 0
```

### rotctld启动参数

| 参数 | 说明 |
|------|------|
| `-m, --model=id` | 旋转器型号ID |
| `-r, --rot-file=device` | 设备端口路径 |
| `-s, --serial-speed=baud` | 串口波特率 |
| `-T, --listen-addr=IPADDR` | 监听IP地址 |
| `-t, --port=number` | TCP端口 (默认4533) |
| `-C, --set-conf=parm=val` | 设置配置参数 |
| `-l, --list` | 列出支持的旋转器型号 |
| `-v, --verbose` | 详细输出 |

### 使用场景

1. **协议网关**: 将串口旋转器(如GS-232)转换为网络可访问
2. **软件集成**: SatNOGS、GPredict等软件通过rotctld控制旋转器
3. **多客户端**: 多个程序可同时连接到同一个rotctld实例
4. **远程控制**: 通过网络远程控制物理旋转器

### 与其他协议的关系
- rotctld后端支持多种物理协议: GS-232, EasyComm I/II/III, SPID等
- "NET rotctl" (model 2) 允许应用程序通过网络使用Hamlib API
- 作为中间层，rotctld协议非常适合作为协议转换网关的输入源

---

## 8. Stellarium Telescope Protocol

### 概述
Stellarium的内置望远镜控制协议，用于Stellarium软件与望远镜服务器之间的通信。

### 传输层
- **物理层**: TCP/IP
- **端口**: 通常10001 (可配置)
- **字节序**: 小端 (Little Endian)

### 消息结构
所有消息都以2字节长度和2字节类型开头。

### MessageCurrentPosition (服务器→客户端, type=0)

用于报告望远镜当前位置:

| 字段 | 大小 | 类型 | 说明 |
|------|------|------|------|
| LENGTH | 2字节 | uint16 | 消息总长度 (=24) |
| TYPE | 2字节 | uint16 | 消息类型 (=0) |
| TIME | 8字节 | int64 | 微秒时间戳 (从1970-01-01 UT) |
| RA | 4字节 | uint32 | 赤经 (0x80000000 = 12h) |
| DEC | 4字节 | int32 | 赤纬 (0x40000000 = 90°) |
| STATUS | 4字节 | int32 | 状态 (0=OK, <0=错误) |

### MessageGoto (客户端→服务器, type=0)

用于发送GOTO命令:

| 字段 | 大小 | 类型 | 说明 |
|------|------|------|------|
| LENGTH | 2字节 | uint16 | 消息总长度 (=20) |
| TYPE | 2字节 | uint16 | 消息类型 (=0) |
| TIME | 8字节 | int64 | 微秒时间戳 |
| RA | 4字节 | uint32 | 目标赤经 |
| DEC | 4字节 | int32 | 目标赤纬 |

### 坐标编码

**赤经 (RA)**:
- 无符号32位整数
- 0x00000000 = 0h
- 0x80000000 = 12h
- 0x100000000 (溢出到0) = 24h = 0h
- 公式: RA_hours = value * 24.0 / 0x100000000

**赤纬 (Dec)**:
- 有符号32位整数
- 0x00000000 = 0°
- 0x40000000 = +90°
- -0x40000000 = -90°
- 公式: Dec_degrees = value * 90.0 / 0x40000000

### 示例代码 (伪代码)
```rust
// 解析RA
fn parse_ra(value: u32) -> f64 {
    (value as f64) * 24.0 / (0x100000000u64 as f64)
}

// 解析Dec
fn parse_dec(value: i32) -> f64 {
    (value as f64) * 90.0 / (0x40000000 as f64)
}

// 编码RA
fn encode_ra(hours: f64) -> u32 {
    ((hours / 24.0) * (0x100000000u64 as f64)) as u32
}

// 编码Dec
fn encode_dec(degrees: f64) -> i32 {
    ((degrees / 90.0) * (0x40000000 as f64)) as i32
}
```

---

## 9. ASCOM Alpaca 协议

### 概述
ASCOM Alpaca是ASCOM的网络版本，通过HTTP/REST API提供设备控制，可跨平台使用。

### 传输层
- **协议**: HTTP/HTTPS
- **数据格式**: JSON
- **端口**: 默认11111

### API结构

基础URL格式:
```
{protocol}://{address}:{port}/api/v1/{device_type}/{device_number}/{endpoint}
```

设备类型包括: telescope, camera, rotator, focuser, dome, filterwheel 等

### Rotator 设备端点

#### 属性 (GET)

| 端点 | 返回类型 | 说明 |
|------|----------|------|
| /canreverse | bool | 是否支持反向 |
| /connected | bool | 连接状态 |
| /connecting | bool | 是否正在连接 |
| /ismoving | bool | 是否正在移动 |
| /mechanicalposition | float | 机械角度 (度) |
| /position | float | 当前位置 (带同步偏移) |
| /reverse | bool | 是否反向 |
| /stepsize | float | 最小步进角度 (度) |
| /targetposition | float | 目标位置 |

#### 方法 (PUT)

| 端点 | 参数 | 说明 |
|------|------|------|
| /halt | - | 立即停止 |
| /move | Position (度) | 相对移动 |
| /moveabsolute | Position (度) | 绝对移动 |
| /movemechanical | Position (度) | 移动到机械位置 |
| /sync | Position (度) | 同步位置 |
| /connect | - | 连接设备 |
| /disconnect | - | 断开设备 |

### Telescope 设备端点

#### 位置属性

| 端点 | 返回类型 | 说明 |
|------|----------|------|
| /rightascension | float | 赤经 (小时) |
| /declination | float | 赤纬 (度) |
| /altitude | float | 高度角 (度) |
| /azimuth | float | 方位角 (度) |
| /siderealtime | float | 恒星时 |

#### 移动方法

| 端点 | 参数 | 说明 |
|------|------|------|
| /slewtocoordinates | RA, Dec | GOTO到赤道坐标 |
| /slewtocoordinatesasync | RA, Dec | 异步GOTO |
| /slewtoaltaz | Altitude, Azimuth | GOTO到地平坐标 |
| /slewtoaltazasync | Altitude, Azimuth | 异步GOTO |
| /synctocoordinates | RA, Dec | 同步坐标 |
| /abortslew | - | 中止移动 |
| /moveaxis | Axis, Rate | 轴移动 |

### 请求/响应示例

**获取位置:**
```http
GET /api/v1/rotator/0/position HTTP/1.1
Host: 192.168.1.100:11111

Response:
{
  "Value": 45.5,
  "ClientTransactionID": 1,
  "ServerTransactionID": 1,
  "ErrorNumber": 0,
  "ErrorMessage": ""
}
```

**移动到指定位置:**
```http
PUT /api/v1/rotator/0/moveabsolute HTTP/1.1
Host: 192.168.1.100:11111
Content-Type: application/x-www-form-urlencoded

Position=90.0&ClientID=1&ClientTransactionID=2

Response:
{
  "ClientTransactionID": 2,
  "ServerTransactionID": 2,
  "ErrorNumber": 0,
  "ErrorMessage": ""
}
```

### 设备发现
Alpaca支持UDP广播发现协议 (端口32227)。

---

## 10. INDI 协议

### 概述
INDI (Instrument Neutral Distributed Interface) 是开源的天文设备控制协议，基于XML，广泛用于Linux天文软件。

### 传输层
- **协议**: TCP/IP
- **端口**: 7624 (IANA注册)
- **数据格式**: XML文本流

### 架构
- **Server**: 运行设备驱动
- **Client**: 控制软件 (如KStars, EKOS)
- 支持多客户端连接

### 属性类型

| 类型 | 说明 | 示例 |
|------|------|------|
| Text | 文本字符串 | 设备名称 |
| Number | 数值 | 坐标、温度 |
| Switch | 布尔开关 | 连接、跟踪开关 |
| Light | 状态指示灯 | Idle, OK, Busy, Alert |
| BLOB | 二进制大对象 | 图像数据 |

### XML消息结构

#### 定义属性 (def*)

```xml
<defNumberVector device="Mount" name="EQUATORIAL_EOD_COORD"
    label="Eq. Coordinates" group="Main Control"
    state="Idle" perm="rw" timeout="60">
  <defNumber name="RA" label="RA (hh:mm:ss)"
      format="%010.6m" min="0" max="24" step="0">
    0
  </defNumber>
  <defNumber name="DEC" label="Dec (dd:mm:ss)"
      format="%010.6m" min="-90" max="90" step="0">
    0
  </defNumber>
</defNumberVector>

<defSwitchVector device="Mount" name="CONNECTION"
    label="Connection" group="Main Control"
    state="Idle" perm="rw" rule="OneOfMany">
  <defSwitch name="CONNECT" label="Connect">
    Off
  </defSwitch>
  <defSwitch name="DISCONNECT" label="Disconnect">
    On
  </defSwitch>
</defSwitchVector>
```

#### 设置属性 (set*)

服务器发送状态更新:
```xml
<setNumberVector device="Mount" name="EQUATORIAL_EOD_COORD"
    state="Ok" timeout="60" timestamp="2024-01-01T12:00:00">
  <oneNumber name="RA">12.5</oneNumber>
  <oneNumber name="DEC">45.0</oneNumber>
</setNumberVector>
```

#### 新值请求 (new*)

客户端发送命令:
```xml
<newSwitchVector device="Mount" name="CONNECTION">
  <oneSwitch name="CONNECT">On</oneSwitch>
</newSwitchVector>

<newNumberVector device="Mount" name="EQUATORIAL_EOD_COORD">
  <oneNumber name="RA">6.75</oneNumber>
  <oneNumber name="DEC">-16.72</oneNumber>
</newNumberVector>
```

### 标准望远镜属性

| 属性名 | 类型 | 说明 |
|--------|------|------|
| CONNECTION | Switch | 连接开关 |
| EQUATORIAL_EOD_COORD | Number | 赤道坐标 (RA, DEC) |
| TARGET_EOD_COORD | Number | 目标坐标 |
| HORIZONTAL_COORD | Number | 地平坐标 (ALT, AZ) |
| TELESCOPE_ABORT_MOTION | Switch | 中止移动 |
| TELESCOPE_MOTION_NS | Switch | 南北移动 |
| TELESCOPE_MOTION_WE | Switch | 东西移动 |
| TELESCOPE_SLEW_RATE | Switch | 移动速度 |
| TELESCOPE_PARK | Switch | 归位 |
| TELESCOPE_TRACK_MODE | Switch | 跟踪模式 |

### 标准旋转器属性

| 属性名 | 类型 | 说明 |
|--------|------|------|
| ABS_ROTATOR_ANGLE | Number | 绝对角度 |
| ROTATOR_REVERSE | Switch | 反向开关 |

### 消息流程示例

```
Client                          Server
  |                               |
  |--- getProperties ------------>|  请求属性列表
  |                               |
  |<-- defSwitchVector -----------|  定义CONNECTION
  |<-- defNumberVector -----------|  定义EQUATORIAL_EOD_COORD
  |                               |
  |--- newSwitchVector ---------->|  请求连接
  |                               |
  |<-- setSwitchVector -----------|  连接成功
  |<-- setNumberVector -----------|  更新坐标
  |                               |
  |--- newNumberVector ---------->|  GOTO命令
  |                               |
  |<-- setNumberVector (Busy) ----|  开始移动
  |<-- setNumberVector (Ok) ------|  到达目标
```

---

## 11. INDIGO 协议

### 概述
INDIGO是INDI的下一代协议，向后兼容INDI 1.7，但改进了架构和性能。

### 与INDI的主要区别

| 特性 | INDI | INDIGO |
|------|------|--------|
| 架构 | 进程间通信 | 软件总线 |
| 性能 | 标准 | 最高1000x提升 |
| 平台 | 主要Linux | 跨平台 (含macOS) |
| 许可 | GPL | MIT (商业友好) |
| BLOB传输 | 标准 | 优化 |
| 协议版本 | 1.7 | 2.0+ |

### 协议扩展

INDIGO在INDI基础上增加了:
- Number属性增加 `target` 属性区分当前值和目标值
- 更一致的属性命名
- 状态映射 ("Idle" 映射为 "Ok")

### 向后兼容
INDIGO可以使用INDI 1.7协议与传统驱动通信。

---

## 12. 协议对比总结

### 按应用领域分类

| 领域 | 协议 | 特点 |
|------|------|------|
| 安防监控 | Pelco-D, Pelco-P | 简单二进制，RS-485 |
| 业余无线电 | GS-232, EasyComm, rotctld | ASCII命令，卫星追踪 |
| 天文望远镜 | LX200, NexStar | ASCII命令，专业功能 |
| 软件平台 | INDI, ASCOM/Alpaca | 完整生态系统 |
| 通用集成 | Stellarium Protocol | 简单二进制，TCP |
| 协议网关 | rotctld | 网络化，多后端支持 |

### 按传输层分类

| 传输方式 | 协议 |
|----------|------|
| RS-232/RS-485串口 | Pelco-D/P, GS-232, LX200, NexStar, EasyComm |
| TCP/IP | Stellarium, INDI, rotctld, EasyComm (可选) |
| HTTP REST | ASCOM Alpaca |

### 按数据格式分类

| 格式 | 协议 |
|------|------|
| 二进制固定格式 | Pelco-D/P, Stellarium |
| ASCII文本 | GS-232, LX200, NexStar, EasyComm, rotctld |
| XML | INDI, INDIGO |
| JSON | ASCOM Alpaca |

### 功能对比

| 功能 | Pelco-D | GS-232 | LX200 | rotctld | INDI | Alpaca |
|------|---------|--------|-------|---------|------|--------|
| 连续移动 | Yes | Yes | Yes | Yes | Yes | Yes |
| GOTO | 预置位 | Yes | Yes | Yes | Yes | Yes |
| 坐标查询 | 扩展 | Yes | Yes | Yes | Yes | Yes |
| 多设备 | Yes | 单设备 | 单设备 | 单设备 | Yes | Yes |
| 跟踪 | No | No | Yes | No | Yes | Yes |
| 多客户端 | No | No | No | Yes | Yes | Yes |
| 设备发现 | No | No | No | No | No | Yes |
| 网络原生 | No | No | No | Yes | Yes | Yes |

---

## 参考资料

### Pelco
- [Pelco-D Protocol Tutorial](https://www.commfront.com/pages/pelco-d-protocol-tutorial)
- [Pelco-P Protocol Tutorial](https://www.commfront.com/pages/pelco-p-protocol-tutorial)
- [PELCO-D Protocol Command List](https://www.epiphan.com/userguides/LUMiO12x/Content/UserGuides/PTZ/3-operation/PELCODcommands.htm)

### GS-232
- [GS-232B Command Reference](https://www.jzgelectronics.com/gs-232b-command-reference/)
- [Yaesu GS-232B Manual](https://www.manualslib.com/manual/1000473/Yaesu-Gs-232b.html)

### LX200
- [LX200 Command Set](https://skymtn.com/mapug-astronomy/ragreiner/LX200Commands.html)
- [Meade LX200 Protocol](http://www.company7.com/library/meade/LX200CommandSet.pdf)

### NexStar
- [NexStar Communication Protocol](https://s3.amazonaws.com/celestron-site-support-files/support_files/1154108406_nexstarcommprot.pdf)
- [libnexstar Library](https://github.com/indigo-astronomy/libnexstar)

### EasyComm
- [EasyComm Protocol Specification](https://github.com/airween/hamlib_old/blob/master/easycomm/easycomm.txt)
- [Hamlib EasyComm Backend](https://github.com/yapiolibs/hamlib-rotctl-easycomm-parser)

### Hamlib rotctld
- [rotctld Man Page](https://hamlib.sourceforge.net/html/rotctld.1.html)
- [rotctl Man Page](https://hamlib.sourceforge.net/html/rotctl.1.html)
- [Hamlib Project](https://github.com/Hamlib/Hamlib)
- [Ubuntu rotctld Manpage](https://manpages.ubuntu.com/manpages/jammy/en/man1/rotctld.1.html)

### Stellarium
- [Stellarium Telescope Protocol](https://free-astro.org/images/b/b7/Stellarium_telescope_protocol.txt)
- [Stellarium Telescope Control Plugin](https://stellarium.org/doc/23.0/group__telescopeControl.html)

### ASCOM/Alpaca
- [ASCOM Standards](https://ascom-standards.org/)
- [ASCOM Alpaca Documentation](https://ascom-standards.org/newdocs/)
- [ascom-alpaca-rs (Rust)](https://github.com/RReverser/ascom-alpaca-rs)

### INDI/INDIGO
- [INDI Protocol Documentation](http://docs.indilib.org/protocol/)
- [INDI Protocol Specification](https://www.clearskyinstitute.com/INDI/INDI.pdf)
- [INDIGO Project](https://github.com/indigo-astronomy/indigo)
- [INDIGO Protocols](https://github.com/indigo-astronomy/indigo/blob/master/indigo_docs/PROTOCOLS.md)

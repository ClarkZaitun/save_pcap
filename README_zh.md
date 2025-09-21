# save_pcap

[![English README](https://img.shields.io/badge/English-README-blue.svg)](README.md)

一个用Rust编写的跨平台库，用于捕获指定网卡的网络帧并保存为pcap或pcapng文件，支持Linux和Windows系统。

## 功能特性

- 跨平台支持（Linux和Windows）
- 捕获指定网卡的网络帧
- 支持保存为pcap或pcapng格式
- 可自定义文件名前缀和文件路径
- 可设置数据包捕获限制
- 提供简单易用的API
- 支持日志输出
- 自动检测可用网络设备

## 安装

将以下内容添加到你的Cargo.toml文件中：

```toml
dependencies =
{
    save_pcap = "0.1.0"
}
```

## 使用示例

### 基本用法

```rust
use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};

fn main() {
    // 创建捕获选项
    let options = PcapCaptureOptions {
        device_name: "eth0".to_string(), // Linux网卡示例，Windows下可能类似"\\Device\\NPF_{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"
        file_prefix: "my_capture".to_string(),
        file_path: "./captures".to_string(),
        file_format: FileFormat::Pcap,
        packet_limit: Some(1000), // 可选，限制捕获1000个数据包
        snaplen: 65535, // 默认捕获长度
        timeout_ms: 1000, // 默认超时时间
    };
    
    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);
    
    match capturer.capture() {
        Ok(_) => println!("捕获完成成功"),
        Err(e) => eprintln!("错误: {}", e),
    }
}
```

### 使用命令行参数和配置文件

本库提供了一个增强版示例程序`configurable_capture`，支持通过命令行参数或配置文件来设置捕获选项。

#### 通过命令行参数配置

```bash
# 基本使用
cargo run --example configurable_capture -- --device-name "your_network_device" --file-prefix "my_capture"

# 完整参数
cargo run --example configurable_capture -- --device-name "your_network_device" --file-prefix "my_capture" --file-path "./captures" --file-format "pcap" --packet-limit 100 --snaplen 65535 --timeout-ms 1000
```

命令行参数说明：
- `-d, --device-name`：网络设备名称（必需，你可以通过`get_available_devices()`函数获取可用的网卡名称）
- `-p, --file-prefix`：输出文件前缀
- `-o, --file-path`：输出文件路径（默认：./）
- `-f, --file-format`：输出文件格式（pcap 或 pcapng，默认：pcap）
- `-l, --packet-limit`：捕获的数据包数量限制
- `-s, --snaplen`：捕获的数据包大小限制（默认：65535）
- `-t, --timeout-ms`：捕获超时时间(毫秒，默认：1000)
- `-c, --config-file`：配置文件路径

#### 通过配置文件配置

1. 创建配置文件 `config.json`:

```json
{
  "device_name": "your_network_device",
  "file_prefix": "capture",
  "file_path": "./captures/",
  "file_format": "pcap",
  "packet_limit": 100,
  "snaplen": 65535,
  "timeout_ms": 1000
}
```

2. 使用配置文件运行程序:

```bash
cargo run --example configurable_capture -- --config-file config.json
```

3. 混合使用配置文件和命令行参数(命令行参数优先级更高):

```bash
cargo run --example configurable_capture -- --config-file config.json --file-prefix "special_capture" --packet-limit 200
```

项目根目录下的 `examples/config_example.json` 文件提供了完整的配置示例。

### 示例程序

#### 交互式示例 (capture_example.rs)

这个示例提供了一个交互式界面，引导你选择网卡、设置文件名前缀、文件路径、文件格式和数据包限制。

```bash
cargo run --example capture_example
```

#### 可配置示例 (configurable_capture.rs)

这个示例支持通过命令行参数或配置文件来设置捕获选项，提供了更灵活的使用方式。

详细用法请参考上面的"使用命令行参数和配置文件"部分。

#### 测试运行示例 (test_run.rs)

这个示例提供了一个简单的测试运行方式，演示了库的基本功能和用法。

```bash
cargo run --example test_run
```

## 获取可用网卡

```rust
use save_pcap::get_available_devices;

fn main() {
    match get_available_devices() {
        Ok(devices) => {
            println!("可用的网络设备:");
            for device in devices {
                println!("- {}", device);
            }
        },
        Err(e) => eprintln!("错误: {}", e),
    }
}

// 在Linux上可能的输出示例：
// 可用的网络设备:
// - eth0
// - wlan0
// - lo

// 在Windows上可能的输出示例：
// 可用的网络设备:
// - \\Device\\NPF_{12345678-1234-1234-1234-1234567890AB}
// - \\Device\\NPF_{87654321-4321-4321-4321-BA0987654321}
```

## API参考

### PcapCaptureOptions

配置捕获参数的结构体。

```rust
pub struct PcapCaptureOptions {
    pub device_name: String,     // 网卡名称
    pub file_prefix: String,     // 文件名前缀
    pub file_path: String,       // 文件保存路径
    pub file_format: FileFormat, // 文件格式（Pcap或PcapNg）
    pub packet_limit: Option<usize>, // 数据包限制（可选）
    pub snaplen: i32,            // 捕获长度
    pub timeout_ms: i32,         // 超时时间（毫秒）
}
```

提供了默认实现：

```rust
let options = PcapCaptureOptions::default();
// 默认值：
// device_name: "",
// file_prefix: "capture",
// file_path: ".",
// file_format: FileFormat::Pcap,
// packet_limit: None,
// snaplen: 65535,
// timeout_ms: 1000,
```

### FileFormat

枚举类型，表示保存的文件格式。

```rust
#[derive(Debug)]
pub enum FileFormat {
    Pcap,   // pcap格式
    PcapNg, // pcapng格式
}
```

注意：目前两种格式都会以pcap格式保存数据。

### PcapCapturer

捕获器结构体，用于执行捕获操作。

```rust
// 创建捕获器
pub fn new(options: PcapCaptureOptions) -> Self

// 开始捕获并保存到文件
pub fn capture(&self) -> Result<(), SavePcapError>
```

### 错误处理

库定义了`SavePcapError`枚举类型，用于处理各种可能的错误情况：

```rust
#[derive(Error, Debug)]
pub enum SavePcapError {
    #[error("Pcap错误: {0}")]
    PcapError(#[from] PcapError),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("无效的设备名称: {0}")]
    InvalidDevice(String),

    #[error("目录创建失败: {0}")]
    DirectoryCreationFailed(String),

    #[error("捕获被中断")]
    CaptureInterrupted,

    #[error("Pcap文件错误: {0}")]
    PcapFileError(String),
}
```

## 注意事项

1. 在Windows系统上，可能需要安装WinPcap或Npcap驱动程序才能正常使用此库。
2. 在Linux系统上，可能需要安装libpcap-dev包。
3. 捕获网络帧通常需要管理员/root权限。
4. 长时间捕获大量数据包可能会消耗较多的系统资源和磁盘空间。
5. 不同操作系统的网卡命名规则不同：
   - Linux系统：通常为eth0、wlan0、lo等
   - Windows系统：通常为形如"\\Device\\NPF_{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"的GUID格式
6. 请确保指定的网卡名称存在，并且你有足够的权限访问它。
7. 建议使用`get_available_devices()`函数获取当前系统可用的网卡列表。

## 依赖

- [pcap](https://crates.io/crates/pcap) - 用于捕获网络数据包
- [pcap-file](https://crates.io/crates/pcap-file) - 用于写入pcap/pcapng文件
- [anyhow](https://crates.io/crates/anyhow) - 用于错误处理
- [thiserror](https://crates.io/crates/thiserror) - 用于定义自定义错误类型
- [log](https://crates.io/crates/log) 和 [env_logger](https://crates.io/crates/env_logger) - 用于日志输出
- [chrono](https://crates.io/crates/chrono) - 用于处理时间戳

## 许可证

MIT
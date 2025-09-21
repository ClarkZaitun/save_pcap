use anyhow::{Context, Result};
use clap::Parser;
use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};
use serde::{Deserialize, Serialize};
use std::fs;

// 配置文件结构
#[derive(Debug, Deserialize, Serialize)]
struct CaptureConfig {
    device_name: String,
    file_prefix: String,
    file_path: String,
    file_format: String,
    packet_limit: Option<usize>,
    snaplen: i32,
    timeout_ms: i32,
    // 滚动保存相关配置
    continuous_capture: Option<bool>,
    rollover_time_seconds: Option<u64>,
    rollover_packet_count: Option<usize>,
    rollover_file_size_mb: Option<u64>,
}

// 命令行参数定义
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 网络设备名称
    #[arg(short, long)]
    device_name: Option<String>,

    /// 输出文件前缀
    #[arg(short = 'p', long)]
    file_prefix: Option<String>,

    /// 输出文件路径
    #[arg(short = 'o', long, default_value = "./")]
    file_path: Option<String>,

    /// 输出文件格式 (pcap 或 pcapng)
    #[arg(short, long, default_value = "pcap")]
    file_format: Option<String>,

    /// 捕获的数据包数量限制
    #[arg(short = 'l', long)]
    packet_limit: Option<usize>,

    /// 捕获的数据包大小限制
    #[arg(short = 's', long, default_value_t = 65535)]
    snaplen: i32,

    /// 捕获超时时间(毫秒)
    #[arg(short = 't', long, default_value_t = 1000)]
    timeout_ms: i32,

    /// 配置文件路径
    #[arg(short, long)]
    config_file: Option<String>,

    /// 启用持续捕获模式
    #[arg(long, default_value_t = false)]
    continuous_capture: bool,

    /// 滚动保存时间间隔(秒)
    #[arg(long)]
    rollover_time_seconds: Option<u64>,

    /// 每个文件最大数据包数量
    #[arg(long)]
    rollover_packet_count: Option<usize>,

    /// 每个文件最大大小(MB)
    #[arg(long)]
    rollover_file_size_mb: Option<u64>,
}

// 将字符串转换为FileFormat枚举
fn str_to_file_format(format: &str) -> Result<FileFormat> {
    match format.to_lowercase().as_str() {
        "pcap" => Ok(FileFormat::Pcap),
        "pcapng" => Ok(FileFormat::PcapNg),
        _ => Err(anyhow::anyhow!("不支持的文件格式: {}", format)),
    }
}

// 从配置文件加载配置
fn load_config_from_file(file_path: &str) -> Result<CaptureConfig> {
    let config_str = fs::read_to_string(file_path)
        .with_context(|| format!("无法读取配置文件: {}", file_path))?;
    let config: CaptureConfig = serde_json::from_str(&config_str)
        .with_context(|| format!("配置文件格式错误: {}", file_path))?;
    Ok(config)
}

fn main() -> Result<()> {
    env_logger::init();
    // 解析命令行参数
    let args = Args::parse();

    // 构建PcapCaptureOptions
    let options = if let Some(config_file) = &args.config_file {
        // 从配置文件加载配置
        let config = load_config_from_file(config_file)?;

        // 创建选项，命令行参数优先级高于配置文件
        PcapCaptureOptions {
            packet_source: save_pcap::PacketSource::NetworkDevice(
                args.device_name.unwrap_or(config.device_name),
            ),
            file_prefix: args.file_prefix.unwrap_or(config.file_prefix),
            file_path: args.file_path.unwrap_or(config.file_path),
            file_format: str_to_file_format(&args.file_format.unwrap_or(config.file_format))?,
            packet_limit: args.packet_limit.or(config.packet_limit),
            snaplen: args.snaplen,
            timeout_ms: args.timeout_ms,
            continuous_capture: if args.continuous_capture {
                true
            } else {
                config.continuous_capture.unwrap_or(false)
            },
            rollover_time_seconds: args.rollover_time_seconds.or(config.rollover_time_seconds),
            rollover_packet_count: args.rollover_packet_count.or(config.rollover_packet_count),
            rollover_file_size_mb: args.rollover_file_size_mb.or(config.rollover_file_size_mb),
        }
    } else {
        // 仅使用命令行参数
        PcapCaptureOptions {
            packet_source: save_pcap::PacketSource::NetworkDevice(args.device_name.ok_or_else(
                || anyhow::anyhow!("必须提供网络设备名称，请使用--device-name参数或配置文件"),
            )?),
            file_prefix: args.file_prefix.ok_or_else(|| {
                anyhow::anyhow!("必须提供文件前缀，请使用--file-prefix参数或配置文件")
            })?,
            file_path: args.file_path.unwrap_or("./".to_string()),
            file_format: str_to_file_format(&args.file_format.unwrap_or("pcap".to_string()))?,
            packet_limit: args.packet_limit,
            snaplen: args.snaplen,
            timeout_ms: args.timeout_ms,
            continuous_capture: args.continuous_capture,
            rollover_time_seconds: args.rollover_time_seconds,
            rollover_packet_count: args.rollover_packet_count,
            rollover_file_size_mb: args.rollover_file_size_mb,
        }
    };

    // 打印配置信息
    println!("开始捕获数据包...");
    println!(
        "设备名称: {:?}",
        match &options.packet_source {
            save_pcap::PacketSource::NetworkDevice(name) => name.as_str(),
            _ => "用户提供的数据包",
        }
    );
    println!("输出路径: {}", options.file_path);
    println!("文件前缀: {}", options.file_prefix);
    println!("文件格式: {:?}", options.file_format);
    if let Some(limit) = options.packet_limit {
        println!("数据包限制: {}", limit);
    }
    println!("快照长度: {}", options.snaplen);
    println!("超时时间: {}ms", options.timeout_ms);
    println!(
        "持续捕获模式: {}",
        if options.continuous_capture {
            "已启用"
        } else {
            "已禁用"
        }
    );
    if let Some(seconds) = options.rollover_time_seconds {
        println!("时间滚动间隔: {}秒", seconds);
    }
    if let Some(count) = options.rollover_packet_count {
        println!("数据包数量滚动阈值: {}个", count);
    }
    if let Some(size) = options.rollover_file_size_mb {
        println!("文件大小滚动阈值: {}MB", size);
    }

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);
    match capturer.capture() {
        Ok(_) => println!("捕获完成！"),
        Err(e) => eprintln!("捕获失败：{}", e),
    }

    Ok(())
}

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
    // 解析命令行参数
    let args = Args::parse();

    // 构建PcapCaptureOptions
    let options = if let Some(config_file) = &args.config_file {
        // 从配置文件加载配置
        let config = load_config_from_file(config_file)?;

        // 创建选项，命令行参数优先级高于配置文件
        PcapCaptureOptions {
            device_name: args.device_name.unwrap_or(config.device_name),
            file_prefix: args.file_prefix.unwrap_or(config.file_prefix),
            file_path: args.file_path.unwrap_or(config.file_path),
            file_format: str_to_file_format(&args.file_format.unwrap_or(config.file_format))?,
            packet_limit: args.packet_limit.or(config.packet_limit),
            snaplen: args.snaplen,
            timeout_ms: args.timeout_ms,
        }
    } else {
        // 仅使用命令行参数
        PcapCaptureOptions {
            device_name: args.device_name.ok_or_else(|| {
                anyhow::anyhow!("必须提供网络设备名称，请使用--device-name参数或配置文件")
            })?,
            file_prefix: args.file_prefix.ok_or_else(|| {
                anyhow::anyhow!("必须提供文件前缀，请使用--file-prefix参数或配置文件")
            })?,
            file_path: args.file_path.unwrap_or("./".to_string()),
            file_format: str_to_file_format(&args.file_format.unwrap_or("pcap".to_string()))?,
            packet_limit: args.packet_limit,
            snaplen: args.snaplen,
            timeout_ms: args.timeout_ms,
        }
    };

    // 打印配置信息
    println!("开始捕获数据包...");
    println!("设备名称: {}", options.device_name);
    println!("输出路径: {}", options.file_path);
    println!("文件前缀: {}", options.file_prefix);
    println!("文件格式: {:?}", options.file_format);
    if let Some(limit) = options.packet_limit {
        println!("数据包限制: {}", limit);
    }
    println!("快照长度: {}", options.snaplen);
    println!("超时时间: {}ms", options.timeout_ms);

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);
    match capturer.capture() {
        Ok(_) => println!("捕获完成！"),
        Err(e) => eprintln!("捕获失败：{}", e),
    }

    Ok(())
}

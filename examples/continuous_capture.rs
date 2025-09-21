use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(serde::Deserialize)]
struct Config {
    primary_network_device: String,
    default_file_prefix: String,
    default_file_path: String,
    default_packet_limit: Option<usize>,
    default_rollover_time_seconds: Option<u64>,
    default_rollover_packet_count: Option<usize>,
    default_rollover_file_size_mb: Option<u64>,
}

fn main() {
    env_logger::init();
    
    // 从配置文件读取配置信息
    let config_path = Path::new("config/local_config.json");
    let mut config_file = match File::open(&config_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("错误：无法打开配置文件 {:?}：{}", config_path, e);
            std::process::exit(1);
        }
    };

    let mut config_content = String::new();
    if let Err(e) = config_file.read_to_string(&mut config_content) {
        eprintln!("错误：无法读取配置文件内容：{}", e);
        std::process::exit(1);
    }

    let config: Config = match serde_json::from_str(&config_content) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("错误：配置文件格式无效：{}", e);
            std::process::exit(1);
        }
    };

    // 设置连续捕获选项
    let options = PcapCaptureOptions {
        packet_source: save_pcap::PacketSource::NetworkDevice(
            config.primary_network_device.clone(),
        ),
        file_prefix: config.default_file_prefix,
        file_path: config.default_file_path,
        file_format: FileFormat::Pcap,
        packet_limit: config.default_packet_limit,
        snaplen: 65535,
        timeout_ms: 1000,
        continuous_capture: true, // 启用持续捕获模式
        // 设置滚动保存参数（根据需要选择一个或多个）
        rollover_time_seconds: config.default_rollover_time_seconds,
        rollover_packet_count: config.default_rollover_packet_count,
        rollover_file_size_mb: config.default_rollover_file_size_mb,
    };

    println!("启动持续数据包捕获和滚动保存...");
    println!(
        "设备: {:?}",
        match &options.packet_source {
            save_pcap::PacketSource::NetworkDevice(name) => name.as_str(),
            _ => "用户提供的数据包",
        }
    );
    println!("文件前缀: {}", options.file_prefix);
    println!("文件路径: {}", options.file_path);
    println!("连续捕获模式: 已启用");

    if let Some(seconds) = options.rollover_time_seconds {
        println!("时间滚动间隔: {}秒", seconds);
    }
    if let Some(count) = options.rollover_packet_count {
        println!("数据包数量滚动阈值: {}个", count);
    }
    if let Some(size) = options.rollover_file_size_mb {
        println!("文件大小滚动阈值: {}MB", size);
    }

    println!("按Ctrl+C键停止捕获...");

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);
    match capturer.capture() {
        Ok(_) => println!("捕获完成！"),
        Err(e) => eprintln!("捕获失败：{}", e),
    }
}

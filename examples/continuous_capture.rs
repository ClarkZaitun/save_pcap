use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};
use std::time::Duration;

fn main() {
    // 设置连续捕获选项
    let options = PcapCaptureOptions {
        device_name: "your_network_device".to_string(), // 请替换为你的实际网卡名称，可通过list_devices示例获取
        file_prefix: "continuous_capture".to_string(),
        file_path: "./".to_string(),
        file_format: FileFormat::Pcap,
        packet_limit: Some(10000), // 可选：全局限制捕获的数据包总数，达到后停止
        snaplen: 65535,
        timeout_ms: 1000,
        continuous_capture: true, // 启用持续捕获模式
        // 设置滚动保存参数（根据需要选择一个或多个）
        rollover_time_seconds: Some(30), // 每30秒创建一个新文件，便于快速测试
        rollover_packet_count: Some(500), // 每个文件最多保存500个数据包
        rollover_file_size_mb: Some(5),  // 每个文件最大5MB
    };

    println!("启动持续数据包捕获和滚动保存...");
    println!("设备: {}", options.device_name);
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

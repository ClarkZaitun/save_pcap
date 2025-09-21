use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};

fn main() {
    // 设置捕获选项
    let options = PcapCaptureOptions {
        device_name: "your_network_device".to_string(), // 替换为你的实际网卡名称
        file_prefix: "capture_test".to_string(),
        file_path: "./".to_string(),
        file_format: FileFormat::Pcap,
        packet_limit: Some(100), // 只捕获100个包就停止，避免长时间运行
        snaplen: 65535,
        timeout_ms: 1000,
        continuous_capture: false,   // 禁用持续捕获
        rollover_time_seconds: None, // 无时间滚动
        rollover_packet_count: None, // 无数据包数量滚动
        rollover_file_size_mb: None, // 无文件大小滚动
    };

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);
    match capturer.capture() {
        Ok(_) => println!("捕获完成！"),
        Err(e) => eprintln!("捕获失败：{}", e),
    }
}

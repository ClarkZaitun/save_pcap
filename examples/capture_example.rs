use save_pcap::{
    FileFormat, PcapCaptureOptions, PcapCapturer, SavePcapError, get_available_devices,
};
use std::io::{self, BufRead, Write};

fn main() {
    match run() {
        Ok(_) => println!("Capture completed successfully"),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn run() -> Result<(), SavePcapError> {
    // 列出所有可用的网络设备
    println!("Available network devices:");
    let devices = get_available_devices()?;

    for (i, device) in devices.iter().enumerate() {
        println!("{}. {}", i + 1, device);
    }

    // 让用户选择一个设备
    print!("Enter the device number to capture: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;

    let device_index: usize = input
        .trim()
        .parse()
        .map_err(|_| SavePcapError::InvalidDevice("Invalid device number".to_string()))?;

    if device_index < 1 || device_index > devices.len() {
        return Err(SavePcapError::InvalidDevice(
            "Device number out of range".to_string(),
        ));
    }

    let device_name = devices[device_index - 1].clone();

    // 让用户设置文件前缀
    print!("Enter file prefix (default: 'capture'): ");
    io::stdout().flush()?;

    let mut file_prefix = String::new();
    io::stdin().lock().read_line(&mut file_prefix)?;

    let file_prefix = if file_prefix.trim().is_empty() {
        "capture".to_string()
    } else {
        file_prefix.trim().to_string()
    };

    // 让用户设置文件路径
    print!("Enter file path (default: '.'): ");
    io::stdout().flush()?;

    let mut file_path = String::new();
    io::stdin().lock().read_line(&mut file_path)?;

    let file_path = if file_path.trim().is_empty() {
        ".".to_string()
    } else {
        file_path.trim().to_string()
    };

    // 让用户选择文件格式
    print!("Enter file format (1 for pcap, 2 for pcapng, default: 1): ");
    io::stdout().flush()?;

    let mut format_input = String::new();
    io::stdin().lock().read_line(&mut format_input)?;

    let file_format = match format_input.trim() {
        "2" => FileFormat::PcapNg,
        _ => FileFormat::Pcap,
    };

    // 让用户设置数据包限制（可选）
    print!("Enter packet limit (leave empty for unlimited): ");
    io::stdout().flush()?;

    let mut packet_limit_input = String::new();
    io::stdin().lock().read_line(&mut packet_limit_input)?;

    let packet_limit = if packet_limit_input.trim().is_empty() {
        None
    } else {
        Some(
            packet_limit_input
                .trim()
                .parse()
                .map_err(|_| SavePcapError::InvalidDevice("Invalid packet limit".to_string()))?,
        )
    };

    // 让用户选择是否启用持续捕获模式
    print!("Enable continuous capture mode? (y/n, default: n): ");
    io::stdout().flush()?;

    let mut continuous_capture_input = String::new();
    io::stdin()
        .lock()
        .read_line(&mut continuous_capture_input)?;

    let continuous_capture = continuous_capture_input.trim().to_lowercase() == "y";

    // 如果启用持续捕获模式，让用户设置滚动参数
    let (rollover_time_seconds, rollover_packet_count, rollover_file_size_mb) =
        if continuous_capture {
            // 让用户设置时间滚动间隔（秒）
            print!(
                "Enter rollover time interval in seconds (leave empty for no time-based rollover): "
            );
            io::stdout().flush()?;

            let mut time_input = String::new();
            io::stdin().lock().read_line(&mut time_input)?;
            let time_value = if time_input.trim().is_empty() {
                None
            } else {
                Some(time_input.trim().parse().map_err(|_| {
                    SavePcapError::InvalidDevice("Invalid time interval".to_string())
                })?)
            };

            // 让用户设置数据包数量滚动阈值
            print!(
                "Enter packet count rollover threshold (leave empty for no packet-based rollover): "
            );
            io::stdout().flush()?;

            let mut count_input = String::new();
            io::stdin().lock().read_line(&mut count_input)?;
            let count_value = if count_input.trim().is_empty() {
                None
            } else {
                Some(count_input.trim().parse().map_err(|_| {
                    SavePcapError::InvalidDevice("Invalid packet count".to_string())
                })?)
            };

            // 让用户设置文件大小滚动阈值（MB）
            print!(
                "Enter file size rollover threshold in MB (leave empty for no size-based rollover): "
            );
            io::stdout().flush()?;

            let mut size_input = String::new();
            io::stdin().lock().read_line(&mut size_input)?;
            let size_value =
                if size_input.trim().is_empty() {
                    None
                } else {
                    Some(size_input.trim().parse().map_err(|_| {
                        SavePcapError::InvalidDevice("Invalid file size".to_string())
                    })?)
                };

            (time_value, count_value, size_value)
        } else {
            (None, None, None)
        };

    // 创建捕获选项
    let options = PcapCaptureOptions {
        device_name,
        file_prefix,
        file_path,
        file_format,
        packet_limit,
        snaplen: 65535,   // 默认捕获长度
        timeout_ms: 1000, // 默认超时时间
        continuous_capture,
        rollover_time_seconds,
        rollover_packet_count,
        rollover_file_size_mb,
    };

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);

    println!("Starting capture. Press Ctrl+C to stop.");
    capturer.capture()?;

    Ok(())
}

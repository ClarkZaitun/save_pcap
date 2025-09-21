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

    // 创建捕获选项
    let options = PcapCaptureOptions {
        device_name,
        file_prefix,
        file_path,
        file_format,
        packet_limit,
        snaplen: 65535,   // 默认捕获长度
        timeout_ms: 1000, // 默认超时时间
    };

    // 创建捕获器并开始捕获
    let capturer = PcapCapturer::new(options);

    println!("Starting capture. Press Ctrl+C to stop.");
    capturer.capture()?;

    Ok(())
}

// 这个示例展示如何使用用户提供的数据包功能
// 你可以通过这个示例学习如何创建自己的数据包并发送给save_pcap库保存

use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer, UserPacket};
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();
    
    // 创建捕获选项，设置为用户提供数据包模式
    let options = PcapCaptureOptions {
        packet_source: save_pcap::PacketSource::UserProvided,
        file_prefix: "user_packets".to_string(),
        file_path: ".".to_string(),
        file_format: FileFormat::Pcap,
        snaplen: 65535,
        timeout_ms: 1000,
        continuous_capture: true,
        packet_limit: Some(100),         // 限制捕获100个数据包
        rollover_packet_count: Some(20), // 每20个数据包创建一个新文件
        rollover_file_size_mb: None,     // 不按文件大小滚动
        rollover_time_seconds: None,     // 不按时间滚动
    };

    // 创建捕获器
    let capturer = PcapCapturer::new(options);

    // 获取数据包发送器
    let packet_sender = match capturer.get_packet_sender() {
        Some(sender) => sender,
        None => {
            eprintln!("Failed to get packet sender");
            return;
        }
    };

    // 启动一个线程来提供数据包
    let sender_thread = thread::spawn(move || {
        // 发送一些示例数据包
        for i in 0..100 {
            // 创建一个简单的以太网数据包
            let mut packet_data = vec![0u8; 64];

            // 设置目标MAC地址
            packet_data[0..6].copy_from_slice(&[0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
            // 设置源MAC地址
            packet_data[6..12].copy_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
            // 设置以太网类型为IPv4
            packet_data[12..14].copy_from_slice(&[0x08, 0x00]);
            // 在数据包中添加一些标识信息
            packet_data[14] = (i & 0xFF) as u8;
            packet_data[15] = ((i >> 8) & 0xFF) as u8;

            // 创建用户数据包
            let user_packet = UserPacket {
                data: packet_data,
                timestamp: None, // 使用当前时间作为时间戳
            };

            // 发送数据包
            if let Err(err) = packet_sender.send(user_packet) {
                eprintln!("Failed to send packet {}: {:?}", i, err);
                break;
            }

            // 每100毫秒发送一个数据包
            thread::sleep(Duration::from_millis(100));
        }
    });

    // 启动捕获
    if let Err(err) = capturer.capture() {
        eprintln!("Capture error: {:?}", err);
    }

    // 等待发送线程完成
    if let Err(err) = sender_thread.join() {
        eprintln!("Sender thread error: {:?}", err);
    }

    println!("User provided packet capture completed");
}

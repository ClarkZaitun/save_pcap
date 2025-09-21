use save_pcap::get_available_devices;

fn main() {
    match get_available_devices() {
        Ok(devices) => {
            println!("可用的网络设备列表:");
            for device in devices {
                println!("- {}", device);
            }
            println!();
            println!("请在运行示例程序时使用上述设备名称之一。");
        }
        Err(e) => eprintln!("获取设备列表失败: {}", e),
    }
}

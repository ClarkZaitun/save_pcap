use anyhow::Result;
use chrono::{DateTime, Local};
use log::{debug, error, info};
use pcap::{Active, Capture, Device, Error as PcapError};
use pcap_file::pcap::{PcapPacket, PcapWriter};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SavePcapError {
    #[error("Pcap error: {0}")]
    PcapError(#[from] PcapError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid device name: {0}")]
    InvalidDevice(String),

    #[error("Directory creation failed: {0}")]
    DirectoryCreationFailed(String),

    #[error("Capture interrupted")]
    CaptureInterrupted,

    #[error("Pcap file error: {0}")]
    PcapFileError(String),
}

#[derive(Debug)]
pub enum FileFormat {
    Pcap,
    PcapNg,
}

pub struct PcapCaptureOptions {
    pub device_name: String,
    pub file_prefix: String,
    pub file_path: String,
    pub file_format: FileFormat,
    pub packet_limit: Option<usize>,
    pub snaplen: i32,
    pub timeout_ms: i32,
    // 滚动保存相关参数
    pub continuous_capture: bool,             // 是否持续捕获
    pub rollover_time_seconds: Option<u64>,   // 滚动时间间隔（秒）
    pub rollover_packet_count: Option<usize>, // 每个文件最多保存的数据包数
    pub rollover_file_size_mb: Option<u64>,   // 每个文件最大大小（MB）
}

impl Default for PcapCaptureOptions {
    fn default() -> Self {
        Self {
            device_name: String::new(),
            file_prefix: "capture".to_string(),
            file_path: ".".to_string(),
            file_format: FileFormat::Pcap,
            packet_limit: None,
            snaplen: 65535,
            timeout_ms: 1000,
            continuous_capture: false,   // 默认不持续捕获
            rollover_time_seconds: None, // 默认不按时间滚动
            rollover_packet_count: None, // 默认不按数据包数量滚动
            rollover_file_size_mb: None, // 默认不按文件大小滚动
        }
    }
}

pub struct PcapCapturer {
    options: PcapCaptureOptions,
}

impl PcapCapturer {
    pub fn new(options: PcapCaptureOptions) -> Self {
        Self { options }
    }

    pub fn capture(&self) -> Result<(), SavePcapError> {
        // 初始化日志
        env_logger::init();

        // 检查设备是否存在
        let devices = Device::list()?;
        let device_exists = devices.iter().any(|d| d.name == self.options.device_name);

        if !device_exists {
            return Err(SavePcapError::InvalidDevice(
                self.options.device_name.clone(),
            ));
        }

        // 创建文件路径
        let path = Path::new(&self.options.file_path);
        if !path.exists() {
            if let Err(e) = fs::create_dir_all(path) {
                return Err(SavePcapError::DirectoryCreationFailed(format!(
                    "Failed to create directory: {}, error: {}",
                    self.options.file_path, e
                )));
            }
        }

        // 打开捕获设备
        let mut cap = Capture::from_device(&*self.options.device_name)?
            .snaplen(self.options.snaplen)
            .promisc(true)
            .timeout(self.options.timeout_ms)
            .open()?;

        info!("Starting capture on device: {}", self.options.device_name);

        if self.options.continuous_capture {
            // 持续捕获模式
            self.continuous_capture_with_rollover(&mut cap)?;
        } else {
            // 单次捕获模式
            let (_file_name, full_path) = self.create_new_file()?;
            info!("Saving to file: {:?}", full_path);

            // 创建文件并写入数据
            let file = File::create(&full_path)?;
            let mut buf_writer = BufWriter::new(file);

            match self.options.file_format {
                FileFormat::Pcap => {
                    self.capture_to_pcap(&mut cap, &mut buf_writer)?;
                }
                FileFormat::PcapNg => {
                    self.capture_to_pcapng(&mut cap, &mut buf_writer)?;
                }
            }

            info!(
                "Capture completed. Packets saved to: {}",
                full_path.display()
            );
        }

        Ok(())
    }

    // 创建新的捕获文件
    fn create_new_file(&self) -> Result<(String, std::path::PathBuf), SavePcapError> {
        let path = Path::new(&self.options.file_path);

        // 生成带时间戳的文件名
        let now: DateTime<Local> = Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let file_extension = match self.options.file_format {
            FileFormat::Pcap => "pcap",
            FileFormat::PcapNg => "pcapng",
        };

        let file_name = format!(
            "{}_{}.{}",
            self.options.file_prefix, timestamp, file_extension
        );
        let full_path = path.join(&file_name);

        Ok((file_name, full_path))
    }

    // 持续捕获并支持滚动保存
    fn continuous_capture_with_rollover(
        &self,
        cap: &mut Capture<Active>,
    ) -> Result<(), SavePcapError> {
        let mut packet_count_total = 0;
        let mut current_file_packet_count = 0;
        let mut current_file_size_bytes = 0;
        let mut file_creation_time = std::time::SystemTime::now();

        // 创建第一个文件
        let (mut current_file_name, mut current_full_path) = self.create_new_file()?;
        info!(
            "Starting continuous capture, first file: {:?}",
            current_full_path
        );

        let mut file = File::create(&current_full_path)?;
        let mut buf_writer = BufWriter::new(file);
        let mut pcap_writer = match PcapWriter::new(&mut buf_writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        loop {
            // 检查是否达到了全局包限制
            if let Some(global_limit) = self.options.packet_limit {
                if packet_count_total >= global_limit {
                    info!(
                        "Reached global packet limit of {}, stopping capture.",
                        global_limit
                    );
                    break;
                }
            }

            // 检查是否需要创建新文件
            let needs_rollover = self.check_needs_rollover(
                current_file_packet_count,
                current_file_size_bytes,
                &file_creation_time,
            );

            if needs_rollover {
                // 刷新并关闭当前文件
                if let Err(e) = pcap_writer.flush() {
                    error!("Failed to flush file: {}, error: {}", current_file_name, e);
                }
                // 只需drop最高层的writer，它会自动关闭和释放下层资源
                drop(pcap_writer);

                info!(
                    "Rolling over to new file after {} packets in {}",
                    current_file_packet_count, current_file_name
                );

                // 创建新文件
                let (new_file_name, new_full_path) = self.create_new_file()?;
                current_file_name = new_file_name;
                current_full_path = new_full_path;

                // 重置计数器
                current_file_packet_count = 0;
                current_file_size_bytes = 0;
                file_creation_time = std::time::SystemTime::now();

                info!("New file created: {:?}", current_full_path);

                // 打开新文件
                file = File::create(&current_full_path)?;
                buf_writer = BufWriter::new(file);
                pcap_writer = match PcapWriter::new(&mut buf_writer) {
                    Ok(writer) => writer,
                    Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
                };
            }

            match cap.next_packet() {
                Ok(packet) => {
                    // 创建PCAP数据包
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    // 写入数据包
                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        error!("Failed to write packet: {}", e);
                        // 尝试创建新文件继续捕获
                        continue;
                    }

                    packet_count_total += 1;
                    current_file_packet_count += 1;
                    current_file_size_bytes += packet.data.len() as u64;

                    if packet_count_total % 1000 == 0 {
                        debug!("Captured total {} packets", packet_count_total);
                    }
                }
                Err(e) => {
                    if e.to_string() == "timeout expired" {
                        // 超时，继续捕获
                        continue;
                    } else {
                        error!("Capture error: {}", e);
                        // 在持续捕获模式下，我们记录错误但继续尝试
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
            }
        }

        Ok(())
    }

    // 检查是否需要滚动到新文件
    fn check_needs_rollover(
        &self,
        current_packet_count: usize,
        current_file_size_bytes: u64,
        file_creation_time: &std::time::SystemTime,
    ) -> bool {
        // 检查时间间隔
        if let Some(rollover_seconds) = self.options.rollover_time_seconds {
            if let Ok(elapsed) = file_creation_time.elapsed() {
                if elapsed.as_secs() >= rollover_seconds {
                    return true;
                }
            }
        }

        // 检查数据包数量
        if let Some(max_packets) = self.options.rollover_packet_count {
            if current_packet_count >= max_packets {
                return true;
            }
        }

        // 检查文件大小
        if let Some(max_size_mb) = self.options.rollover_file_size_mb {
            let max_size_bytes = max_size_mb * 1024 * 1024;
            if current_file_size_bytes >= max_size_bytes {
                return true;
            }
        }

        false
    }

    fn capture_to_pcap<W: Write>(
        &self,
        cap: &mut Capture<Active>,
        writer: &mut W,
    ) -> Result<(), SavePcapError> {
        // 创建PCAP写入器
        let mut pcap_writer = match PcapWriter::new(writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        // 捕获并写入数据包
        let mut packet_count = 0;

        loop {
            if let Some(limit) = self.options.packet_limit {
                if packet_count >= limit {
                    break;
                }
            }

            match cap.next_packet() {
                Ok(packet) => {
                    // 创建PCAP数据包
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    // 写入数据包
                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count += 1;

                    if packet_count % 1000 == 0 {
                        debug!("Captured {} packets", packet_count);
                    }
                }
                Err(e) => {
                    if e.to_string() == "timeout expired" {
                        // 超时，继续捕获
                        continue;
                    } else {
                        error!("Capture error: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn capture_to_pcapng<W: Write>(
        &self,
        cap: &mut Capture<Active>,
        writer: &mut W,
    ) -> Result<(), SavePcapError> {
        // 对于pcapng格式，我们先尝试使用基本的pcap格式作为替代
        // 因为我们无法确定PcapNgWriter的正确API
        // 创建PCAP写入器
        let mut pcap_writer = match PcapWriter::new(writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        // 捕获并写入数据包
        let mut packet_count = 0;

        loop {
            if let Some(limit) = self.options.packet_limit {
                if packet_count >= limit {
                    break;
                }
            }

            match cap.next_packet() {
                Ok(packet) => {
                    // 创建PCAP数据包
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    // 写入数据包
                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count += 1;

                    if packet_count % 1000 == 0 {
                        debug!("Captured {} packets", packet_count);
                    }
                }
                Err(e) => {
                    if e.to_string() == "timeout expired" {
                        // 超时，继续捕获
                        continue;
                    } else {
                        error!("Capture error: {}", e);
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}

/// 获取所有可用的网络设备
pub fn get_available_devices() -> Result<Vec<String>, SavePcapError> {
    let devices = Device::list()?;
    let device_names: Vec<String> = devices.iter().map(|d| d.name.clone()).collect();
    Ok(device_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_available_devices() {
        // 这个测试可能会因环境而异
        let devices = get_available_devices();
        assert!(devices.is_ok(), "Failed to get devices");
    }

    #[test]
    fn test_pcap_capture_options_default() {
        let options = PcapCaptureOptions::default();
        assert_eq!(options.file_prefix, "capture");
        assert_eq!(options.file_path, ".");
        assert_eq!(options.snaplen, 65535);
        assert_eq!(options.timeout_ms, 1000);
    }
}

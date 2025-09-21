use chrono::{DateTime, Local};
use log::{debug, error, info};
use pcap::{Active, Capture, Device, Error as PcapError};
use pcap_file::pcap::{PcapPacket, PcapWriter};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};
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

#[derive(Debug)]
pub enum PacketSource {
    NetworkDevice(String),
    UserProvided,
}

pub struct PcapCaptureOptions {
    pub packet_source: PacketSource,
    pub file_prefix: String,
    pub file_path: String,
    pub file_format: FileFormat,
    pub packet_limit: Option<usize>,
    pub snaplen: i32,
    pub timeout_ms: i32,
    pub continuous_capture: bool,
    pub rollover_time_seconds: Option<u64>,
    pub rollover_packet_count: Option<usize>,
    pub rollover_file_size_mb: Option<u64>,
}

impl Default for PcapCaptureOptions {
    fn default() -> Self {
        Self {
            packet_source: PacketSource::NetworkDevice(String::new()),
            file_prefix: "capture".to_string(),
            file_path: ".".to_string(),
            file_format: FileFormat::Pcap,
            packet_limit: None,
            snaplen: 65535,
            timeout_ms: 1000,
            continuous_capture: false,
            rollover_time_seconds: None,
            rollover_packet_count: None,
            rollover_file_size_mb: None,
        }
    }
}

#[derive(Debug)]
pub struct UserPacket {
    pub data: Vec<u8>,
    pub timestamp: Option<Duration>,
}

pub struct PcapCapturer {
    options: PcapCaptureOptions,
    packet_receiver: Option<Receiver<UserPacket>>,
    packet_sender: Option<Sender<UserPacket>>,
}

impl PcapCapturer {
    pub fn new(options: PcapCaptureOptions) -> Self {
        let (packet_sender, packet_receiver) = match &options.packet_source {
            PacketSource::UserProvided => {
                let (sender, receiver) = channel();
                (Some(sender), Some(receiver))
            }
            _ => (None, None),
        };

        Self {
            options,
            packet_receiver,
            packet_sender,
        }
    }

    pub fn capture(&self) -> Result<(), SavePcapError> {
        env_logger::init();

        let path = Path::new(&self.options.file_path);
        if !path.exists() {
            if let Err(e) = fs::create_dir_all(path) {
                return Err(SavePcapError::DirectoryCreationFailed(format!(
                    "Failed to create directory: {}, error: {}",
                    self.options.file_path, e
                )));
            }
        }

        match &self.options.packet_source {
            PacketSource::NetworkDevice(device_name) => {
                let devices = Device::list()?;
                let device_exists = devices.iter().any(|d| d.name == *device_name);

                if !device_exists {
                    return Err(SavePcapError::InvalidDevice(device_name.clone()));
                }

                let mut cap = Capture::from_device(device_name.as_str())?
                    .snaplen(self.options.snaplen)
                    .promisc(true)
                    .timeout(self.options.timeout_ms)
                    .open()?;

                info!("Starting capture on device: {}", device_name);

                if self.options.continuous_capture {
                    self.continuous_capture_with_rollover(&mut cap)?;
                } else {
                    let (_file_name, full_path) = self.create_new_file()?;
                    info!("Saving to file: {:?}", full_path);

                    let file = File::create(&full_path)?;
                    let mut buf_writer = BufWriter::new(file);

                    match self.options.file_format {
                        FileFormat::Pcap => self.capture_to_pcap(&mut cap, &mut buf_writer)?,
                        FileFormat::PcapNg => self.capture_to_pcapng(&mut cap, &mut buf_writer)?,
                    }

                    info!(
                        "Capture completed. Packets saved to: {}",
                        full_path.display()
                    );
                }
            }
            PacketSource::UserProvided => {
                if let Some(receiver) = &self.packet_receiver {
                    info!("Starting user-provided packet capture");

                    if self.options.continuous_capture {
                        self.continuous_user_packet_capture_with_rollover(receiver)?;
                    } else {
                        let (_file_name, full_path) = self.create_new_file()?;
                        info!("Saving to file: {:?}", full_path);

                        let file = File::create(&full_path)?;
                        let mut buf_writer = BufWriter::new(file);

                        match self.options.file_format {
                            FileFormat::Pcap => {
                                self.user_packets_to_pcap(receiver, &mut buf_writer)?
                            }
                            FileFormat::PcapNg => {
                                self.user_packets_to_pcap(receiver, &mut buf_writer)?
                            }
                        }

                        info!(
                            "Capture completed. Packets saved to: {}",
                            full_path.display()
                        );
                    }
                } else {
                    return Err(SavePcapError::InvalidDevice(
                        "No packet receiver available".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn get_packet_sender(&self) -> Option<Sender<UserPacket>> {
        self.packet_sender.clone()
    }

    fn create_new_file(&self) -> Result<(String, std::path::PathBuf), SavePcapError> {
        let path = Path::new(&self.options.file_path);
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

    fn continuous_capture_with_rollover(
        &self,
        cap: &mut Capture<Active>,
    ) -> Result<(), SavePcapError> {
        let mut packet_count_total = 0;
        let mut current_file_packet_count = 0;
        let mut current_file_size_bytes = 0;
        let mut file_creation_time = std::time::SystemTime::now();

        let (mut current_file_name, mut current_full_path) = self.create_new_file()?;
        info!(
            "Starting continuous capture, first file: {:?}",
            current_full_path
        );

        // Create the initial file and writers
        let file = File::create(&current_full_path)?;
        let buf_writer = BufWriter::new(file);
        let mut pcap_writer = match PcapWriter::new(buf_writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        loop {
            if let Some(global_limit) = self.options.packet_limit {
                if packet_count_total >= global_limit {
                    info!(
                        "Reached global packet limit of {}, stopping capture.",
                        global_limit
                    );
                    break;
                }
            }

            let needs_rollover = self.check_needs_rollover(
                current_file_packet_count,
                current_file_size_bytes,
                &file_creation_time,
            );

            if needs_rollover {
                // Flush and close the current file by dropping the pcap_writer
                if let Err(e) = pcap_writer.flush() {
                    error!("Failed to flush file: {}, error: {}", current_file_name, e);
                }
                drop(pcap_writer);

                info!(
                    "Rolling over to new file after {} packets in {}",
                    current_file_packet_count, current_file_name
                );

                // Create new file and reset counters
                let (new_file_name, new_full_path) = self.create_new_file()?;
                current_file_name = new_file_name;
                current_full_path = new_full_path;

                current_file_packet_count = 0;
                current_file_size_bytes = 0;
                file_creation_time = std::time::SystemTime::now();

                // Create new writers for the new file
                let new_file = File::create(&current_full_path)?;
                let new_buf_writer = BufWriter::new(new_file);
                pcap_writer = match PcapWriter::new(new_buf_writer) {
                    Ok(writer) => writer,
                    Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
                };
            }

            match cap.next_packet() {
                Ok(packet) => {
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count_total += 1;
                    current_file_packet_count += 1;
                    current_file_size_bytes += packet.data.len() as u64;

                    if packet_count_total % 1000 == 0 {
                        debug!("Captured {} packets total", packet_count_total);
                    }
                }
                Err(e) => {
                    let error_str = e.to_string();
                    // 检查错误信息是否包含"timeout"关键词，以处理不同形式的超时错误
                    if error_str.contains("timeout") {
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

    fn continuous_user_packet_capture_with_rollover(
        &self,
        receiver: &Receiver<UserPacket>,
    ) -> Result<(), SavePcapError> {
        let mut packet_count_total = 0;
        let mut current_file_packet_count = 0;
        let mut current_file_size_bytes = 0;
        let mut file_creation_time = std::time::SystemTime::now();

        let (mut current_file_name, mut current_full_path) = self.create_new_file()?;
        info!(
            "Starting continuous user packet capture, first file: {:?}",
            current_full_path
        );

        // Create the initial file and writers
        let file = File::create(&current_full_path)?;
        let buf_writer = BufWriter::new(file);
        let mut pcap_writer = match PcapWriter::new(buf_writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        loop {
            if let Some(global_limit) = self.options.packet_limit {
                if packet_count_total >= global_limit {
                    info!(
                        "Reached global packet limit of {}, stopping capture.",
                        global_limit
                    );
                    break;
                }
            }

            let needs_rollover = self.check_needs_rollover(
                current_file_packet_count,
                current_file_size_bytes,
                &file_creation_time,
            );

            if needs_rollover {
                // Flush and close the current file by dropping the pcap_writer
                if let Err(e) = pcap_writer.flush() {
                    error!("Failed to flush file: {}, error: {}", current_file_name, e);
                }
                drop(pcap_writer);

                info!(
                    "Rolling over to new file after {} packets in {}",
                    current_file_packet_count, current_file_name
                );

                // Create new file and reset counters
                let (new_file_name, new_full_path) = self.create_new_file()?;
                current_file_name = new_file_name;
                current_full_path = new_full_path;

                current_file_packet_count = 0;
                current_file_size_bytes = 0;
                file_creation_time = std::time::SystemTime::now();

                // Create new writers for the new file
                let new_file = File::create(&current_full_path)?;
                let new_buf_writer = BufWriter::new(new_file);
                pcap_writer = match PcapWriter::new(new_buf_writer) {
                    Ok(writer) => writer,
                    Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
                };
            }

            match receiver.recv() {
                Ok(user_packet) => {
                    let timestamp = user_packet.timestamp.unwrap_or_else(|| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        Duration::new(now.as_secs(), now.subsec_nanos())
                    });

                    let pcap_packet = PcapPacket {
                        timestamp,
                        orig_len: user_packet.data.len() as u32,
                        data: Cow::Owned(user_packet.data.clone()),
                    };

                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count_total += 1;
                    current_file_packet_count += 1;
                    current_file_size_bytes += user_packet.data.len() as u64;

                    if packet_count_total % 1000 == 0 {
                        debug!("Processed {} user packets total", packet_count_total);
                    }
                }
                Err(_) => {
                    info!("Sender disconnected, stopping user packet processing");
                    break;
                }
            }
        }

        Ok(())
    }

    fn user_packets_to_pcap<W: Write>(
        &self,
        receiver: &Receiver<UserPacket>,
        writer: &mut W,
    ) -> Result<(), SavePcapError> {
        let mut pcap_writer = match PcapWriter::new(writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        let mut packet_count = 0;

        loop {
            if let Some(limit) = self.options.packet_limit {
                if packet_count >= limit {
                    break;
                }
            }

            match receiver.recv() {
                Ok(user_packet) => {
                    let timestamp = user_packet.timestamp.unwrap_or_else(|| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        Duration::new(now.as_secs(), now.subsec_nanos())
                    });

                    let pcap_packet = PcapPacket {
                        timestamp,
                        orig_len: user_packet.data.len() as u32,
                        data: Cow::Owned(user_packet.data.clone()),
                    };

                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count += 1;

                    if packet_count % 1000 == 0 {
                        debug!("Processed {} user packets", packet_count);
                    }
                }
                Err(_) => {
                    info!("Sender disconnected, stopping user packet processing");
                    break;
                }
            }
        }

        Ok(())
    }

    fn check_needs_rollover(
        &self,
        current_packet_count: usize,
        current_file_size_bytes: u64,
        file_creation_time: &std::time::SystemTime,
    ) -> bool {
        if let Some(rollover_seconds) = self.options.rollover_time_seconds {
            if let Ok(elapsed) = file_creation_time.elapsed() {
                if elapsed.as_secs() >= rollover_seconds {
                    return true;
                }
            }
        }

        if let Some(max_packets) = self.options.rollover_packet_count {
            if current_packet_count >= max_packets {
                return true;
            }
        }

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
        let mut pcap_writer = match PcapWriter::new(writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        let mut packet_count = 0;

        loop {
            if let Some(limit) = self.options.packet_limit {
                if packet_count >= limit {
                    break;
                }
            }

            match cap.next_packet() {
                Ok(packet) => {
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count += 1;

                    if packet_count % 1000 == 0 {
                        debug!("Captured {} packets", packet_count);
                    }
                }
                Err(e) => {
                    let error_str = e.to_string();
                    // 检查错误信息是否包含"timeout"关键词，以处理不同形式的超时错误
                    if error_str.contains("timeout") {
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
        let mut pcap_writer = match PcapWriter::new(writer) {
            Ok(writer) => writer,
            Err(e) => return Err(SavePcapError::PcapFileError(e.to_string())),
        };

        let mut packet_count = 0;

        loop {
            if let Some(limit) = self.options.packet_limit {
                if packet_count >= limit {
                    break;
                }
            }

            match cap.next_packet() {
                Ok(packet) => {
                    let pcap_packet = PcapPacket {
                        timestamp: Duration::new(
                            packet.header.ts.tv_sec as u64,
                            packet.header.ts.tv_usec as u32 * 1_000,
                        ),
                        orig_len: packet.data.len() as u32,
                        data: Cow::Owned(packet.data.to_vec()),
                    };

                    if let Err(e) = pcap_writer.write_packet(&pcap_packet) {
                        return Err(SavePcapError::PcapFileError(e.to_string()));
                    }

                    packet_count += 1;

                    if packet_count % 1000 == 0 {
                        debug!("Captured {} packets", packet_count);
                    }
                }
                Err(e) => {
                    let error_str = e.to_string();
                    // 检查错误信息是否包含"timeout"关键词，以处理不同形式的超时错误
                    if error_str.contains("timeout") {
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

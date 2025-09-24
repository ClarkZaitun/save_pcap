# save_pcap

[![中文README](https://img.shields.io/badge/%E4%B8%AD%E6%96%87-README-ff69b4.svg)](README_zh.md)

A cross-platform library written in Rust for capturing network frames from a specified network interface and saving them as pcap or pcapng files, supporting Linux and Windows systems.

## Features

- Cross-platform support (Linux and Windows)
- Capture network frames from a specified network interface
- Support saving as pcap or pcapng format
- Customizable file prefix and path
- Configurable packet capture limit
- Easy-to-use API
- Support for logging output
- Automatic detection of available network devices
- Continuous capture with file rollover functionality
  - Time-based rollover (e.g., create new file every X seconds)
  - Packet count-based rollover (e.g., create new file after X packets)
  - File size-based rollover (e.g., create new file after X MB)
- JSON configuration file support for easy setup
- Support for saving user-provided packets (for testing, simulation, or other non-direct capture scenarios)

## Installation

Add the following to your Cargo.toml file:

```toml
dependencies =
{
    save_pcap = "0.1.0"
}
```

## Usage Examples

### Basic Usage

```rust
use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};

fn main() {
    // Create capture options
    let options = PcapCaptureOptions {
        device_name: "eth0".to_string(), // Linux network interface example, on Windows it might look like "\\Device\\NPF_{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"
        file_prefix: "my_capture".to_string(),
        file_path: "./captures".to_string(),
        file_format: FileFormat::Pcap,
        packet_limit: Some(1000), // Optional, limit to capturing 1000 packets
        snaplen: 65535, // Default capture length
        timeout_ms: 1000, // Default timeout in milliseconds
        continuous_capture: false, // Disable continuous capture
        rollover_time_seconds: None, // No time-based rollover
        rollover_packet_count: None, // No packet count-based rollover
        rollover_file_size_mb: None, // No file size-based rollover
    };

    // Create capturer and start capturing
    let capturer = PcapCapturer::new(options);

    match capturer.capture() {
        Ok(_) => println!("Capture completed successfully"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Using Continuous Capture with File Rollover

This example demonstrates how to use the continuous capture feature with file rollover based on time, packet count, or file size.

```rust
use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer};

fn main() {
    // Create capture options with continuous capture and rollover settings
    let options = PcapCaptureOptions {
        device_name: "eth0".to_string(), // Replace with your network interface
        file_prefix: "continuous_capture".to_string(),
        file_path: "./".to_string(),
        file_format: FileFormat::Pcap,
        packet_limit: None, // No packet limit for continuous capture
        snaplen: 65535,
        timeout_ms: 1000,
        continuous_capture: true, // Enable continuous capture
        rollover_time_seconds: Some(3600), // Create new file every hour
        rollover_packet_count: Some(10000), // Or when 10,000 packets are captured
        rollover_file_size_mb: Some(100), // Or when file size reaches 100 MB
    };

    // Create capturer and start continuous capturing
    let capturer = PcapCapturer::new(options);

    println!("Starting continuous capture. Press Ctrl+C to stop.");
    match capturer.capture() {
        Ok(_) => println!("Capture completed successfully"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Using Command Line Arguments and Configuration Files

This library provides an enhanced example program `configurable_capture` that supports setting capture options through command line arguments or configuration files.

#### Configuring via Command Line Arguments

```bash
# Basic usage
cargo run --example configurable_capture -- --device-name "your_network_device" --file-prefix "my_capture"

# Complete arguments
cargo run --example configurable_capture -- --device-name "your_network_device" --file-prefix "my_capture" --file-path "./captures" --file-format "pcap" --packet-limit 100 --snaplen 65535 --timeout-ms 1000
```

Command line arguments explanation:
- `-d, --device-name`: Network device name (required, you can get available network interface names using `get_available_devices()` function)
- `-p, --file-prefix`: Output file prefix
- `-o, --file-path`: Output file path (default: ./)
- `-f, --file-format`: Output file format (pcap or pcapng, default: pcap)
- `-l, --packet-limit`: Limit on the number of packets to capture
- `-s, --snaplen`: Limit on the size of packets to capture (default: 65535)
- `-t, --timeout-ms`: Capture timeout in milliseconds (default: 1000)
- `-c, --config-file`: Configuration file path

#### Configuring via Configuration File

The library supports JSON configuration files for easy setup. Configuration files can be placed in a custom location or in the recommended `config/` directory at the project root.

1. **Recommended configuration file location**: `config/local_config.json` (this path is ignored by git to prevent sensitive information from being committed)

2. Create a configuration file with the following format:

```json
{
  "device_name": "your_network_device",
  "file_prefix": "capture",
  "file_path": "./captures/",
  "file_format": "pcap",
  "packet_limit": 100,
  "snaplen": 65535,
  "timeout_ms": 1000,
  "continuous_capture": false,
  "rollover_time_seconds": null,
  "rollover_packet_count": null,
  "rollover_file_size_mb": null
}
```

2. Run the program with the configuration file:

```bash
cargo run --example configurable_capture -- --config-file config.json
```

3. Mixing configuration file and command line arguments (command line arguments have higher priority):

```bash
cargo run --example configurable_capture -- --config-file config.json --file-prefix "special_capture" --packet-limit 200
```

3. Run the program with the configuration file:

```bash
cargo run --example configurable_capture -- --config-file config/local_config.json
```

4. Mixing configuration file and command line arguments (command line arguments have higher priority):

```bash
cargo run --example configurable_capture -- --config-file config/local_config.json --file-prefix "special_capture" --packet-limit 200
```

A complete configuration example is provided in the `examples/config_example.json` file in the project root directory.

### Local Configuration File Usage

The library now includes built-in support for loading configuration from a JSON file in `config/local_config.json`. This approach offers several advantages:

- Configuration information is stored separately from code
- Sensitive information (like network device identifiers) is not tracked by git
- Easy to modify settings without changing code

To use this feature:

1. Create a `config` directory at the project root if it doesn't exist
2. Create a `local_config.json` file inside the `config` directory with your configuration
3. The library will automatically load this configuration when running examples like `continuous_capture`

### Example Programs

#### Interactive Example (capture_example.rs)

This example provides an interactive interface that guides you through selecting a network interface, setting file prefix, file path, file format, and packet limit.

```bash
cargo run --example capture_example
```

#### Configurable Example (configurable_capture.rs)

This example supports setting capture options through command line arguments or configuration files, providing a more flexible usage method.

Please refer to the "Using Command Line Arguments and Configuration Files" section above for detailed usage.

#### Test Run Example (test_run.rs)

This example provides a simple test run method, demonstrating the basic functionality and usage of the library.

```bash
cargo run --example test_run
```

## Getting Available Network Devices

```rust
use save_pcap::get_available_devices;

fn main() {
    match get_available_devices() {
        Ok(devices) => {
            println!("Available network devices:");
            for device in devices {
                println!("- {}", device);
            }
        },
        Err(e) => eprintln!("Error: {}", e),
    }
}

// Example output on Linux:
// Available network devices:
// - eth0
// - wlan0
// - lo

// Example output on Windows:
// Available network devices:
// - \\Device\\NPF_{12345678-1234-1234-1234-1234567890AB}
// - \\Device\\NPF_{87654321-4321-4321-4321-BA0987654321}
```

## API Reference

### PcapCaptureOptions

A struct for configuring capture parameters.

```rust
pub struct PcapCaptureOptions {
    pub device_name: String,     // Network interface name
    pub file_prefix: String,     // File name prefix
    pub file_path: String,       // File save path
    pub file_format: FileFormat, // File format (Pcap or PcapNg)
    pub packet_limit: Option<usize>, // Packet limit (optional)
    pub snaplen: i32,            // Capture length
    pub timeout_ms: i32,         // Timeout in milliseconds
    pub continuous_capture: bool, // Enable continuous capture with rollover
    pub rollover_time_seconds: Option<u64>, // Time interval for file rollover (seconds)
    pub rollover_packet_count: Option<usize>, // Packet count for file rollover
    pub rollover_file_size_mb: Option<u64>, // File size for file rollover (MB)
}
```

A default implementation is provided:

```rust
let options = PcapCaptureOptions::default();
// Default values:
// device_name: "",
// file_prefix: "capture",
// file_path: ".",
// file_format: FileFormat::Pcap,
// packet_limit: None,
// snaplen: 65535,
// timeout_ms: 1000,
// continuous_capture: false,
// rollover_time_seconds: None,
// rollover_packet_count: None,
// rollover_file_size_mb: None,
```

### FileFormat

An enum type representing the saved file format.

```rust
#[derive(Debug)]
pub enum FileFormat {
    Pcap,   // pcap format
    PcapNg, // pcapng format
}
```

Note: Currently, both formats will save data in pcap format.

### PcapCapturer

A capturer struct for performing capture operations.

```rust
// Create a capturer
pub fn new(options: PcapCaptureOptions) -> Self

// Start capturing and save to file
pub fn capture(&self) -> Result<(), SavePcapError>

// Start capture process
pub fn start_capture(&mut self) -> Result<(), SavePcapError>

// Get a packet sender for sending user-provided packets
// Only available when packet_source is set to UserProvided
pub fn get_packet_sender(&self) -> Option<Sender<UserPacket>>
```

### UserPacket

A struct representing a user-provided packet that can be saved to a pcap file.

```rust
pub struct UserPacket {
    pub data: Vec<u8>,           // The raw packet data
    pub timestamp: Option<Duration>, // Optional timestamp for the packet
}
```

### PacketSource

An enum representing the source of packets to capture.

```rust
#[derive(Debug, Clone)]
pub enum PacketSource {
    NetworkDevice,  // Capture packets from a network device
    UserProvided    // Use packets provided by the user
}
```

## Using User-Provided Packets

The library supports saving user-provided packets to pcap files, which is useful for testing, simulation, or when you want to save packets that weren't captured directly from a network interface.

### Basic Usage

Here's a step-by-step guide to using this feature:

1. **Configure for User-Provided Packets**
   Set the `packet_source` option to `UserProvided` when creating `PcapCaptureOptions`.

2. **Get the Packet Sender**
   Use the `get_packet_sender()` method to obtain a sender for your packets.

3. **Send Your Packets**
   Create `UserPacket` objects with your packet data and send them through the sender.

4. **Start the Capture Process**
   Call `capture()` to start processing and saving the packets.

### Example

```rust
use save_pcap::{FileFormat, PcapCaptureOptions, PcapCapturer, UserPacket};
use std::thread;
use std::time::Duration;

fn main() {
    // Configure capture options for user-provided packets
    let options = PcapCaptureOptions {
        packet_source: save_pcap::PacketSource::UserProvided,
        file_prefix: "user_packets".to_string(),
        file_path: ".".to_string(),
        file_format: FileFormat::Pcap,
        snaplen: 65535,
        timeout_ms: 1000,
        // Additional options as needed
        ..Default::default()
    };

    // Create the capturer
    let capturer = PcapCapturer::new(options);

    // Get the packet sender
    let packet_sender = match capturer.get_packet_sender() {
        Some(sender) => sender,
        None => {
            eprintln!("Failed to get packet sender");
            return;
        }
    };

    // Start a thread to provide packets
    let sender_thread = thread::spawn(move || {
        // Send sample packets
        for i in 0..10 {
            // Create your packet data
            let packet_data = vec![/* your packet bytes here */];

            // Create a user packet
            let user_packet = UserPacket {
                data: packet_data,
                timestamp: None, // Use current time, or provide your own
            };

            // Send the packet
            if let Err(err) = packet_sender.send(user_packet) {
                eprintln!("Failed to send packet: {:?}", err);
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // Start capturing
    if let Err(err) = capturer.capture() {
        eprintln!("Capture error: {:?}", err);
    }

    // Wait for the sender thread to complete
    if let Err(err) = sender_thread.join() {
        eprintln!("Sender thread error: {:?}", err);
    }
}
```

### Advanced Example: Continuous Capture with Rollover

You can also use user-provided packets with continuous capture and file rollover functionality:

```rust
let options = PcapCaptureOptions {
    packet_source: save_pcap::PacketSource::UserProvided,
    file_prefix: "user_packets".to_string(),
    file_path: ".".to_string(),
    file_format: FileFormat::Pcap,
    continuous_capture: true,
    rollover_packet_count: Some(20), // Rollover after 20 packets
    packet_limit: Some(100),         // Stop after 100 packets
    // Other options as needed
    ..Default::default()
};

// Rest of the code remains the same as the basic example
```

## Error Handling

The library defines the `SavePcapError` enum type for handling various possible error situations:

```rust
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
```

## Notes

1. On Windows systems, you may need to install WinPcap or Npcap drivers to use this library properly.
2. On Linux systems, you may need to install the libpcap-dev package.
3. Capturing network frames usually requires administrator/root privileges.
4. Capturing a large number of packets for a long time may consume significant system resources and disk space.
5. Network interface naming conventions differ across operating systems:
   - Linux: Usually eth0, wlan0, lo, etc.
   - Windows: Usually in the form of GUID like "\\Device\\NPF_{XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}"
6. Please ensure that the specified network interface name exists and that you have sufficient permissions to access it.
7. It is recommended to use the `get_available_devices()` function to obtain a list of available network interfaces on the current system.

## Dependencies

- [pcap](https://crates.io/crates/pcap) - For capturing network packets
- [pcap-file](https://crates.io/crates/pcap-file) - For writing pcap/pcapng files
- [anyhow](https://crates.io/crates/anyhow) - For error handling
- [thiserror](https://crates.io/crates/thiserror) - For defining custom error types
- [log](https://crates.io/crates/log) and [env_logger](https://crates.io/crates/env_logger) - For logging output
- [chrono](https://crates.io/crates/chrono) - For handling timestamps

## License

MIT

# BoxMonitor

A network monitoring tool with a terminal user interface (TUI) for monitoring server connectivity via ICMP ping and SSH.

## Features

- **ICMP Ping Monitoring**: Monitor network connectivity to targets
- **SSH Connection Testing**: Test SSH connectivity and authentication
- **Terminal UI**: Real-time monitoring with charts and status displays
- **Multiple Input Formats**: Support for JSON config or simple IP lists
- **Command Line Arguments**: Quick monitoring setup via CLI

## Requirements

- Root privileges (required for ICMP ping)
- Rust toolchain for building

## Usage

### Basic Usage
```bash
sudo ./boxmonitor
```

### Command Line Options
```bash
# Monitor specific IPs
sudo ./boxmonitor --ip "192.168.1.1,192.168.1.2"

# Monitor SSH targets
sudo ./boxmonitor --ssh "user@192.168.1.1:22,admin@192.168.1.2"

# Use simple IP list format
sudo ./boxmonitor --simple

# Show current configuration
sudo ./boxmonitor --config
```

## Configuration

### Simple List Format
Create `~/.config/box/.iplist` with one IP per line:
```
192.168.1.1
192.168.1.2
10.0.0.1
```

### JSON Configuration
For advanced configuration with SSH targets and custom settings.

## Building

```bash
cargo build --release
```

## Dependencies

- ratatui - Terminal UI framework
- tokio - Async runtime
- surge-ping - ICMP ping implementation
- ssh2 - SSH client functionality
- crossterm - Terminal handling

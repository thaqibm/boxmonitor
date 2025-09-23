# BoxMonitor

A terminal-based network monitoring tool focused on ICMP connectivity checks, now with built-in DNS resolution so you can point it at IPs or domain names directly.

## Features

- **ICMP Ping Monitoring**: Track network connectivity and latency trends
- **Domain Support**: Monitor hostnames directly with automatic DNS lookups
- **Terminal UI**: Real-time monitoring with charts and status displays
- **Multiple Input Formats**: Support for JSON config or simple host lists
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
# Monitor specific hosts (IP addresses or domain names)
sudo ./boxmonitor --ip "192.168.1.1,example.com"

# Use simple host list format
sudo ./boxmonitor --simple

# Show current configuration
sudo ./boxmonitor --config

# Debug ping behaviour and view raw failures
sudo ./boxmonitor --debug --ip "example.com"
```

## Configuration

### Simple List Format
Create `~/.config/box/.iplist` with one host per line:
```
192.168.1.1
example.com
api.internal.local
```

### JSON Configuration
For advanced configuration with custom names, ping intervals, and history sizing.

## Building

```bash
cargo build --release
```

## Dependencies

- ratatui - Terminal UI framework
- tokio - Async runtime
- surge-ping - ICMP ping implementation
- crossterm - Terminal handling

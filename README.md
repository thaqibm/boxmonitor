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

## Browser demo

The browser demo compiles the **same Rust statistics and Ratatui widgets** to
WebAssembly. Its white page and command sidebar follow the OS browser demo,
with a full-color terminal. It runs entirely on a static host, including
GitHub Pages, without QEMU, a backend, or cross-origin isolation headers.

Browsers cannot send raw ICMP packets. This demo uses deterministic synthetic
samples for four documentation-only IP addresses. Healthy traffic, latency /
packet loss, and an API outage exercise the real charts and failure views.
The initial window contains 60 samples; one new sample arrives per second and
latency history retains 100 samples. Failure logs independently retain the
last 100 failures per target, matching the native app. Scenario selection adds
one sample immediately, including while paused. Reset restores the initial
healthy window. No real hosts are contacted by the monitor.

### Build and run

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.104 --locked
./scripts/build-web.sh
node web/server.mjs
```

Open <http://127.0.0.1:8091/boxmonitor/> and select **Start demo**.
Use the sidebar buttons or focus the terminal and press Left / Right to switch
targets, **P** to cycle plots, and **Space** to pause. Tab retains normal browser
focus navigation. On small screens the terminal scrolls horizontally.

The generated static files are in `build/site`. All asset URLs are relative
so the demo also works under a GitHub Pages repository prefix.

### Verify

```bash
cargo test --locked
cargo check --locked --bin boxmonitor
npm --prefix web ci
cd web
npx playwright install chromium firefox
npm test
```

The browser tests load the actual Wasm binary in Chromium and Firefox, check
colored output, target / plot navigation, failure scenarios, pause / reset,
mobile layout, and retry after a failed Wasm download.

### GitHub Pages

The `Browser demo` workflow builds and tests on GitHub-hosted Ubuntu runners.
Pull requests validate only; successful pushes to `main` or manual runs on
`main` deploy `build/site`. In repository **Settings → Pages**, select
**GitHub Actions** as the source before the first deployment.

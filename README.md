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

[Open the browser demo](https://thaqibm.github.io/boxmonitor/).

The same Rust statistics and Ratatui widgets compile to WebAssembly. The page
opens automatically, with a colored monitor, two navigation buttons, and one
input. Use **< / >**, arrow keys, or **h / l** to switch targets; **p** cycles
plots. Keyboard shortcuts do not intercept typing in the input.

Enter `Name = example.com`, `Name = https://example.com/path`, or a bare host /
HTTP(S) URL, then press Enter to add it. URLs are reduced to their hostname.
Names accept 1–40 printable ASCII characters. Duplicate hosts are rejected;
up to 16 total targets are supported, including the four initial examples.
Added targets start collecting immediately and exist until the page reloads.
On small screens the terminal scrolls horizontally.

**All traffic is simulated, including added hosts.** Browsers cannot send raw
ICMP packets. This static demo makes no monitoring requests to entered hosts.
It starts with 60 synthetic samples and adds one sample per second, retaining
100 latency samples per target. The native app performs real ICMP monitoring.

### Build and run

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.104 --locked
./scripts/build-web.sh
node web/server.mjs
```

Open <http://127.0.0.1:8091/boxmonitor/>. Generated files are in `build/site`;
relative asset URLs support GitHub Pages repository prefixes.

### Verify

```bash
cargo test --locked
cargo check --locked --bin boxmonitor
npm --prefix web ci
cd web
npx playwright install chromium firefox
npm test
```

Browser checks cover automatic Wasm startup, color, navigation, named host
addition, URL parsing, validation, continued updates, mobile layout, and
recovery from a failed Wasm download in Chromium and Firefox.

### GitHub Pages

The `Browser demo` workflow builds and tests on GitHub-hosted Ubuntu runners.
Successful pushes to `main` deploy `build/site` through GitHub Actions.

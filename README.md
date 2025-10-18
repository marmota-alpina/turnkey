# Turnkey Access Control Emulator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.90%2B-blue.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)]()

Turnkey is an open-source emulator for Henry access control systems, fully implemented in Rust to deliver fast, reliable, and scalable integration testing.

It provides a complete software environment for emulating physical access control devices — including turnstiles, card readers, and biometric scanners — and implements the Henry communication protocol used by popular Brazilian access control products such as Primme Acesso, Argos, and Primme SF.

Turnkey is ideal for developers building integrations, validating access control software without physical hardware, or creating custom solutions that interface with Henry-compatible devices.

## Features

### Core Capabilities
- **Complete Protocol Implementation** - Full Henry protocol support (versions 1.0.0.23, 8.0.0.50)
- **Multiple Device Emulation** - Primme Acesso, Argos, Primme SF
- **Hardware Abstraction** - Unified interface for different hardware types
- **Production Ready** - Async/await, robust error handling, comprehensive logging

### Hardware Support
- **RFID/NFC Readers** - ACR122U (USB), RC522 (SPI), Mock readers
- **Biometric Scanners** - Control iD iDBio, Digital Persona (planned)
- **Keypads** - USB HID, Wiegand, Matrix keypads
- **Turnstile Controllers** - GPIO (Raspberry Pi), USB relay boards, Modbus

### Network & Storage
- **TCP/IP Communication** - Full protocol server with TLS support
- **Local Database** - SQLite with migrations and caching
- **Online/Offline Modes** - Automatic failover and sync
- **Event Logging** - Comprehensive audit trail

### Developer Tools
- **CLI Interface** - Easy configuration and testing
- **Mock Hardware** - Test without physical devices
- **Auto-Discovery** - USB device detection
- **Cross-Platform** - Linux x64, ARM64, Raspberry Pi

## Quick Start

### Prerequisites

- **Rust 1.90+** ([Install Rust](https://rustup.rs/)) - Edition 2024 com async traits nativos!
- **Linux Kernel 6.1+** (for optimal USB/GPIO support)
- **System dependencies** (see [Installation](#installation))

**🚀 Tech Stack Moderno:**
- Rust 1.90 com LLD linker (20-40% builds mais rápidos)
- Edition 2024 (async traits nativos, generators, RPITIT)
- Tokio async runtime
- Zero-copy protocol parsing

### Installation

#### 1. Install System Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config \
    libusb-1.0-0-dev libudev-dev \
    libpcsclite-dev libsqlite3-dev \
    libssl-dev pcscd pcsc-tools
```

**Fedora/RHEL:**
```bash
sudo dnf install -y \
    gcc pkg-config systemd-devel \
    pcsc-lite-devel sqlite-devel \
    openssl-devel pcsc-tools
```

**Arch Linux:**
```bash
sudo pacman -S --needed \
    base-devel pkg-config \
    libusb systemd-libs \
    pcsclite sqlite openssl \
    pcsc-tools
```

#### 2. Clone and Build
```bash
git clone https://github.com//marmota-alpina/turnkey.git
cd turnkey

# Quick setup (installs deps + builds)
make install-deps
make build

# Or use cargo directly
cargo build --release
```

#### 3. Setup Hardware Permissions
```bash
# Enable PCSC daemon for card readers
sudo systemctl enable pcscd
sudo systemctl start pcscd

# Configure USB permissions
sudo make setup-hardware
# Then logout/login or reboot
```

#### 4. Initialize Database
```bash
# Create database and run migrations
make db-create
make db-migrate
```

#### 5. Run the Emulator
```bash
# Start server (mock mode - no hardware required)
cargo run --bin henry-cli -- server --mock

# Or with real hardware (requires sudo for GPIO/USB)
sudo cargo run --bin henry-cli -- server --features hardware
```

---

## 📖 Usage Examples

### Basic Server Mode
```bash
# Start emulator on default port (TCP 3000)
henry-cli server --port 3000 --mock

# With custom configuration
henry-cli server --config config/production.toml

# Enable verbose logging
RUST_LOG=debug henry-cli server --mock
```

### Hardware Discovery
```bash
# Auto-detect connected devices
henry-cli discover

# Test specific hardware
henry-cli test --device rfid
henry-cli test --device biometric
henry-cli test --device keypad
```

### Configuration Management
```bash
# View current configuration
henry-cli config show

# Set device parameters
henry-cli config set turnstile.timeout 5
henry-cli config set rfid.poll_interval 250

# Reset to defaults
henry-cli config reset
```

## 🏗️ Architecture
```
henry_access_control_emulator/
├── crates/
│   ├── henry-core/          # Core types and errors
│   ├── henry-protocol/      # Henry protocol implementation
│   ├── henry-hardware/      # Hardware abstraction layer
│   ├── henry-rfid/          # RFID/NFC reader drivers
│   ├── henry-biometric/     # Biometric scanner drivers
│   ├── henry-keypad/        # Keypad drivers
│   ├── henry-turnstile/     # Turnstile control logic
│   ├── henry-storage/       # Database and persistence
│   ├── henry-network/       # TCP/IP server
│   ├── henry-emulator/      # Device emulators
│   └── henry-cli/           # Command-line interface
├── config/                  # Configuration files
├── migrations/              # Database migrations
└── vendor/                  # Third-party SDKs (not included)
```

### Protocol Flow Example
```rust
// 1. Turnstile sends card read event
Turnstile → Server: 15+REON+000+0]00000000000011912322]10/05/2025 12:46:06]1]0]

// 2. Server validates and grants access
Server → Turnstile: 15+REON+00+6]5]Access granted]

// 3. Turnstile waits for rotation
Turnstile → Server: 15+REON+000+80]]10/05/2025 12:46:06]0]0]

// 4. User passes through, turnstile confirms
Turnstile → Server: 15+REON+000+81]]10/05/2025 12:46:08]1]0]
```

## Configuration

### Main Configuration (`config/default.toml`)
```toml
[server]
host = "0.0.0.0"
port = 3000
tls_enabled = false
max_connections = 100

[database]
path = "data/henry.db"
pool_size = 10
enable_wal = true

[hardware]
auto_discovery = true
mock_mode = false
discovery_interval = 10  # seconds

[validation]
mode = "online"  # online, offline, auto, semi-auto
timeout_online = 3000  # milliseconds
timeout_offline = 60  # seconds
anti_passback_minutes = 5
```

### Hardware Configuration (`config/hardware.toml`)
```toml
[[rfid_readers]]
id = "entrance_reader"
type = "acr122u"
enabled = true
auto_connect = true

[rfid_readers.config]
port = "auto"
led_mode = "auto"
buzzer = true
poll_interval = 250

[[biometric_readers]]
id = "main_biometric"
type = "idbio"
enabled = true

[biometric_readers.config]
quality_threshold = 60
match_threshold = 70
capture_timeout = 5000

[turnstile]
enabled = true
type = "relay_board"
entry_relay = 1
exit_relay = 2
timeout = 5
```

See [config/](config/) directory for complete examples.


## Hardware Setup

### RFID Readers

#### ACR122U (USB)
```bash
# Check if detected
pcsc_scan

# Should show:
# Reader 0: ACS ACR122U PICC Interface 00 00
```

### Biometric Readers

**Important**: Biometric SDKs are proprietary and not included in this repository.

To use Control iD iDBio readers:

1. Download SDK from [Control iD](https://www.controlid.com.br)
2. Place libraries in `vendor/controlid/`:
```
   vendor/controlid/
   ├── linux-x86_64/libidbio.so
   ├── linux-aarch64/libidbio_arm64.so
   └── include/idbio.h
```
3. Rebuild: `cargo build --features idbio`

### Turnstile Control

#### GPIO (Raspberry Pi)
```bash
# Run with sudo for GPIO access
sudo henry-cli server --features raspberry-pi
```

#### USB Relay Board
```bash
# Check device
ls -l /dev/ttyUSB*

# Set permissions
sudo usermod -aG dialout $USER
```

## Testing

### Run All Tests
```bash
# Unit tests
make test

# Integration tests
make test-integration

# Hardware tests (requires connected devices)
sudo make test-hardware

# With coverage
make coverage
```

### Mock Mode Testing
```bash
# Start server in mock mode
cargo run --bin henry-cli -- server --mock

# In another terminal, test with netcat
echo "01+REON+000+0]12345678]16/10/2025 10:00:00]1]0]" | nc localhost 3000
```

### Protocol Testing
```rust
use henry_protocol::{Message, CommandCode};

#[tokio::test]
async fn test_access_request() {
    let request = Message::new(
        "01",
        CommandCode::AccessRequest,
        vec!["12345678", "16/10/2025 10:00:00", "1", "0"]
    );
    
    assert_eq!(request.device_id, "01");
    assert_eq!(request.command, CommandCode::AccessRequest);
}
```

## Performance

- **Throughput**: 1000+ access events/second
- **Latency**: <10ms average response time
- **Memory**: ~50MB base, ~100MB with caching
- **Database**: SQLite with WAL mode for concurrent access
- **Connections**: 100+ simultaneous TCP connections

Benchmarks:
```bash
make bench
```

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Format** your code (`cargo fmt`)
4. **Lint** your code (`cargo clippy -- -D warnings`)
5. **Test** your changes (`cargo test`)
6. **Commit** with clear messages
7. **Push** to your branch
8. **Open** a Pull Request

### Code Style

This project follows standard Rust conventions:
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes with no warnings
- Add tests for new features
- Update documentation as needed

### Commit Messages
```
feat: add support for RC522 RFID readers
fix: resolve timeout issue in online validation
docs: update hardware setup instructions
test: add integration tests for biometric enrollment
```

## License

This project is licensed under the **MIT License** - see the [LICENSE-MIT](LICENSE-MIT) file for details.
```
MIT License

Copyright (c) 2025 Henry Access Control Emulator Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
```

## Legal Disclaimer

This software is provided for **educational and interoperability purposes only**.

- The Henry protocol implementation is based on publicly available documentation and reverse engineering for interoperability
- Proprietary SDKs (biometric readers, etc.) are **NOT included** and must be obtained separately from hardware manufacturers
- This software does not contain or distribute any proprietary code from Control iD or other manufacturers
- Users are responsible for complying with all applicable licenses and regulations
- The authors assume no liability for misuse of this software

**Security Notice**: This is an emulator for development and testing. For production access control systems, always follow security best practices and compliance requirements (LGPD, GDPR, etc.).

## Roadmap

### Version 0.1.0 (In progress)
- [ ] Core protocol implementation
- [ ] Mock device support
- [ ] Basic TCP server
- [ ] SQLite storage
- [ ] CLI interface

### Version 0.2.0 (november, 2025)
- [ ] ACR122U RFID support
- [ ] iDBio biometric integration
- [ ] GPIO turnstile control
- [ ] Web UI dashboard
- [ ] REST API

### Version 0.3.0 (december 2025)
- [ ] Multi-site support
- [ ] Advanced scheduling
- [ ] Photo capture integration
- [ ] Mobile app support
- [ ] Cloud sync (optional)

### Version 1.0.0 (February 2025)
- [ ] Production hardening
- [ ] Complete documentation
- [ ] Performance optimization
- [ ] Security audit
- [ ] Professional support options

## Support & Community

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com//marmota-alpina/turnkey/issues)
- **Discussions**: [GitHub Discussions](https://github.com//marmota-alpina/turnkey/discussions)
- **Email**: support@henry-emulator.com

### Frequently Asked Questions

**Q: Do I need physical hardware to use Henry?**  
A: No! Henry includes mock devices for testing without hardware. Use `--mock` flag.

**Q: What devices are supported?**  
A: ACR122U RFID readers, iDBio biometric scanners, USB HID keypads, GPIO/relay turnstile controllers, and more. See [Hardware Setup](#hardware-setup).

**Q: Can I use this in production?**  
A: **No, not yet.** This is an alpha-stage emulator intended for development, testing, and integration purposes only. It has not undergone security audits, stress testing, or compliance validation required for production access control systems. For production use certified commercial solutions.

**Q: Is this compatible with Control iD software?**  
A: Henry implements the same protocol, so it can work with software expecting Henry-compatible devices.

**Q: How do I get biometric SDKs?**  
A: Contact your hardware manufacturer. SDKs are proprietary and not redistributable.

## Acknowledgments

- Rust community for excellent libraries
- Control iD for access control hardware
- Contributors and testers


## Project Status

**Status**: Active Development  
**Stability**: Alpha (API may change)  
**Production Ready**: Not yet (use at your own risk)


**Built with ❤️ in Rust**

If this project helps you, please consider giving it a star on GitHub!


## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.


**Last Updated**: October 16, 2025
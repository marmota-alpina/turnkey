# Complete Architecture - Turnkey Access Control Emulator

## 1. Directory Structure and Project Organization

```
turnkey/
├── Cargo.toml                          # Workspace root
├── Cargo.lock
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .gitignore
├── .env.example
├── rust-toolchain.toml                 # Pinned Rust version (1.90+)
├── deny.toml                           # Cargo deny for security audit
├── Cross.toml                          # Cross-compilation config
├── Makefile                            # Build automation
├── build.rs                            # Main build script
│
├── .cargo/
│   └── config.toml                    # Local cargo configuration
│
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # CI/CD pipeline
│   │   ├── security-audit.yml         # Security auditing
│   │   └── release.yml                # Release automation
│   └── dependabot.yml                 # Automated dependency updates
│
├── crates/                             # Workspace members
│   ├── turnkey-core/                  # Core emulator functionality
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs               # Centralized error handling
│   │       ├── types.rs               # Shared types
│   │       └── constants.rs           # System constants
│   │
│   ├── turnkey-protocol/              # Henry protocol implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── message.rs             # Message structures
│   │       ├── parser.rs              # Message parser
│   │       ├── builder.rs             # Message builder
│   │       ├── codec.rs               # Tokio codec
│   │       ├── checksum.rs            # Checksum calculation
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── access.rs          # Access commands
│   │           ├── config.rs          # Configuration commands
│   │           └── management.rs      # Management commands
│   │
│   ├── turnkey-hardware/              # Hardware abstraction
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # SDK build script
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Base traits
│   │       ├── manager.rs             # Hardware manager
│   │       ├── discovery.rs           # USB auto-discovery
│   │       └── events.rs              # Event system
│   │
│   ├── turnkey-rfid/                  # RFID/NFC readers
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # CardReader trait
│   │       ├── acr122u/
│   │       │   ├── mod.rs
│   │       │   ├── driver.rs          # ACR122U driver
│   │       │   ├── commands.rs        # APDU commands
│   │       │   └── monitor.rs         # Card monitoring
│   │       ├── rc522/
│   │       │   └── driver.rs          # RC522 support (SPI)
│   │       └── mock/
│   │           └── mock_reader.rs     # Mock for testing
│   │
│   ├── turnkey-biometric/             # Biometric readers
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # iDBio SDK build
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # BiometricReader trait
│   │       ├── idbio/
│   │       │   ├── mod.rs
│   │       │   ├── driver.rs          # iDBio driver
│   │       │   ├── sdk.rs             # FFI bindings
│   │       │   └── protocol.rs        # iDBio protocol
│   │       ├── digital_persona/       # Future support
│   │       │   └── driver.rs
│   │       ├── template_manager.rs    # Template management
│   │       └── mock/
│   │           └── mock_biometric.rs  # Mock for testing
│   │
│   ├── turnkey-keypad/                # Numeric keypads
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Keypad trait
│   │       ├── usb_hid/
│   │       │   └── driver.rs          # USB HID keypads
│   │       ├── matrix/
│   │       │   └── driver.rs          # GPIO matrix keypad
│   │       ├── wiegand/
│   │       │   └── driver.rs          # Wiegand keypads
│   │       └── mock/
│   │           └── mock_keypad.rs     # Mock for testing
│   │
│   ├── turnkey-turnstile/             # Turnstile control
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── controller.rs          # Main controller
│   │       ├── gpio/
│   │       │   └── raspberry_pi.rs    # Raspberry Pi GPIO
│   │       ├── relay/
│   │       │   ├── usb_relay.rs       # USB relay boards
│   │       │   └── modbus.rs          # Modbus relays
│   │       ├── sensors/
│   │       │   ├── rotation.rs        # Rotation sensor
│   │       │   └── position.rs        # Position sensor
│   │       └── state_machine.rs       # State machine
│   │
│   ├── turnkey-storage/               # Persistence layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database.rs            # Database abstraction
│   │       ├── sqlite/
│   │       │   ├── mod.rs
│   │       │   ├── connection.rs      # Connection pool
│   │       │   └── migrations.rs      # Migration system
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── user.rs            # User model
│   │       │   ├── card.rs            # Card model
│   │       │   ├── access_log.rs      # Access logs
│   │       │   └── device_state.rs    # Device state
│   │       ├── repository/
│   │       │   ├── mod.rs
│   │       │   ├── user_repo.rs       # User repository
│   │       │   ├── card_repo.rs       # Card repository
│   │       │   └── log_repo.rs        # Log repository
│   │       └── cache/
│   │           ├── mod.rs
│   │           └── memory.rs          # In-memory cache
│   │
│   ├── turnkey-network/               # Network layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs              # TCP server
│   │       ├── connection.rs          # Connection management
│   │       ├── tls.rs                 # TLS support
│   │       └── protocol_handler.rs    # Protocol handler
│   │
│   ├── turnkey-emulator/              # Device emulators
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── device_trait.rs        # Device trait
│   │       ├── primme_acesso/
│   │       │   ├── mod.rs
│   │       │   ├── device.rs          # Primme emulator
│   │       │   └── features.rs        # Specific features
│   │       ├── argos/
│   │       │   └── device.rs          # Argos emulator
│   │       ├── primme_sf/
│   │       │   └── device.rs          # Primme SF emulator
│   │       └── bridge/
│   │           ├── mod.rs
│   │           └── hardware_bridge.rs # Hardware->Protocol bridge
│   │
│   └── turnkey-cli/                   # CLI application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── server.rs          # Server command
│           │   ├── test.rs            # Test command
│           │   └── config.rs          # Config command
│           └── ui/
│               ├── mod.rs
│               └── terminal.rs        # TUI interface
│
├── vendor/                             # Third-party SDKs
│   ├── controlid/
│   │   ├── linux-x86_64/
│   │   │   └── libidbio.so           # iDBio SDK x64
│   │   ├── linux-aarch64/
│   │   │   └── libidbio_arm64.so     # iDBio SDK ARM64
│   │   └── include/
│   │       └── idbio.h               # Headers
│   └── README.md                      # SDK instructions
│
├── config/                             # Configuration files
│   ├── default.toml                   # Default config
│   ├── development.toml               # Development config
│   ├── production.toml                # Production config
│   ├── hardware.toml                  # Hardware config
│   └── logging.toml                   # Logging config
│
├── migrations/                         # SQLite migrations
│   ├── 001_initial_schema.sql
│   ├── 002_add_users.sql
│   ├── 003_add_cards.sql
│   ├── 004_add_biometrics.sql
│   ├── 005_add_access_logs.sql
│   └── 006_add_device_states.sql
│
├── scripts/                            # Helper scripts
│   ├── install-deps.sh                # Install dependencies
│   ├── setup-hardware.sh              # Hardware setup
│   ├── generate-keys.sh               # Generate TLS keys
│   └── cross-compile.sh               # Cross-compilation
│
├── tests/                              # Integration tests
│   ├── common/
│   │   └── mod.rs                     # Test helpers
│   ├── integration/
│   │   ├── protocol_test.rs
│   │   ├── hardware_test.rs
│   │   └── e2e_test.rs
│   └── fixtures/
│       ├── test_data.sql
│       └── mock_devices.toml
│
├── benches/                            # Benchmarks
│   ├── protocol_bench.rs
│   ├── database_bench.rs
│   └── throughput_bench.rs
│
├── docs/                               # Documentation
│   ├── architecture.md
│   ├── hardware-setup.md
│   ├── api-reference.md
│   └── troubleshooting.md
│
└── examples/                           # Usage examples
    ├── basic_server.rs
    ├── hardware_discovery.rs
    ├── biometric_enrollment.rs
    └── stress_test.rs
```

## 2. Main Cargo.toml (Workspace Root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/turnkey-core",
    "crates/turnkey-protocol",
    "crates/turnkey-hardware",
    "crates/turnkey-rfid",
    "crates/turnkey-biometric",
    "crates/turnkey-keypad",
    "crates/turnkey-turnstile",
    "crates/turnkey-storage",
    "crates/turnkey-network",
    "crates/turnkey-emulator",
    "crates/turnkey-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.90"
authors = ["Turnkey Team <team@turnkey-emulator.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/marmota-alpina/turnkey"

[workspace.dependencies]
# Async Runtime - Tokio is the default choice for async in Rust
tokio = { version = "1.40", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec", "net"] }
async-trait = "0.1"

# Serialization - Serde is the de facto standard
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_repr = "0.1"
bincode = "1.3"

# Error Handling - thiserror for typed errors, anyhow for applications
thiserror = "1.0"
anyhow = "1.0"

# Logging - tracing is more modern and structured than log
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"

# Database - SQLx with SQLite for simplicity and performance
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }

# Configuration - config-rs is flexible and well maintained
config = { version = "0.14", features = ["toml"] }
toml = "0.8"

# Date/Time - chrono is the standard
chrono = { version = "0.4", features = ["serde"] }

# Hardware/USB - rusb for USB, serialport for serial
rusb = "0.9"
serialport = "4.5"
hidapi = "2.6"

# Smart Card - pcsc for card readers
pcsc = "2.8"

# GPIO - rppal for Raspberry Pi
rppal = { version = "0.19", optional = true }

# Networking
bytes = "1.7"
futures = "0.3"

# Utilities
uuid = { version = "1.10", features = ["v4", "serde"] }
dashmap = "6.1"
parking_lot = "0.12"
crossbeam-channel = "0.5"

# Testing
mockall = "0.13"
rstest = "0.22"
tempfile = "3.12"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"

[profile.dev]
opt-level = 0
debug = true

[profile.bench]
inherits = "release"
```

## 3. Crate turnkey-core (crates/turnkey-core/Cargo.toml)

```toml
[package]
name = "turnkey-core"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror.workspace = true
serde.workspace = true
chrono.workspace = true
uuid.workspace = true

[lib]
name = "turnkey_core"
path = "src/lib.rs"
```

### crates/turnkey-core/src/lib.rs

```rust
pub mod error;
pub mod types;
pub mod constants;

pub use error::{Error, Result};
pub use types::*;
pub use constants::*;

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIME: &str = env!("VERGEN_BUILD_TIMESTAMP");
```

### crates/turnkey-core/src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Hardware error: {0}")]
    Hardware(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid state transition")]
    InvalidStateTransition,
}

pub type Result<T> = std::result::Result<T, Error>;
```

## 4. Crate turnkey-hardware (crates/turnkey-hardware/Cargo.toml)

```toml
[package]
name = "turnkey-hardware"
version.workspace = true
edition.workspace = true

[dependencies]
turnkey-core = { path = "../turnkey-core" }
async-trait.workspace = true
tokio.workspace = true
tracing.workspace = true
serde.workspace = true
dashmap.workspace = true
rusb.workspace = true
uuid.workspace = true

[features]
mock = []
```

### crates/turnkey-hardware/src/traits.rs

```rust
use async_trait::async_trait;
use turnkey_core::Result;
use serde::{Serialize, Deserialize};
use std::any::Any;

/// Base trait for all hardware devices
#[async_trait]
pub trait HardwareDevice: Send + Sync + Any {
    /// Unique device identifier
    fn device_id(&self) -> &str;

    /// Device type
    fn device_type(&self) -> DeviceType;

    /// Connect to device
    async fn connect(&mut self) -> Result<()>;

    /// Disconnect from device
    async fn disconnect(&mut self) -> Result<()>;

    /// Check if connected
    async fn is_connected(&self) -> bool;

    /// Get device info
    async fn get_info(&self) -> DeviceInfo;

    /// Reset device
    async fn reset(&mut self) -> Result<()>;

    /// For downcasting
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceType {
    RfidReader,
    BiometricReader,
    Keypad,
    TurnstileController,
    RelayBoard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,
    pub firmware_version: String,
    pub capabilities: Vec<String>,
}
```

## 5. Crate turnkey-rfid (crates/turnkey-rfid/Cargo.toml)

```toml
[package]
name = "turnkey-rfid"
version.workspace = true
edition.workspace = true

[dependencies]
turnkey-core = { path = "../turnkey-core" }
turnkey-hardware = { path = "../turnkey-hardware" }
async-trait.workspace = true
tokio.workspace = true
tracing.workspace = true
pcsc.workspace = true
bytes.workspace = true

[features]
acr122u = ["pcsc"]
rc522 = []
mock = []
```

## 6. Crate turnkey-biometric (crates/turnkey-biometric/Cargo.toml)

```toml
[package]
name = "turnkey-biometric"
version.workspace = true
edition.workspace = true

[dependencies]
turnkey-core = { path = "../turnkey-core" }
turnkey-hardware = { path = "../turnkey-hardware" }
async-trait.workspace = true
tokio.workspace = true
tracing.workspace = true
libloading = "0.8"
base64 = "0.22"

[build-dependencies]
bindgen = "0.70"
cc = "1.1"

[features]
idbio = []
digital-persona = []
mock = []
```

## 7. Crate turnkey-keypad (crates/turnkey-keypad/Cargo.toml)

```toml
[package]
name = "turnkey-keypad"
version.workspace = true
edition.workspace = true

[dependencies]
turnkey-core = { path = "../turnkey-core" }
turnkey-hardware = { path = "../turnkey-hardware" }
async-trait.workspace = true
tokio.workspace = true
tracing.workspace = true
hidapi.workspace = true
serialport.workspace = true

[features]
usb-hid = ["hidapi"]
wiegand = []
matrix = []
mock = []
```

## 8. Crate turnkey-storage (crates/turnkey-storage/Cargo.toml)

```toml
[package]
name = "turnkey-storage"
version.workspace = true
edition.workspace = true

[dependencies]
turnkey-core = { path = "../turnkey-core" }
sqlx.workspace = true
tokio.workspace = true
tracing.workspace = true
serde.workspace = true
chrono.workspace = true
dashmap.workspace = true
uuid.workspace = true

[features]
sqlite = ["sqlx/sqlite"]
postgres = ["sqlx/postgres"]
mysql = ["sqlx/mysql"]
```

## 9. Hardware Configuration (config/hardware.toml)

```toml
# Turnkey Hardware Configuration
# Supports multiple simultaneous devices

[general]
auto_discovery = true
discovery_interval = 10  # seconds
mock_mode = false  # true for testing without hardware

# === RFID/NFC Readers ===
[[rfid_readers]]
id = "entrance_reader"
type = "acr122u"
enabled = true
auto_connect = true

[rfid_readers.config]
port = "auto"  # auto-detect or specify USB port
led_mode = "auto"
buzzer = true
poll_interval = 250  # ms

[rfid_readers.mifare]
default_key_a = "FFFFFFFFFFFF"
default_key_b = "FFFFFFFFFFFF"
sector = 1
block = 4

[[rfid_readers]]
id = "exit_reader"
type = "rc522"
enabled = false

[rfid_readers.config]
spi_bus = 0
spi_device = 0
reset_pin = 25
irq_pin = 24

# === Biometric Readers ===
[[biometric_readers]]
id = "main_biometric"
type = "idbio"
enabled = true
auto_connect = true

[biometric_readers.config]
device_index = 0
capture_timeout = 5000  # ms
quality_threshold = 60
match_threshold = 70
auto_enroll = false

[biometric_readers.led]
capture = "blue"
success = "green"
failure = "red"

# === Keypads ===
[[keypads]]
id = "main_keypad"
type = "usb_hid"
enabled = true

[keypads.config]
vendor_id = 0x1234
product_id = 0x5678
timeout = 30000  # ms for typing timeout
min_pin_length = 4
max_pin_length = 8

[keypads.feedback]
beep_on_press = true
mask_display = true
mask_char = "*"

[[keypads]]
id = "wiegand_keypad"
type = "wiegand"
enabled = false

[keypads.config]
data0_pin = 17
data1_pin = 18
bits = 26  # 26 or 34 bit Wiegand

# === Turnstile Controllers ===
[turnstile]
enabled = true
type = "relay_board"  # relay_board, gpio, modbus

[turnstile.relay_board]
port = "/dev/ttyUSB0"
baudrate = 9600
entry_relay = 1
exit_relay = 2
timeout = 5  # seconds

[turnstile.gpio]
platform = "raspberry_pi"
entry_pin = 17
exit_pin = 27
sensor_rotation_pin = 22
sensor_position_pin = 23
active_low = false

[turnstile.sensors]
rotation_enabled = true
position_enabled = true
debounce_ms = 50

# === Device Mapping ===
# Maps physical IDs to system IDs
[mapping]
cards = [
    { uid = "04A1B2C3D4E5F6", system_id = "12345678" },
    { uid = "04D5E6F7A8B9C0", system_id = "87654321" }
]

# === Security ===
[security]
encrypt_templates = true
secure_storage = true
wipe_on_tamper = false
```

## 10. Logging Configuration (config/logging.toml)

```toml
# Logging Configuration

[general]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json, pretty, compact
timestamps = true
target = "stdout"  # stdout, file, both

[file]
enabled = true
path = "logs/turnkey.log"
rotation = "daily"  # daily, size, never
max_size = "100MB"
max_backups = 7
compress = true

[filters]
# Per-module log levels
turnkey_core = "debug"
turnkey_protocol = "debug"
turnkey_hardware = "info"
turnkey_rfid = "debug"
turnkey_biometric = "debug"
turnkey_storage = "warn"
turnkey_network = "info"

[structured]
# Additional structured fields
add_hostname = true
add_process_id = true
add_thread_id = true
add_module_path = true

[events]
# Specific events for logging
card_read = true
biometric_capture = true
access_granted = true
access_denied = true
device_connected = true
device_error = true
protocol_error = true

[performance]
# Performance metrics
log_slow_queries = true
slow_query_threshold = 100  # ms
log_memory_usage = true
memory_log_interval = 60  # seconds

[audit]
# Audit trail
enabled = true
file = "logs/audit.log"
include_all_access = true
include_config_changes = true
include_admin_actions = true
```

## 11. Makefile

```makefile
# Turnkey Access Control Emulator - Makefile
RUST_VERSION := 1.90
CARGO := cargo
CROSS := cross
TARGET_LINUX_X64 := x86_64-unknown-linux-gnu
TARGET_LINUX_ARM := aarch64-unknown-linux-gnu
TARGET_RPI := armv7-unknown-linux-gnueabihf

.PHONY: all build release test clean install-deps

# Default target
all: build

# Check Rust version
check-rust:
	@rustc --version | grep -E "1.(90|9[1-9]|[1-9][0-9][0-9])" || \
		(echo "Rust 1.90+ required" && exit 1)

# Measure build time
build-timed: check-rust
	@echo "Starting timed build..."
	@time $(CARGO) build --workspace

# Check Rust version
version:
	@rustc --version
	@cargo --version

# Install system dependencies (Debian/Ubuntu)
install-deps:
	sudo apt-get update
	sudo apt-get install -y \
		build-essential \
		pkg-config \
		libusb-1.0-0-dev \
		libudev-dev \
		libpcsclite-dev \
		libsqlite3-dev \
		libssl-dev \
		clang \
		pcscd \
		pcsc-tools
	# Install Rust tools
	cargo install sqlx-cli
	cargo install cargo-watch
	cargo install cargo-audit
	cargo install cargo-tarpaulin
	cargo install cross

# Setup hardware permissions
setup-hardware:
	# PCSC for card readers
	sudo systemctl enable pcscd
	sudo systemctl start pcscd
	# USB permissions
	sudo cp scripts/99-turnkey.rules /etc/udev/rules.d/
	sudo udevadm control --reload-rules
	sudo usermod -aG dialout,plugdev $$USER

# Build commands
build: check-rust
	$(CARGO) build --workspace

release:
	$(CARGO) build --release --workspace

# Cross-compilation
build-arm:
	$(CROSS) build --release --target $(TARGET_LINUX_ARM)

build-rpi:
	$(CROSS) build --release --target $(TARGET_RPI) \
		--features raspberry-pi

# Test commands
test:
	$(CARGO) test --workspace --all-features

test-hardware:
	sudo $(CARGO) test --workspace \
		--features "hardware" \
		-- --test-threads=1

test-integration:
	$(CARGO) test --workspace \
		--test integration \
		-- --test-threads=1

# PCSC test
pcsc-test:
	pcsc_scan

# Benchmarks
bench:
	$(CARGO) bench --workspace

# Database
db-create:
	sqlx database create

db-migrate:
	sqlx migrate run --source migrations

db-reset: db-drop db-create db-migrate

db-drop:
	sqlx database drop -y

# Documentation
docs:
	$(CARGO) doc --workspace --no-deps --open

# Linting and formatting
lint:
	$(CARGO) clippy --workspace -- -D warnings

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

# Security audit
audit:
	$(CARGO) audit

# Coverage
coverage:
	$(CARGO) tarpaulin --workspace --out Html

# Clean
clean:
	$(CARGO) clean
	rm -rf logs/*.log

# Run development server
run:
	RUST_LOG=debug $(CARGO) run --bin turnkey-cli -- server

# Run with hardware
run-hw:
	sudo RUST_LOG=debug $(CARGO) run --bin turnkey-cli \
		--features "hardware" -- server

# Watch mode for development
watch:
	$(CARGO) watch -x 'run --bin turnkey-cli'

# Install
install: release
	sudo cp target/release/turnkey-cli /usr/local/bin/turnkey

# Uninstall
uninstall:
	sudo rm -f /usr/local/bin/turnkey

# Docker
docker-build:
	docker build -t turnkey-emulator:latest .

docker-run:
	docker run -d \
		--name turnkey \
		--device /dev/bus/usb \
		-v /var/run/pcscd:/var/run/pcscd \
		-p 8080:8080 \
		turnkey-emulator:latest

# Help
help:
	@echo "Turnkey Access Control Emulator - Build System"
	@echo ""
	@echo "Setup:"
	@echo "  install-deps     Install system dependencies"
	@echo "  setup-hardware   Configure hardware permissions"
	@echo ""
	@echo "Build:"
	@echo "  build           Build debug version"
	@echo "  release         Build release version"
	@echo "  build-arm       Cross-compile for ARM64"
	@echo "  build-rpi       Cross-compile for Raspberry Pi"
	@echo ""
	@echo "Test:"
	@echo "  test            Run all tests"
	@echo "  test-hardware   Run hardware tests (requires sudo)"
	@echo "  bench           Run benchmarks"
	@echo ""
	@echo "Database:"
	@echo "  db-create       Create database"
	@echo "  db-migrate      Run migrations"
	@echo "  db-reset        Reset database"
	@echo ""
	@echo "Development:"
	@echo "  run             Run development server"
	@echo "  run-hw          Run with hardware support"
	@echo "  watch           Watch mode"
	@echo "  docs            Generate documentation"
	@echo ""
	@echo "Quality:"
	@echo "  lint            Run clippy"
	@echo "  fmt             Format code"
	@echo "  audit           Security audit"
	@echo "  coverage        Generate coverage report"
```

## 12. rust-toolchain.toml

```toml
[toolchain]
channel = "1.90.0"
components = ["rustfmt", "clippy", "rust-analyzer"]
profile = "default"
```

## 13. Setup Scripts (scripts/install-deps.sh)

```bash
#!/bin/bash
set -e

echo "=== Turnkey Access Control Emulator - Dependency Installation ==="

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    DISTRO=$(lsb_release -si)
    VERSION=$(lsb_release -sr)
    KERNEL=$(uname -r)

    echo "Detected: $DISTRO $VERSION (Kernel $KERNEL)"

    # Check kernel version (6.1+)
    KERNEL_MAJOR=$(echo $KERNEL | cut -d. -f1)
    KERNEL_MINOR=$(echo $KERNEL | cut -d. -f2)

    if [ "$KERNEL_MAJOR" -lt 6 ] || ([ "$KERNEL_MAJOR" -eq 6 ] && [ "$KERNEL_MINOR" -lt 1 ]); then
        echo "Warning: Kernel 6.1+ recommended, you have $KERNEL"
    fi

    # Install based on distro
    case $DISTRO in
        Ubuntu|Debian)
            sudo apt-get update
            sudo apt-get install -y \
                build-essential \
                pkg-config \
                libusb-1.0-0-dev \
                libudev-dev \
                libpcsclite-dev \
                libsqlite3-dev \
                libssl-dev \
                libhidapi-dev \
                clang \
                llvm \
                pcscd \
                pcsc-tools \
                usbutils
            ;;
        Fedora|RedHat|CentOS)
            sudo dnf install -y \
                gcc \
                pkg-config \
                systemd-devel \
                pcsc-lite-devel \
                sqlite-devel \
                openssl-devel \
                hidapi-devel \
                clang \
                llvm \
                pcsc-tools \
                usbutils
            ;;
        Arch|Manjaro)
            sudo pacman -S --needed \
                base-devel \
                pkg-config \
                libusb \
                systemd-libs \
                pcsclite \
                sqlite \
                openssl \
                hidapi \
                clang \
                llvm \
                pcsc-tools \
                usbutils
            ;;
        *)
            echo "Unsupported distribution: $DISTRO"
            exit 1
            ;;
    esac
else
    echo "This script is for Linux only"
    exit 1
fi

echo "=== Installing Rust tools ==="
cargo install sqlx-cli --no-default-features --features sqlite
cargo install cargo-watch
cargo install cargo-audit
cargo install cross

echo "=== Setup complete! ==="
```

## 5. Terminal User Interface (TUI)

### Overview

The Turnkey emulator provides a realistic Terminal User Interface built with `ratatui` that mimics the appearance and behavior of physical Brazilian turnstiles (Primme SF, Henry Lumen).

### Main Components

#### Display LCD (2 lines × 40 columns)
- **Visual**: Blue background, white text, bold font
- **Purpose**: Shows system messages, user input, and status
- **States**: IDLE, VALIDATING, GRANTED, DENIED, WAITING_ROTATION
- **Themes**: Default (blue), Dark (black/green), Light (white), Green (LCD-style)

#### Numeric Keypad
- **Layout**: 4 rows × 3 columns (0-9, *, #)
- **Buttons**: ENTER, CANCEL, CLEAR
- **Visual Feedback**: Key highlighting on press (gray → yellow)
- **Input**: Buffered code entry with max length validation

#### Logs Panel
- **Type**: Scrollable, auto-updating event log
- **Format**: `[HH:MM:SS] Message`
- **Capacity**: 1000 entries (configurable)
- **Colors**: Info (white), Success (green), Warning (yellow), Error (red)
- **Features**: Search, scroll navigation, timestamp filtering

#### Status Bar
- **Reader Status**: RFID✓ BIO✗ KEYPAD✓ WIEGAND✗
- **Statistics**: Event count, user count, storage usage
- **Network**: Connection status, IP address, mode (ONLINE/OFFLINE)

### Keyboard Shortcuts

| Key             | Action                        |
|-----------------|-------------------------------|
| `0`-`9`, `*`, `#` | Keypad input                  |
| `Enter`         | Confirm / Submit              |
| `Escape`        | Cancel input                  |
| `Backspace`     | Clear last digit              |
| `Tab`           | Switch focus (Display ↔ Logs) |
| `F1`            | Show help                     |
| `F5`            | Reload configuration          |
| `F8`            | Export events                 |
| `F10`           | Settings menu                 |
| `q` or `Ctrl+C` | Quit emulator                 |

### Responsive Design

- **Minimum**: 80 columns × 24 lines
- **Optimal**: 120 columns × 30 lines
- **Layout**: 70% emulator column, 30% logs column
- **Adaptation**: Auto-adjusts proportions based on terminal size

**See**: [TUI Specification](tui-specification.md) for complete design details.

---

## 6. Operation Modes

The emulator supports two primary operation modes that determine how access validation is performed.

### ONLINE Mode (Production Testing)

**Purpose**: Emulate a physical turnstile that sends access requests to an external TCP client for validation.

**Key Characteristics**:
- Emulator has no validation logic (mimics real hardware)
- TCP client makes all access decisions
- Real-time event and status reporting
- Configurable timeout and fallback handling

**Configuration**:
```toml
[mode]
online = true
status_online = true       # Send periodic heartbeat
event_online = true        # Send events in real-time
fallback_offline = true    # Switch to offline on timeout

[network]
type = "tcp"
tcp_mode = "server"        # Wait for client connections
ip = "192.168.0.100"
port = 3000
```

**Message Flow**:
1. **Access Request** (Emulator → Client):
   ```
   01+REON+000+0]12345678]20/10/2025 14:30:00]1]0]
   ```
   - Card number: `12345678`
   - Timestamp: `20/10/2025 14:30:00`
   - Direction: `1` (entry), `2` (exit)

2. **Grant Response** (Client → Emulator):
   ```
   01+REON+00+6]5]Acesso liberado]
   ```
   - Command: `00+6` (grant exit)
   - Display time: `5` seconds
   - Message: `Acesso liberado`

3. **Rotation Events**:
   - **Waiting**: `01+REON+000+80]...]`
   - **Complete**: `01+REON+000+81]...]`
   - **Timeout**: `01+REON+000+82]...]`

**Timeout Handling**:
- Default timeout: 3000ms
- If `fallback_offline = true`: Switch to OFFLINE mode
- If `fallback_offline = false`: Deny access, return to IDLE

### OFFLINE Mode (Standalone Operation)

**Purpose**: Local validation using SQLite database without network dependencies.

**Key Characteristics**:
- All validation logic in emulator
- No network requirements
- Local user/card database
- Suitable for testing and development

**Configuration**:
```toml
[mode]
online = false
smart_mode = true
local_registration = true

[storage]
database_path = "data/turnkey.db"
max_events = 50000
max_users = 5000
```

**Validation Flow**:
1. **User Lookup**: Query by code, card number, or biometric
2. **Validity Check**: Active status, validity period
3. **Access Method Check**: Verify allowed methods (card, bio, keypad)
4. **Grant/Deny**: Display message, log event, simulate rotation

**Database Tables**:
- `users` - User credentials and access permissions
- `cards` - RFID card associations
- `biometric_templates` - Fingerprint data
- `access_logs` - Complete access history

### Hybrid Mode (Fallback)

**Configuration**:
```toml
[mode]
online = true
fallback_offline = true
fallback_timeout = 3000
```

**Behavior**:
1. Start in ONLINE mode
2. On validation timeout: Switch to OFFLINE
3. Query local database, grant/deny locally
4. Display "MODO OFFLINE"
5. When connection restored: Sync events, return to ONLINE

**See**: [Emulator Modes](emulator-modes.md) for detailed flow documentation.

---

## 7. Configuration System

### Configuration Files

The emulator uses TOML-based configuration with a priority system:

**File Locations**:
```
config/
├── default.toml         # Default configuration (committed to git)
├── local.toml           # Local overrides (gitignored)
├── hardware.toml        # Hardware-specific settings
└── logging.toml         # Logging configuration
```

**Load Priority** (highest to lowest):
1. **Environment Variables**: `TURNKEY_SECTION_KEY=value`
2. **config/local.toml**: Development overrides
3. **config/default.toml**: Baseline configuration

### Main Configuration Sections

#### [device] - Device Identity
```toml
id = 1                          # Device ID (01-99)
model = "Turnkey Emulator"
firmware_version = "1.0.0"
protocol_version = "1.0.0.15"
display_message = "DIGITE SEU CÓDIGO"  # Max 40 characters
```

#### [mode] - Operation Mode
```toml
online = true                   # true = ONLINE, false = OFFLINE
status_online = true            # Send heartbeat
event_online = true             # Send events in real-time
fallback_offline = true         # Fallback on timeout
fallback_timeout = 3000         # Milliseconds
```

#### [network] - Network Settings
```toml
type = "tcp"                    # tcp or serial
ip = "192.168.0.100"
port = 3000
tcp_mode = "server"             # server or client
dhcp = false
```

#### [readers] - Reader Configuration
```toml
reader1 = "rfid"                # rfid, keypad, biometric, wiegand, disabled
reader2 = "keypad"
reader3 = "disabled"
reader4 = "disabled"
keypad_enabled = true
keypad_timeout = 30             # seconds
```

#### [biometrics] - Biometric Settings
```toml
verify_card_with_bio = false    # Require fingerprint after card
treat_1n = true                 # 1:N identification mode
auto_on = false                 # Auto-identify by fingerprint
sensitivity = 55                # 48-55 (higher = more sensitive)
security_level = 80             # 48-82 (higher = stricter)
```

#### [storage] - Database Settings
```toml
database_path = "data/turnkey.db"
max_events = 100000
max_users = 10000
wal_enabled = true
synchronous = "NORMAL"          # OFF, NORMAL, FULL
backup_interval = 60            # minutes
```

#### [ui] - Terminal UI Settings
```toml
enabled = true
display_lines = 2
display_cols = 40
theme = "default"               # default, dark, light, green
log_panel_height = 30           # percentage
```

### Runtime Configuration Reload

**Methods**:
- **Signal**: `kill -HUP <pid>` (Linux)
- **TUI**: Press `F5`
- **API**: `POST /api/reload-config` (future)

**Reloadable Settings**: Display messages, reader enable/disable, log levels, UI theme, timeouts

**Requires Restart**: Network IP/port, database path, device ID, operation mode

**See**: [Emulator Configuration](emulator-configuration.md) for complete reference.

---

## 8. Data Import/Export

The emulator supports multiple file formats for bulk operations, backups, and interoperability with external systems.

### Supported File Formats

#### 1. User Import/Export (`colaborador.txt`)
**Format**: Pipe-separated values
**Fields**: PIS, NOME, MATRICULA, CPF, VALIDADE_INICIO, VALIDADE_FIM, ATIVO, ALLOW_CARD, ALLOW_BIO, ALLOW_KEYPAD, CODIGO

**Example**:
```
12345678901|João da Silva|1001|12345678901|01/01/2025|31/12/2025|1|1|1|1|1234
98765432101|Maria Santos|1002|98765432101|01/01/2025||1|1|0|1|5678
```

#### 2. Card Import/Export (`cartoes.txt`)
**Format**: Pipe-separated values
**Fields**: NUMERO_CARTAO, MATRICULA, VALIDADE_INICIO, VALIDADE_FIM, ATIVO

**Example**:
```
00000000000011912322|1001|01/01/2025|31/12/2025|1
00000000000022823433|1002|01/01/2025||1
```

#### 3. Biometric Templates (`biometria.txt`)
**Format**: Pipe-separated values
**Fields**: MATRICULA, POSICAO (0-9 finger index), TEMPLATE_BASE64

**Example**:
```
1001|1|AQIDBAUG...BASE64...ENCODED==
1001|6|BQYHCQ0K...BASE64...ENCODED==
```

#### 4. Access Logs Export (`eventos.txt`)
**Format**: Pipe-separated values
**Fields**: NSR, DATA_HORA, MATRICULA, CARTAO, TIPO_EVENTO, DIRECAO, DISPOSITIVO, VALIDACAO, NOME

**Example**:
```
1|20/10/2025 08:15:23|1001|00000000000011912322|0|1|01|O|João da Silva
2|20/10/2025 09:30:12|1002|00000000000022823433|0|1|01|F|Maria Santos
```

#### 5. AFD Format (Brazilian Legal Standard)
**Purpose**: Time-tracking and access control compliance (Portaria 1510/2009)
**Structure**:
- Record Type 1: Header (CNPJ, company name, period)
- Record Type 2: Device identification
- Record Type 3: Access events
- Record Type 9: Trailer (record count)

**Example**:
```
1|12345678000190||EMPRESA EXEMPLO LTDA|01102025|20102025|20102025143000
2|1|TK00000001|Turnkey Emulator|1.0.0
3|000001|20102025|081523|12345678901
9|3
```

### Bulk Import/Export Commands

**Import All Data**:
```bash
turnkey-cli import bulk ./import-data/
```

**Export Commands**:
```bash
# Export users
turnkey-cli export colaborador --output colaborador-backup.txt

# Export cards
turnkey-cli export cartoes --output cartoes-backup.txt

# Export events with date range
turnkey-cli export eventos \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output eventos-outubro.txt

# Export AFD format
turnkey-cli export afd \
  --cnpj "12345678000190" \
  --razao-social "Empresa Exemplo" \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output afd-outubro.txt
```

### Transaction Behavior

- All imports wrapped in database transaction
- Rollback on any validation failure
- Duplicate records skipped with warning
- Progress shown for large files (>1000 records)

**See**: [Data Formats](data-formats.md) for complete format specifications.

---

## 14. Technical Justification

### **Libraries Selected:**

1. **Tokio** - Most mature and performant async runtime in the Rust ecosystem
2. **SQLx** - Type-safe SQL with compile-time checking, native async support
3. **Tracing** - Modern structured logging, superior to the `log` crate
4. **Serde** - De facto standard for serialization in Rust
5. **PCSC** - Standard interface for smart card readers on Linux
6. **HidAPI** - Cross-platform access to USB HID devices
7. **DashMap** - Lock-free concurrent HashMap for high performance
8. **Ratatui** - Modern, actively maintained TUI framework with excellent documentation

### **Architecture:**

- **Cargo Workspace**: Maximum modularity, optimized compilation
- **Separation of Concerns**: Each crate has a single responsibility
- **Trait-based Design**: Extensibility via traits, facilitates mocking
- **Async/Await**: Performance and scalability
- **Type Safety**: Extensive use of Rust's type system

### **Storage:**

- **SQLite**: Embedded, zero-configuration, perfect for edge devices
- **Migrations**: Controlled schema versioning
- **Repository Pattern**: Data layer abstraction

### **Compatibility:**

- **Linux Kernel 6.1+**: Modern USB, GPIO, hidraw support
- **Cross-compilation**: Supports x64, ARM64, ARMv7 (Raspberry Pi)
- **Rust 1.90+**: Modern features, stable async traits
- **Rust edition**: 2024

---

## 15. References

### Core Documentation
- [README.md](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - Development guide for Claude Code

### Architecture and Design
- [Emulator Architecture](emulator-architecture.md) - System architecture and component design
- [Emulator Modes](emulator-modes.md) - ONLINE vs OFFLINE operation modes
- [TUI Specification](tui-specification.md) - Terminal User Interface design

### Configuration and Data
- [Emulator Configuration](emulator-configuration.md) - Complete TOML configuration reference
- [Data Formats](data-formats.md) - Import/export file format specifications

### Protocol
- [Henry Protocol Guide](turnkey-protocol-guide-en.md) - Henry protocol specification
- [Henry Client Commands](henry-client-emulator-commands.md) - Commands discovered from official client

### Hardware
- [Hardware Setup Guide](hardware-setup.md) - Physical hardware integration
- [Primme SF Manual](Catraca Sf item 01.pdf) - Original equipment documentation
```
# Turnkey Emulator Configuration

## Overview

The Turnkey emulator uses TOML configuration files for all runtime settings. Configuration is loaded at boot and can be reloaded on-demand without restarting the application.

## Configuration Files

### File Locations

```
config/
├── default.toml         # Default configuration (committed to git)
├── local.toml           # Local overrides (gitignored)
├── hardware.toml        # Hardware-specific settings
└── logging.toml         # Logging configuration
```

### Load Priority

1. **Environment Variables** (highest priority)
   - Format: `TURNKEY_SECTION_KEY=value`
   - Example: `TURNKEY_DEVICE_ID=5`

2. **config/local.toml** (development overrides)
   - Gitignored file for local customization
   - Overrides default.toml values

3. **config/default.toml** (baseline configuration)
   - Default values for all settings
   - Committed to repository

## Complete Configuration Reference

### [device] - Device Identity

Basic device identification and display settings.

```toml
[device]
# Device ID (01-99) - Must be unique in the network
id = 1

# Model name displayed in Info screen
model = "Turnkey Emulator"

# Firmware version (simulated)
firmware_version = "1.0.0"

# Protocol version compatibility
protocol_version = "1.0.0.15"

# Default message displayed on LCD (max 40 characters)
display_message = "DIGITE SEU CÓDIGO"
```

**Validation Rules:**
- `id`: Must be 1-99
- `model`: Max 50 characters
- `display_message`: Max 40 characters
- `firmware_version`: Semantic versioning format (X.Y.Z)

---

### [mode] - Operation Mode

Controls how the emulator validates access and communicates.

```toml
[mode]
# Primary operation mode: true = ONLINE, false = OFFLINE
online = true

# Send device status to server periodically (heartbeat)
status_online = true

# Send access events to server in real-time
event_online = true

# Smart mode: only write to memory when necessary (saves flash wear)
smart_mode = true

# Allow user registration directly on the device
local_registration = false

# Save user reference (matricula) in access logs
save_reference = true

# Fallback to offline mode if server is unreachable
fallback_offline = true

# Timeout for fallback decision (milliseconds)
fallback_timeout = 3000
```

**Behaviors:**

| Setting              | true                         | false                       |
|----------------------|------------------------------|-----------------------------|
| `online`             | Validate via TCP server      | Validate via SQLite         |
| `status_online`      | Send heartbeat every 60s     | No status reporting         |
| `event_online`       | Send events immediately      | Batch events for later sync |
| `smart_mode`         | Write only on change         | Write every event           |
| `local_registration` | Enable F-key registration    | Disable registration        |
| `save_reference`     | Include matricula in logs    | Omit matricula              |
| `fallback_offline`   | Switch to offline on timeout | Deny on timeout             |

---

### [network] - Network Settings

TCP/IP and serial communication configuration.

```toml
[network]
# Communication type: "tcp" or "serial"
type = "tcp"

# IP address for TCP mode
ip = "192.168.0.100"

# TCP port
port = 3000

# Network gateway
gateway = "192.168.0.1"

# Subnet mask
mask = "255.255.255.0"

# DNS server
dns = "192.168.0.1"

# MAC address (for hardware identification)
mac = "00:1A:2B:3C:4D:5E"

# Enable DHCP (auto-configure IP)
dhcp = false

# TCP mode: "server" (wait for connections) or "client" (connect to server)
tcp_mode = "server"

# Server address (for client mode)
server_address = "192.168.0.1"

# Server port (for client mode)
server_port = 3000

# Hostname for DNS resolution
hostname = "TURNKEY001"

# Serial port (for serial mode)
serial_port = "/dev/ttyUSB0"

# Serial baud rate
serial_baud = 115200

# Connection retry attempts
retry_attempts = 3

# Retry delay (milliseconds)
retry_delay = 5000
```

**Network Modes:**

| Mode                  | Behavior                                          |
|-----------------------|---------------------------------------------------|
| `tcp_mode = "server"` | Bind to `ip:port` and wait for client connections |
| `tcp_mode = "client"` | Connect to `server_address:server_port`           |

**DHCP:**
- When `dhcp = true`, `ip`, `gateway`, `mask`, `dns` are auto-configured
- Static configuration is ignored

---

### [security] - Security Settings

Authentication and access control.

```toml
[security]
# Username for web interface and configuration access
username = "admin"

# Password (hashed in production, plain in dev)
password = "123456"

# Require authentication for configuration changes
require_auth = true

# Enable secure connection (restrict to specific IP)
secure_connection = false

# Allowed IP address (when secure_connection = true)
allowed_ip = ""

# Session timeout (minutes)
session_timeout = 30

# Enable audit logging
audit_log = true

# Maximum failed login attempts before lockout
max_failed_attempts = 5

# Lockout duration (minutes)
lockout_duration = 15
```

**Security Notes:**
- Default password `123456` matches real Primme SF devices
- In production, hash passwords using bcrypt
- `allowed_ip` restricts configuration access to single IP
- Audit log records all configuration changes

---

### [readers] - Reader Configuration

Enable/disable and configure input peripherals.

```toml
[readers]
# Reader 1 type: "rfid", "keypad", "biometric", "wiegand", "disabled"
reader1 = "rfid"

# Reader 2 type
reader2 = "keypad"

# Reader 3 type
reader3 = "disabled"

# Reader 4 type
reader4 = "disabled"

# Enable keypad input
keypad_enabled = true

# Keypad timeout (seconds)
keypad_timeout = 30

# Minimum code length
min_code_length = 4

# Maximum code length
max_code_length = 20

# Card number format: "numeric" or "alphanumeric"
card_format = "numeric"

# Enable beep on key press
beep_enabled = true
```

**Reader Types:**

| Type        | Description             | Mock   | Real Hardware        |
|-------------|-------------------------|--------|----------------------|
| `rfid`      | RFID/NFC card reader    | Yes    | ACR122U, RC522       |
| `keypad`    | Numeric keypad          | Yes    | USB HID, GPIO matrix |
| `biometric` | Fingerprint scanner     | Yes    | Control iD iDBio     |
| `wiegand`   | Wiegand protocol reader | Yes    | Generic Wiegand      |
| `disabled`  | Reader slot unused      | N/A    | N/A                  |

**Code Length:**
- `min_code_length`: 1-20 (default: 4)
- `max_code_length`: 1-20 (default: 20)
- Must satisfy: `min_code_length ≤ max_code_length`

---

### [biometrics] - Biometric Settings

Fingerprint reader configuration (when `reader* = "biometric"`).

```toml
[biometrics]
# Require fingerprint verification after card/code entry
verify_card_with_bio = false

# Enable 1:N identification mode (search all fingerprints)
treat_1n = true

# Auto-identify user by fingerprint without code entry
auto_on = false

# Make fingerprint verification mandatory
required = false

# Biometric sensor sensitivity (48-55, higher = more sensitive)
sensitivity = 55

# Security level for fingerprint matching (48-82, higher = stricter)
security_level = 80

# Image quality threshold (48-51)
image_quality = 49

# Fast mode threshold (48-54, lower = faster but less accurate)
fast_mode = 54

# Luminosity mode: "internal" or "external"
# "external" for bright environments, "internal" for indoor
luminosity = "internal"

# Maximum fingerprints per user
max_templates_per_user = 2

# Fingerprint enrollment retries
enrollment_retries = 3
```

**Sensitivity vs Security:**

| Sensitivity   | Security Level   | Use Case                              |
|---------------|------------------|---------------------------------------|
| 48-50         | 48-60            | Low security, high acceptance rate    |
| 51-53         | 61-70            | Balanced                              |
| 54-55         | 71-82            | High security, may reject valid users |

**Modes:**

| Mode                          | Description                      |
|-------------------------------|----------------------------------|
| `verify_card_with_bio = true` | Card + fingerprint required      |
| `treat_1n = true`             | Search all fingerprints (slow)   |
| `auto_on = true`              | Identify by fingerprint alone    |
| `required = true`             | Deny access if fingerprint fails |

---

### [storage] - Database Settings

SQLite configuration and capacity limits.

```toml
[storage]
# SQLite database file path
database_path = "data/turnkey.db"

# Maximum events in database (oldest deleted when exceeded)
max_events = 100000

# Maximum users
max_users = 10000

# Maximum biometric templates
max_templates = 10000

# Maximum cards
max_cards = 10000

# Enable write-ahead logging (WAL) for better concurrency
wal_enabled = true

# Synchronous mode: "OFF", "NORMAL", "FULL"
# OFF = fastest, FULL = safest
synchronous = "NORMAL"

# Cache size (KB)
cache_size = 2000

# Auto-vacuum: "NONE", "FULL", "INCREMENTAL"
auto_vacuum = "INCREMENTAL"

# Backup interval (minutes, 0 = disabled)
backup_interval = 60

# Backup directory
backup_dir = "data/backups"

# Maximum backup files to keep
max_backups = 10
```

**Synchronous Modes:**

| Mode     | Speed   | Safety   | Use Case                 |
|----------|---------|----------|--------------------------|
| `OFF`    | Fast    | Low      | Development only         |
| `NORMAL` | Medium  | Medium   | Production (recommended) |
| `FULL`   | Slow    | High     | Critical data            |

**Capacity Planning:**

| Item     | Storage    | Calculation           |
|----------|------------|-----------------------|
| Event    | ~200 bytes | 100K events = ~20 MB  |
| User     | ~500 bytes | 10K users = ~5 MB     |
| Template | ~500 bytes | 10K templates = ~5 MB |
| Card     | ~100 bytes | 10K cards = ~1 MB     |

**Total:** ~31 MB for maximum capacity

---

### [ui] - Terminal UI Settings

TUI appearance and behavior (ratatui).

```toml
[ui]
# Enable TUI (false = headless mode)
enabled = true

# LCD display lines
display_lines = 2

# LCD display columns
display_cols = 40

# Frame rate (FPS)
frame_rate = 60

# Color theme: "default", "dark", "light", "green"
theme = "default"

# Log panel height (percentage of screen)
log_panel_height = 30

# Maximum log entries in memory
max_log_entries = 1000

# Auto-scroll logs
auto_scroll = true

# Show status bar
show_status_bar = true

# Show help hints
show_hints = true

# Enable mouse support
mouse_enabled = false

# Confirm before quit
confirm_quit = true
```

**Themes:**

| Theme     | Display BG   | Display FG   | Keypad    | Logs   |
|-----------|--------------|--------------|-----------|--------|
| `default` | Blue         | White        | Gray      | Black  |
| `dark`    | Black        | Green        | Gray      | Black  |
| `light`   | White        | Black        | LightGray | White  |
| `green`   | Black        | Green        | DarkGreen | Black  |

---

### [logging] - Logging Configuration

Application logging settings (separate from access logs).

```toml
[logging]
# Log level: "TRACE", "DEBUG", "INFO", "WARN", "ERROR"
level = "INFO"

# Log to file
file_enabled = true

# Log file path
file_path = "logs/turnkey.log"

# Log file max size (MB)
file_max_size = 10

# Max log files to keep (rotation)
file_max_count = 5

# Log to console (stdout)
console_enabled = true

# Console log format: "full", "compact", "json"
console_format = "full"

# Enable colored output
console_color = true

# Log module filter (comma-separated)
# Example: "turnkey_emulator,turnkey_protocol"
module_filter = ""

# Include timestamps
include_timestamp = true

# Include thread ID
include_thread = false

# Include file/line info
include_location = false
```

**Log Levels:**

| Level   | Purpose                         |
|---------|---------------------------------|
| `TRACE` | Very detailed, development only |
| `DEBUG` | Debugging information           |
| `INFO`  | General informational messages  |
| `WARN`  | Warning messages                |
| `ERROR` | Error messages only             |

**Log Rotation:**
- Files rotate when reaching `file_max_size`
- Format: `turnkey.log`, `turnkey.log.1`, `turnkey.log.2`, ...
- Oldest deleted when `file_max_count` exceeded

---

## Environment Variable Overrides

Any configuration value can be overridden via environment variables using the format:

```bash
TURNKEY_SECTION_KEY=value
```

### Examples

```bash
# Override device ID
export TURNKEY_DEVICE_ID=5

# Override network IP
export TURNKEY_NETWORK_IP=10.0.0.50

# Override mode to offline
export TURNKEY_MODE_ONLINE=false

# Override log level
export TURNKEY_LOGGING_LEVEL=DEBUG
```

### Nested Values

For nested values, use double underscore:

```bash
# Override storage database path
export TURNKEY_STORAGE_DATABASE_PATH=/var/lib/turnkey/db.sqlite
```

---

## Configuration Validation

On boot, configuration is validated according to these rules:

### Required Fields
- `device.id`
- `network.ip` (if `network.dhcp = false`)
- `network.port`

### Range Validation
- `device.id`: 1-99
- `biometrics.sensitivity`: 48-55
- `biometrics.security_level`: 48-82
- `storage.max_events`: 100-1000000

### Format Validation
- `network.ip`: Valid IPv4 address
- `network.mac`: Valid MAC address (XX:XX:XX:XX:XX:XX)
- `device.firmware_version`: Semantic versioning (X.Y.Z)

### Consistency Checks
- `mode.online = true` requires `network.type` and `network.ip`
- `readers.reader* = "biometric"` requires `[biometrics]` section
- `security.secure_connection = true` requires `security.allowed_ip`

**On Validation Failure:**
- Log error with field name and constraint
- Use default value for the field
- Display warning in TUI

---

## Configuration Examples

### Minimal ONLINE Configuration

```toml
[device]
id = 1

[mode]
online = true

[network]
type = "tcp"
ip = "192.168.0.100"
port = 3000
tcp_mode = "server"

[readers]
reader1 = "keypad"
```

### Full-Featured OFFLINE Configuration

```toml
[device]
id = 1
display_message = "BEM-VINDO"

[mode]
online = false
smart_mode = true
local_registration = true

[readers]
reader1 = "rfid"
reader2 = "keypad"
reader3 = "biometric"

[biometrics]
verify_card_with_bio = true
treat_1n = true
sensitivity = 53
security_level = 75

[storage]
max_events = 50000
max_users = 5000
backup_interval = 30

[ui]
theme = "green"
log_panel_height = 25
```

### Development/Testing Configuration

```toml
[device]
id = 99

[mode]
online = true
fallback_offline = true

[network]
type = "tcp"
ip = "127.0.0.1"
port = 3000
tcp_mode = "client"
server_address = "127.0.0.1"

[readers]
reader1 = "keypad"  # All mock readers

[logging]
level = "DEBUG"
console_enabled = true
console_format = "full"
console_color = true

[storage]
synchronous = "OFF"  # Fast but unsafe
```

---

## Runtime Configuration Reload

Configuration can be reloaded without restarting:

### Via Signal (Linux)
```bash
kill -HUP <pid>
```

### Via TUI Shortcut
- Press `F5`

### Via API (Future)
```bash
curl -X POST http://192.168.0.100/api/reload-config
```

**What Gets Reloaded:**
- Display messages
- Reader enable/disable
- Log levels
- UI theme
- Timeouts

**What Requires Restart:**
- Network IP/port
- Database path
- Device ID
- Mode (online/offline)

---

## Troubleshooting

### Configuration Not Loading

1. Check file exists: `ls -la config/default.toml`
2. Check TOML syntax: `toml-check config/default.toml`
3. Check file permissions: `chmod 644 config/default.toml`
4. Check logs: `tail -f logs/turnkey.log`

### Environment Variable Not Working

```bash
# Verify variable is set
env | grep TURNKEY

# Check case sensitivity (must be uppercase)
export TURNKEY_DEVICE_ID=5  # Correct
export turnkey_device_id=5  # Incorrect
```

### Validation Errors

```bash
# Run with validation-only mode
cargo run --bin turnkey-cli -- validate-config

# Check specific section
cargo run --bin turnkey-cli -- validate-config --section device
```

---

## References

- [Emulator Architecture](emulator-architecture.md) - System design
- [Emulator Modes](emulator-modes.md) - ONLINE vs OFFLINE operation
- [Data Formats](data-formats.md) - Import/export file formats
- [TUI Specification](tui-specification.md) - Terminal interface design

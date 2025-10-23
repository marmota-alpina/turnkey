# Turnkey Emulator Architecture

## Overview

The Turnkey emulator is a software-based turnstile access control system that faithfully replicates the behavior of Brazilian physical turnstiles (Primme SF, Henry Lumen, Argos). It allows developers to test access control integrations without requiring physical hardware.

## Design Goals

1. **Hardware-Independent**: Run with mock peripherals or connect to real devices
2. **Protocol-Compliant**: Full implementation of Henry protocol
3. **Realistic Behavior**: Identical state machine and timing to physical devices
4. **Developer-Friendly**: TUI for interactive testing and debugging
5. **Flexible Modes**: Support both ONLINE (server validation) and OFFLINE (local validation) operation

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        TUI Layer (ratatui)                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ Display LCD  │  │   Keypad     │  │    Logs Panel        │   │
│  │ (40×2 chars) │  │  (0-9,*,#)   │  │  (real-time events)  │   │
│  └──────────────┘  └──────────────┘  └──────────────────────┘   │
└────────────────────────────┬────────────────────────────────────┘
                             │ User Input / Display Output
                             ↓
┌─────────────────────────────────────────────────────────────────┐
│                       Emulator Core                             │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │               State Machine                              │   │
│  │  IDLE → READING → VALIDATING → GRANTED/DENIED →          │   │
│  │        → WAITING_ROTATION → ROTATING → IDLE              │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │    Config    │  │   Events     │  │   Logging    │           │
│  │   Manager    │  │   Manager    │  │   System     │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└────────────┬─────────────────────────────┬──────────────────────┘
             │                             │
             ↓                             ↓
┌────────────────────────┐    ┌───────────────────────────┐
│   Protocol Layer       │    │    Storage Layer          │
│  (turnkey-protocol)    │    │  (turnkey-storage)        │
│                        │    │                           │
│  ┌──────────────────┐  │    │  ┌─────────────────────┐  │
│  │ Message Parser   │  │    │  │   SQLite Database   │  │
│  │ Message Builder  │  │    │  │  - users            │  │
│  │ Command Codes    │  │    │  │  - cards            │  │
│  │ Validation       │  │    │  │  - access_logs      │  │
│  └──────────────────┘  │    │  │  - biometrics       │  │
└──────────┬─────────────┘    │  └─────────────────────┘  │
           │                  └───────────────────────────┘
           ↓
┌─────────────────────────────────────────────────────────┐
│              Hardware Abstraction Layer                 │
│           (turnkey-hardware trait)                      │
└────┬────────────┬────────────┬────────────┬─────────────┘
     │            │            │            │
     ↓            ↓            ↓            ↓
┌─────────┐  ┌─────────┐  ┌──────────┐  ┌──────────┐
│  RFID   │  │Biometric│  │ Keypad   │  │Turnstile │
│ Reader  │  │ Scanner │  │          │  │Controller│
├─────────┤  ├─────────┤  ├──────────┤  ├──────────┤
│ Mock    │  │ Mock    │  │ Mock     │  │ Mock     │
│ ACR122U │  │Control  │  │ USB HID  │  │ GPIO     │
│ RC522   │  │iD iDBio │  │ Wiegand  │  │ Relay    │
└─────────┘  └─────────┘  └──────────┘  └──────────┘
```

## Component Responsibilities

### TUI Layer (`turnkey-cli/src/tui/`)
- **Display Management**: Render 2-line × 40-column LCD
- **Input Handling**: Capture keyboard events (numeric keys, function keys)
- **Event Logging**: Real-time event display in logs panel
- **Status Monitoring**: Show reader status, connection state, event count

**Key Files:**
- `display.rs` - LCD display widget
- `keypad.rs` - Numeric keypad widget
- `logs.rs` - Logs panel widget
- `app.rs` - Main TUI application state

### Emulator Core (`turnkey-emulator/src/`)
- **State Machine**: Manage turnstile states and transitions
- **Event Orchestration**: Coordinate between peripherals, protocol, and storage
- **Configuration**: Load and apply settings from TOML files
- **Timing**: Handle timeouts for validation, rotation, display messages

**State Machine:**
```rust
pub enum TurnstileState {
    Idle,                    // Waiting for input
    Reading,                 // Reading card/code/fingerprint
    Validating,              // Sending to server or local DB
    GrantedEntry,            // Access granted for entry
    GrantedExit,             // Access granted for exit
    Denied,                  // Access denied
    WaitingRotation,         // Waiting for physical rotation
    Rotating,                // Rotation in progress
    RotationCompleted,       // Rotation finished
    RotationTimeout,         // User didn't pass through
    Error(String),           // Error state
}
```

**Transitions:**
- `Idle` → `Reading` (user interaction detected)
- `Reading` → `Validating` (credential captured)
- `Validating` → `GrantedEntry|GrantedExit|Denied` (validation response)
- `Granted*` → `WaitingRotation` (auto-transition after display time)
- `WaitingRotation` → `Rotating` (rotation started)
- `Rotating` → `RotationCompleted` (rotation finished)
- `RotationCompleted|Denied|Error` → `Idle` (reset after timeout)

### Protocol Layer (`turnkey-protocol/`)
- **Message Parsing**: Parse incoming Henry protocol messages
- **Message Building**: Construct protocol-compliant messages
- **Command Handling**: Execute command-specific logic
- **Validation**: Ensure message format correctness

**Message Flow:**
```
Emulator → Builder → TCP → External Server
External Server → TCP → Parser → Emulator
```

### Storage Layer (`turnkey-storage/`)
- **Repository Pattern**: Abstract database operations
- **Migrations**: SQLite schema management
- **Query Optimization**: Indexed lookups for fast validation
- **Event Logging**: Persistent access log storage

**Key Tables:**
- `users` - User credentials (code, name, active, valid_from, valid_until)
- `cards` - RFID card associations
- `biometric_templates` - Fingerprint data
- `access_logs` - Complete access history
- `device_config` - Runtime configuration cache

### Hardware Layer (`turnkey-hardware/`, `turnkey-rfid/`, etc.)
- **Trait Definition**: Common `HardwareDevice` interface
- **Mock Implementations**: Software-only device simulation
- **Real Drivers**: Integration with physical hardware
- **Hot-Plugging**: Runtime device discovery and initialization

## Boot Sequence

```
1. Load Configuration
   ├─ Read config/default.toml
   ├─ Parse device settings (ID, model, display message)
   ├─ Parse mode settings (online/offline, status reporting)
   ├─ Parse network settings (IP, port, TCP mode)
   └─ Parse reader settings (enabled peripherals)

2. Initialize Storage
   ├─ Open SQLite connection
   ├─ Run pending migrations
   ├─ Load device state from database
   └─ Verify database integrity

3. Initialize Peripherals
   ├─ For each enabled reader:
   │   ├─ Check config (mock vs real)
   │   ├─ If mock: Instantiate mock device
   │   ├─ If real: Discover and connect to hardware
   │   └─ Register event handlers
   └─ Report initialization status

4. Initialize Network (if ONLINE mode)
   ├─ If server mode: Bind to configured IP:port
   ├─ If client mode: Connect to server
   └─ Start protocol handler task

5. Initialize TUI
   ├─ Setup terminal (alternate screen, raw mode)
   ├─ Create display, keypad, logs widgets
   ├─ Render initial state
   └─ Start event loop

6. Enter IDLE State
   └─ Display configured welcome message
```

## Data Flow Examples

### ONLINE Mode: Successful Access

```
User → Keypad: "1234" + ENTER
  ↓
TUI → Emulator: InputEvent::CodeEntered("1234")
  ↓
Emulator: State::Idle → State::Validating
  ↓
Emulator → Protocol: Build access request message
  Protocol: 01+REON+000+0]1234]20/10/2025 14:30:00]1]0]
  ↓
Protocol → Network: Send TCP message
  ↓
Network → External Server: TCP transmission
  ↓ (wait max 3000ms)
External Server → Network: 01+REON+00+6]5]Acesso liberado]
  ↓
Network → Protocol: Receive TCP message
  ↓
Protocol → Emulator: Parse grant response
  ↓
Emulator: State::Validating → State::GrantedExit
  ↓
Emulator → TUI: DisplayEvent::ShowMessage("Acesso liberado", 5s)
  ↓
Emulator → Storage: Log access event
  ↓
(after 5s) Emulator: State::GrantedExit → State::WaitingRotation
  ↓
Emulator → Protocol: Build waiting rotation message
  Protocol: 01+REON+000+80]]20/10/2025 14:30:05]0]0]
  ↓
Protocol → Network: Send TCP message
  ↓
(simulate rotation after 2s)
Emulator: State::WaitingRotation → State::RotationCompleted
  ↓
Emulator → Protocol: Build rotation complete message
  Protocol: 01+REON+000+81]]20/10/2025 14:30:07]1]0]
  ↓
Protocol → Network: Send TCP message
  ↓
Emulator: State::RotationCompleted → State::Idle
  ↓
Emulator → TUI: DisplayEvent::ShowWelcome()
```

### OFFLINE Mode: Local Validation

```
User → Keypad: "1234" + ENTER
  ↓
TUI → Emulator: InputEvent::CodeEntered("1234")
  ↓
Emulator: State::Idle → State::Validating
  ↓
Emulator → Storage: Query user by code
  Storage: SELECT * FROM users WHERE code='1234' AND active=1
  ↓
Storage → Emulator: User{name: "João", valid: true}
  ↓
Emulator: State::Validating → State::GrantedEntry
  ↓
Emulator → TUI: DisplayEvent::ShowMessage("Acesso liberado", 3s)
  ↓
Emulator → Storage: INSERT INTO access_logs (...)
  ↓
(after 3s) Emulator: State::GrantedEntry → State::WaitingRotation
  ↓
(simulate rotation after 2s)
Emulator: State::WaitingRotation → State::RotationCompleted
  ↓
Emulator → Storage: UPDATE access_logs SET rotation_complete=1
  ↓
Emulator: State::RotationCompleted → State::Idle
  ↓
Emulator → TUI: DisplayEvent::ShowWelcome()
```

## Configuration Management

Configuration is loaded from TOML files at boot and can be reloaded without restart.

**Priority Order:**
1. Environment variables (highest priority)
2. `config/local.toml` (gitignored, local overrides)
3. `config/default.toml` (default values)

**Hot Reload:**
- Send SIGHUP signal to reload configuration
- TUI shortcut: F5 key
- Only non-critical settings are reloaded (no network restart)

## Error Handling

### Network Errors
- **Connection Lost**: Switch to offline mode if configured (`mode.fallback_offline = true`)
- **Timeout**: Display timeout message, return to IDLE after 5s
- **Protocol Error**: Log error, display generic message, return to IDLE

### Hardware Errors
- **Device Not Found**: Use mock device as fallback
- **Read Error**: Retry 3 times, then disable reader and notify user
- **Communication Error**: Log and continue with available readers

### Storage Errors
- **Database Locked**: Retry with exponential backoff (max 5 attempts)
- **Disk Full**: Disable event logging, display warning, continue operation
- **Corruption**: Attempt recovery, fallback to in-memory storage

## Performance Considerations

### Memory Usage
- **Base**: ~30MB (emulator core + TUI)
- **With Events**: +10KB per 1000 events
- **With Templates**: +50KB per 100 biometric templates
- **Peak**: <100MB under normal operation

### CPU Usage
- **Idle**: <1% CPU
- **Active Validation**: <5% CPU (single core)
- **TUI Rendering**: <3% CPU @ 60 FPS

### Network
- **Message Size**: 100-500 bytes average
- **Throughput**: 100+ messages/second
- **Latency**: <10ms local processing + network RTT

## Testing Strategy

### Unit Tests
- Each component tested in isolation
- Mock implementations for dependencies
- Focus on state transitions and edge cases

### Integration Tests
- End-to-end flow testing (input → validation → output)
- Protocol message round-trips
- Database persistence verification

### Mock Testing
- All features testable without hardware
- Simulate card reads, fingerprint scans, etc.
- Configurable delays for realistic timing

### Hardware Tests
- Require physical devices
- Verify driver implementations
- Performance benchmarking

## Future Enhancements

1. **Web Interface**: Alternative to TUI for remote management
2. **Multi-Device**: Emulate multiple turnstiles simultaneously
3. **Recording/Playback**: Capture and replay access sequences
4. **API Mode**: Headless operation for CI/CD testing

## References

- [Emulator Configuration](emulator-configuration.md) - Complete configuration reference
- [Emulator Modes](emulator-modes.md) - ONLINE vs OFFLINE detailed documentation
- [TUI Specification](tui-specification.md) - Terminal interface design
- [Henry Protocol Guide](turnkey-protocol-guide-en.md) - Protocol specification

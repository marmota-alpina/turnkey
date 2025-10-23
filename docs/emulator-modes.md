# Turnkey Emulator Operation Modes

## Overview

The Turnkey emulator supports two primary operation modes that determine how access validation is performed:

1. **ONLINE Mode** - Validation via external TCP client (primary use case)
2. **OFFLINE Mode** - Local validation via SQLite database (standalone operation)

The mode is configured in `config/default.toml`:

```toml
[mode]
online = true              # true = ONLINE, false = OFFLINE
fallback_offline = true    # Fallback to OFFLINE if server unreachable
```

---

## ONLINE Mode

### Overview

In ONLINE mode, the emulator acts as a **physical turnstile**, sending access requests to an external TCP client which applies business logic and responds with grant/deny decisions.

**Key Characteristics:**
- Emulator has no validation logic (mimics real hardware)
- TCP client makes all access decisions
- Real-time event and status reporting
- Timeout and fallback handling

### Configuration

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

### Connection Modes

#### Server Mode (Default)

Emulator acts as TCP **server**, waiting for client connections.

```toml
[network]
tcp_mode = "server"
ip = "192.168.0.100"       # Bind address
port = 3000                # Listen port
```

**Behavior:**
```
Boot → Bind to 192.168.0.100:3000 → Wait for client
Client connects → Accept connection → Ready for requests
```

**Use Case:** Testing access control software that connects to turnstiles

#### Client Mode

Emulator acts as TCP **client**, connecting to a server.

```toml
[network]
tcp_mode = "client"
server_address = "192.168.0.1"  # Server IP
server_port = 3000               # Server port
```

**Behavior:**
```
Boot → Connect to 192.168.0.1:3000
Connected → Ready for requests
Connection lost → Retry (max 3 attempts)
```

**Use Case:** Centralized server managing multiple emulators

---

### Access Flow (Card Read Example)

```
┌──────────┐          ┌──────────┐          ┌──────────┐
│   User   │          │ Emulator │          │  Client  │
└────┬─────┘          └────┬─────┘          └────┬─────┘
     │                     │                     │
     │  Swipes card        │                     │
     │  "12345678"         │                     │
     ├────────────────────>│                     │
     │                     │                     │
     │                     │ Access Request      │
     │                     │ (000+0)             │
     │                     ├────────────────────>│
     │                     │                     │
     │                     │                     │ Apply business
     │                     │                     │ logic (validate
     │                     │                     │ card, check
     │                     │                     │ schedule, etc)
     │                     │                     │
     │                     │ Grant Exit (00+6)   │
     │                     │<────────────────────┤
     │                     │                     │
     │   Display:          │                     │
     │   "Acesso liberado" │                     │
     │<────────────────────┤                     │
     │                     │                     │
     │                     │ Waiting Rotation    │
     │                     │ (000+80)            │
     │                     ├────────────────────>│
     │                     │                     │
     │   (simulate         │                     │
     │    rotation)        │                     │
     │<───────────────────-┤                     │
     │                     │                     │
     │                     │ Rotation Complete   │
     │                     │ (000+81)            │
     │                     ├────────────────────>│
     │                     │                     │
     │   Display:          │                     │
     │ "DIGITE SEU CÓDIGO" │                     │
     │<────────────────────┤                     │
```

### Detailed Message Flow

#### 1. Access Request (Turnstile → Client)

**User Action:** Swipes card `12345678` on entry side

**Emulator Sends:**
```
01+REON+000+0]12345678]20/10/2025 14:30:00]1]0]
```

**Message Breakdown:**
- `01` - Device ID
- `REON` - Protocol identifier
- `000+0` - Access request command
- `12345678` - Card number
- `20/10/2025 14:30:00` - Timestamp (dd/mm/yyyy hh:mm:ss)
- `1` - Direction (1 = entry, 2 = exit)
- `0` - Reader type (0 = card reader, 1 = biometric, 2 = keypad)

#### 2a. Grant Response (Client → Turnstile)

**Client Sends (Access Granted):**
```
01+REON+00+6]5]Acesso liberado]
```

**Message Breakdown:**
- `01` - Device ID (echo)
- `REON` - Protocol identifier
- `00+6` - Grant exit access command
- `5` - Seconds to display message
- `Acesso liberado` - Display message (max 40 chars)

**Alternative Grant Commands:**
- `00+1` - Grant both directions
- `00+5` - Grant entry only
- `00+6` - Grant exit only

#### 2b. Deny Response (Client → Turnstile)

**Client Sends (Access Denied):**
```
01+REON+00+30]0]Acesso negado]
```

**Message Breakdown:**
- `00+30` - Deny access command
- `0` - No display duration (permanent until next action)
- `Acesso negado` - Display message

**Emulator Behavior:**
- Display "Acesso negado"
- Play error beep
- Return to IDLE after 5 seconds
- Log denied access event

#### 3. Waiting for Rotation (Turnstile → Client)

**Emulator Sends (After Grant):**
```
01+REON+000+80]]20/10/2025 14:30:05]0]0]
```

**Message Breakdown:**
- `000+80` - Waiting for rotation command
- Empty field (no card number)
- `20/10/2025 14:30:05` - Current timestamp
- `0` - Direction not applicable
- `0` - Reader type not applicable

**Emulator Behavior:**
- Internal state: WAITING_ROTATION
- Starts rotation simulation timer (default: 2 seconds)

#### 4. Rotation Complete (Turnstile → Client)

**Emulator Sends (After Rotation):**
```
01+REON+000+81]]20/10/2025 14:30:07]1]0]
```

**Message Breakdown:**
- `000+81` - Rotation completed command
- Empty field (no card number)
- `20/10/2025 14:30:07` - Completion timestamp
- `1` - Direction user passed (1 = entry, 2 = exit)
- `0` - Reader type

**Emulator Behavior:**
- Internal state: ROTATION_COMPLETED → IDLE
- Display welcome message
- Ready for next access

#### 5. Rotation Timeout (Turnstile → Client)

**Emulator Sends (If User Doesn't Pass):**
```
01+REON+000+82]]20/10/2025 14:30:10]0]0]
```

**Message Breakdown:**
- `000+82` - Rotation timeout/cancelled command
- User didn't pass through turnstile within timeout

**Timeout Configuration:**
```toml
[turnstile]
rotation_timeout = 10  # seconds
```

---

### Status Reporting (Heartbeat)

When `status_online = true`, emulator sends periodic status updates:

**Emulator Sends (Every 60 seconds):**
```
01+REON+RQ+0]
```

**Client Responds:**
```
01+REON+RQ+1]OK]
```

**Status Information Includes:**
- Device online/offline
- Reader status (enabled/disabled/error)
- Event count
- Last access timestamp
- Memory usage

---

### Timeout Handling

#### Validation Timeout

**Configuration:**
```toml
[mode]
fallback_offline = true
fallback_timeout = 3000  # milliseconds
```

**Scenario:** Client doesn't respond within 3000ms

**Behavior:**
1. If `fallback_offline = true`:
   - Switch to OFFLINE mode
   - Query local SQLite database
   - Grant/deny based on local data
   - Display "MODO OFFLINE"

2. If `fallback_offline = false`:
   - Display "ERRO: SERVIDOR"
   - Deny access
   - Return to IDLE
   - Log timeout event

#### Connection Lost

**Scenario:** TCP connection drops during operation

**Behavior:**
1. Attempt reconnection (3 retries, 5s delay)
2. If reconnection fails:
   - Switch to OFFLINE mode (if `fallback_offline = true`)
   - Display "CONEXÃO PERDIDA - OFFLINE"
3. When connection restored:
   - Sync pending events to server
   - Return to ONLINE mode

---

## OFFLINE Mode

### Overview

In OFFLINE mode, the emulator performs **local validation** using a SQLite database. This enables standalone operation without external dependencies.

**Key Characteristics:**
- All validation logic in emulator
- No network requirements
- Local user/card database
- Suitable for testing and development

### Configuration

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

### Database Schema

#### users Table

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code VARCHAR(20) UNIQUE NOT NULL,      -- Access code
    name VARCHAR(100) NOT NULL,             -- User name
    pis VARCHAR(11),                        -- Brazilian PIS number
    reference VARCHAR(20),                  -- Employee reference (matricula)
    active BOOLEAN DEFAULT 1,               -- Enabled/disabled
    valid_from DATETIME,                    -- Validity start date
    valid_until DATETIME,                   -- Validity end date
    allow_card BOOLEAN DEFAULT 1,           -- Allow card access
    allow_biometric BOOLEAN DEFAULT 0,      -- Allow biometric access
    allow_keypad BOOLEAN DEFAULT 1,         -- Allow keypad access
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_code ON users(code);
CREATE INDEX idx_users_active ON users(active);
```

#### cards Table

```sql
CREATE TABLE cards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    card_number VARCHAR(20) UNIQUE NOT NULL,
    card_type VARCHAR(10) DEFAULT 'RFID',  -- RFID, MIFARE
    active BOOLEAN DEFAULT 1,
    valid_from DATETIME,
    valid_until DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_cards_number ON cards(card_number);
CREATE INDEX idx_cards_user ON cards(user_id);
```

#### biometric_templates Table

```sql
CREATE TABLE biometric_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    finger_index INTEGER NOT NULL,          -- 0-9 (left/right hand, 5 fingers)
    template_data BLOB NOT NULL,            -- Fingerprint template
    quality INTEGER,                        -- 0-100
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(user_id, finger_index)
);

CREATE INDEX idx_bio_user ON biometric_templates(user_id);
```

#### access_logs Table

```sql
CREATE TABLE access_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER,
    card_number VARCHAR(20),
    biometric_used BOOLEAN DEFAULT 0,
    access_granted BOOLEAN NOT NULL,
    direction INTEGER,                      -- 1 = entry, 2 = exit
    reader_type INTEGER,                    -- 0 = card, 1 = biometric, 2 = keypad
    event_timestamp DATETIME NOT NULL,
    rotation_complete BOOLEAN DEFAULT 0,
    rotation_timestamp DATETIME,
    deny_reason VARCHAR(100),               -- NULL if granted
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX idx_logs_timestamp ON access_logs(event_timestamp DESC);
CREATE INDEX idx_logs_user ON access_logs(user_id);
```

---

### Access Flow (Code Entry Example)

```
┌──────────┐          ┌──────────┐          ┌──────────┐
│   User   │          │ Emulator │          │ SQLite   │
└────┬─────┘          └────┬─────┘          └────┬─────┘
     │                     │                     │
     │  Enters code        │                     │
     │  "1234" + ENTER     │                     │
     ├────────────────────>│                     │
     │                     │                     │
     │                     │ SELECT * FROM users │
     │                     │ WHERE code='1234'   │
     │                     ├────────────────────>│
     │                     │                     │
     │                     │<────────────────────┤
     │                     │ User found, active  │
     │                     │                     │
     │   Display:          │                     │
     │   "Acesso liberado" │                     │
     │<────────────────────┤                     │
     │                     │                     │
     │   (simulate         │                     │
     │    rotation)        │                     │
     │<────────────────────┤                     │
     │                     │                     │
     │                     │ INSERT INTO         │
     │                     │ access_logs (...)   │
     │                     ├────────────────────>│
     │                     │                     │
     │   Display:          │                     │
     │ "DIGITE SEU CÓDIGO" │                     │
     │<────────────────────┤                     │
```

### Validation Logic

#### Step 1: User Lookup

```sql
SELECT
    id, name, pis, active,
    valid_from, valid_until,
    allow_card, allow_biometric, allow_keypad
FROM users
WHERE code = ?
  AND active = 1
LIMIT 1;
```

#### Step 2: Validity Check

```rust
fn is_user_valid(user: &User, now: DateTime) -> bool {
    // Check if user is active
    if !user.active {
        return false;
    }

    // Check validity period
    if let Some(valid_from) = user.valid_from {
        if now < valid_from {
            return false;  // Not yet valid
        }
    }

    if let Some(valid_until) = user.valid_until {
        if now > valid_until {
            return false;  // Expired
        }
    }

    true
}
```

#### Step 3: Access Type Check

```rust
fn check_access_method(user: &User, method: AccessMethod) -> bool {
    match method {
        AccessMethod::Card => user.allow_card,
        AccessMethod::Biometric => user.allow_biometric,
        AccessMethod::Keypad => user.allow_keypad,
    }
}
```

#### Step 4: Grant or Deny

```rust
if is_user_valid(&user, Utc::now()) && check_access_method(&user, method) {
    // Grant access
    display_message("Acesso liberado", 3);
    log_access(user.id, true, None);
    simulate_rotation();
} else {
    // Deny access
    let reason = if !is_user_valid(&user, Utc::now()) {
        "Usuário inválido"
    } else {
        "Método não permitido"
    };
    display_message(reason, 5);
    log_access(user.id, false, Some(reason));
    play_error_beep();
}
```

---

### Card Access Flow

**User Action:** Swipes card `87654321`

**Database Query:**
```sql
SELECT u.id, u.name, u.active, u.valid_from, u.valid_until, u.allow_card
FROM users u
JOIN cards c ON c.user_id = u.id
WHERE c.card_number = '87654321'
  AND c.active = 1
  AND u.active = 1
LIMIT 1;
```

**Validation:**
1. Card found and active? → Continue
2. User active? → Continue
3. Within validity period? → Continue
4. `allow_card = true`? → Grant
5. Any check fails → Deny

---

### Biometric Access Flow

**User Action:** Places finger on scanner

**1:1 Verification (After Code Entry):**
```sql
SELECT template_data, quality
FROM biometric_templates
WHERE user_id = ?
  AND finger_index = ?;
```

**1:N Identification (Auto-On Mode):**
```sql
SELECT bt.id, bt.user_id, bt.template_data, u.name, u.active
FROM biometric_templates bt
JOIN users u ON u.id = bt.user_id
WHERE u.active = 1
  AND u.allow_biometric = 1;
```

**Note:** 1:N mode is slow (searches all templates). Use only with < 1000 users.

---

### Event Logging

All access attempts are logged:

```rust
async fn log_access_event(
    user_id: Option<i64>,
    granted: bool,
    direction: Direction,
    reader: ReaderType,
    deny_reason: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO access_logs
         (user_id, access_granted, direction, reader_type,
          event_timestamp, deny_reason)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(granted)
    .bind(direction as i32)
    .bind(reader as i32)
    .bind(Utc::now())
    .bind(deny_reason)
    .execute(&pool)
    .await?;

    Ok(())
}
```

**Example Log Entry (Granted):**
```
id: 12345
user_id: 42
access_granted: 1
direction: 1 (entry)
reader_type: 0 (card)
event_timestamp: 2025-10-20 14:30:00
rotation_complete: 1
rotation_timestamp: 2025-10-20 14:30:02
deny_reason: NULL
```

**Example Log Entry (Denied):**
```
id: 12346
user_id: 43
access_granted: 0
direction: 1 (entry)
reader_type: 2 (keypad)
event_timestamp: 2025-10-20 14:31:00
deny_reason: "Usuário inválido"
```

---

## Mode Comparison

| Feature              | ONLINE Mode           | OFFLINE Mode            |
|----------------------|-----------------------|-------------------------|
| **Validation**       | External TCP client   | Local SQLite            |
| **Network Required** | Yes                   | No                      |
| **Business Logic**   | In client             | In emulator             |
| **User Database**    | Client-side           | Emulator-side           |
| **Performance**      | Network latency       | Instant                 |
| **Scalability**      | High (stateless)      | Limited (local DB)      |
| **Fallback**         | To offline (optional) | N/A                     |
| **Use Case**         | Production testing    | Development, standalone |
| **Complexity**       | Low (emulator)        | High (emulator)         |

---

## Hybrid Mode (Fallback)

Combine ONLINE and OFFLINE for resilience:

```toml
[mode]
online = true
fallback_offline = true
fallback_timeout = 3000
```

**Behavior:**
1. Start in ONLINE mode
2. On validation timeout (3000ms):
   - Switch to OFFLINE mode
   - Query local database
   - Grant/deny locally
   - Display "MODO OFFLINE"
3. Continue in OFFLINE until connection restored
4. When online again:
   - Sync pending events to server
   - Return to ONLINE mode
   - Display "MODO ONLINE"

**Event Sync:**
```rust
async fn sync_offline_events(server: &TcpStream) -> Result<()> {
    let events = load_unsynced_events().await?;

    for event in events {
        send_event_to_server(server, &event).await?;
        mark_event_synced(event.id).await?;
    }

    Ok(())
}
```

---

## Development Recommendations

### Testing ONLINE Mode

1. Use `tcp_mode = "server"` for easier testing
2. Connect with `telnet` or custom client:
   ```bash
   telnet 192.168.0.100 3000
   ```
3. Send grant response manually:
   ```
   01+REON+00+6]5]Acesso liberado]
   ```

### Testing OFFLINE Mode

1. Pre-populate database with test users:
   ```sql
   INSERT INTO users (code, name, active, allow_keypad)
   VALUES ('1234', 'Test User', 1, 1);
   ```
2. Test expiration by setting `valid_until` to past date
3. Test access methods by toggling `allow_*` flags

### Testing Fallback

1. Start in ONLINE mode
2. Don't send response (simulate timeout)
3. Verify fallback to OFFLINE
4. Send response (simulate recovery)
5. Verify return to ONLINE

---

## References

- [Emulator Architecture](emulator-architecture.md) - System design
- [Emulator Configuration](emulator-configuration.md) - Configuration reference
- [Henry Protocol Guide](turnkey-protocol-guide-en.md) - Protocol specification
- [Data Formats](data-formats.md) - Import/export formats

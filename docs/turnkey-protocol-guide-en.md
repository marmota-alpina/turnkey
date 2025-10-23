# Complete Henry Protocol Guide - Equipment Emulator

## 1. System Overview

### 1.1 Supported Equipment
- **Primme Acesso** (versions 1.0.0.23 and 8.0.0.50)
- **Argos**
- **Primme SF (Super Easy)**
- **Turnstiles with RFID and Biometric readers**

### 1.2 Communication Protocol
- TCP/IP Communication
- Message format: `ID+REON+CODE+DATA`
- Field separators: `]`, `[`, `+`, `{`, `}`
- Encoding: ASCII
- General structure: `<SB><XXXX><II>+COMMAND+00+DATA<CS><EB>`

## 2. Turnstile Communication Flow

### 2.1 Complete Flow with Rotation Confirmation

#### Command Sequence:

1. **Turnstile Request**
   ```
   15+REON+000+0]00000000000011912322]10/05/2016 12:46:06]1]0]
   ```
   - `15`: Equipment ID
   - `000+0`: Command code (access request)
   - `00000000000011912322`: Card/enrollment number
   - `10/05/2016 12:46:06`: Event date/time
   - `1`: Direction (1=entry, 2=exit)
   - `0`: Additional indicator

2. **Software Response - Access Granted**
   ```
   15+REON+00+6]5]Access granted]
   ```
   - `00+6`: Code for exit release
   - `5`: Release time in seconds
   - `Access granted`: Display message

3. **Turnstile Response - Waiting for Rotation**
   ```
   15+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```
   - `000+80`: Code indicating waiting for rotation
   - Status: Turnstile released waiting for user to rotate

4. **Rotation Simulation/Detection**
   - Sensor detects turnstile arm movement
   - User initiates physical rotation

5. **Turnstile Response - Rotation Completed**
   ```
   15+REON+000+81]]11/05/2016 14:33:24]2]0]
   ```
   - `000+81`: Rotation completed code
   - `2`: Direction of completed rotation

### 2.2 Flow with Rotation Abandonment

1. **Turnstile Request** (identical to previous flow)
   ```
   15+REON+000+0]00000000000011912322]10/05/2016 12:46:06]1]0]
   ```

2. **Software Response - Access Granted**
   ```
   15+REON+00+6]5]Access granted]
   ```

3. **Turnstile Response - Waiting for Rotation**
   ```
   15+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```

4. **Turnstile Response - Rotation Abandonment**
   ```
   15+REON+000+82]]11/05/2016 15:26:03]0]0]
   ```
   - `000+82`: Abandonment code (timeout expired)

5. **Software Releases Again (Manual)**
   ```
   01+REON+00+4]5]Access granted]
   ```
   - `00+4`: Manual release

6. **Turnstile Waits Again**
   ```
   01+REON+000+80]]10/05/2016 12:46:06]0]0]
   ```

7. **Rotation Completed**
   ```
   08+REON+000+81]]11/05/2016 15:26:03]0]0]
   ```

## 3. Command and Response Codes

### 3.1 Release Codes

| Code | Description | Usage |
|------|-------------|-------|
| `00+1` | Release both sides | Bidirectional access |
| `00+5` | Release entry | Entry access |
| `00+6` | Release exit | Exit access |
| `00+4` | Manual release | Forced release by software |
| `00+30` | Access denied | Access block |

### 3.2 Turnstile Status Codes

| Code | Description | Meaning |
|------|-------------|---------|
| `000+0` | Request | Turnstile requests validation |
| `000+80` | Waiting for rotation | Turnstile released waiting for movement |
| `000+81` | Rotation completed | Passage successfully completed |
| `000+82` | Abandonment | Timeout without rotation |

### 3.3 Reader Type Identification

The last field of the request command indicates the type of reader used:

| Value | Reader Type |
|-------|------------|
| `1` | RFID proximity reader |
| `5` | Biometric reader |

## 4. Command Data Format

### 4.1 Online Validation Structure

#### Equipment Request:
```
ID+REON+000+0]ENROLLMENT]DATE_TIME]DIRECTION]INDICATOR]READER_TYPE
```

#### Software Response:
```
ID+REON+00+RELEASE_CODE]TIME]MESSAGE]
```

Where:
- `ID`: Equipment identifier (01-99)
- `ENROLLMENT`: Card or enrollment number (up to 20 digits)
- `DATE_TIME`: Format dd/mm/yyyy hh:mm:ss
- `DIRECTION`: 1=entry, 2=exit, 0=undefined
- `TIME`: Release time in seconds
- `MESSAGE`: Text to display (max 40 characters)

### 4.2 Complete Communication Example

```
// RFID card presented
Turnstile → Software: 01+REON+00+0]12651543]22/08/2011 08:57:01]1]0]1

// Software grants access
Software → Turnstile: 01+REON+00+1]5]Access granted]

// Turnstile confirms release
Turnstile → Software: 01+REON+000+80]]22/08/2011 08:57:01]0]0]

// User passes through turnstile
Turnstile → Software: 01+REON+000+81]]22/08/2011 08:57:02]1]0]
```

## 5. Management Commands

### 5.1 Main Commands

| Code | Name | Description | Primme | Argos | Primme SF |
|------|------|-------------|--------|-------|-----------|
| EC | Settings | Send settings to equipment | ✓ | ✓ | ✓ |
| EE | Employer | Send employer to equipment | ✓ | ✗ | ✗ |
| EU | User | Send user list | ✓ | ✗ | ✗ |
| EH | Date and time | Send date and time to equipment | ✓ | ✓ | ✓ |
| ED | Fingerprints | Send fingerprint list | ✓ | ✓ | ✓ |
| ER | Records | Receive records | ✓ | ✓ | ✓ |
| ECAR | Card | Send card list | ✓ | ✓ | ✓ |
| EACI | Trigger | Send trigger list | ✓ | ✗ | ✗ |
| EPER | Periods | Send period list | ✓ | ✗ | ✗ |
| EHOR | Schedules | Send schedule list | ✓ | ✗ | ✗ |
| EFER | Holidays | Send holiday list | ✓ | ✗ | ✗ |
| EMSG | Messages | Send default messages | ✓ | ✗ | ✗ |
| EGA | Access Group | Send access groups | ✓ | ✗ | ✗ |
| ECGA | Access Group Cards | Send access group cards | ✓ | ✗ | ✗ |
| EFUN | Functions | Send functions | ✓ | ✗ | ✗ |

### 5.2 Reception Commands

| Code | Name | Description | Primme | Argos | Primme SF |
|------|------|-------------|--------|-------|-----------|
| RC | Settings | Receive equipment settings | ✓ | ✓ | ✓ |
| RE | Employer | Receive equipment employer | ✓ | ✗ | ✗ |
| RQ | Quantity/Status | Receive quantities and status | ✓ | ✓ | ✓ |
| RU | User | Receive user list | ✓ | ✗ | ✗ |
| RH | Date/Time | Receive current date/time | ✓ | ✓ | ✓ |
| RR | Access Logs | Receive access logs | ✓ | ✓ | ✓ |
| RD | Biometric List | Receive biometric template list | ✓ | ✓ | ✓ |
| RCAR | Card | Receive card list | ✓ | ✓ | ✓ |
| RGA | Access Group | Receive access groups | ✓ | ✗ | ✗ |
| RCGA | Access Group Cards | Receive access group cards | ✓ | ✗ | ✗ |
| RACI | Relay Trigger | Receive relay trigger list | ✓ | ✗ | ✗ |
| RPER | Time Period | Receive time period list | ✓ | ✗ | ✗ |
| RHOR | Schedule | Receive schedule list | ✓ | ✗ | ✗ |
| RFER | Holiday | Receive holiday list | ✓ | ✗ | ✗ |
| RMSG | Messages | Receive default messages | ✓ | ✗ | ✗ |

## 6. Equipment Configuration

### 6.1 Main Parameters

#### General
- `NR_EQUIP`: Equipment number (0-4294967295)
- `VOLUME`: Sound alert volume (2-9, default: 9)
- `MSG_DISPLAY`: Display message (up to 40 characters)
- `GER_INTELIGENTE`: Smart management (H/D)
- `SENHA_MENU`: Menu password (9 digits Primme, 6 digits Argos)
- `LOGIN`: Web access user (up to 20 characters)

#### Validation and Access
- `TIPO_VALIDA`: Validation type
  - `F`: Offline
  - `O`: Online
  - `A`: Automatic
  - `S`: Semi-automatic
- `ARMAZENA_REGISTRO`: Records stored (T/N/G/L)
- `TIMEOUT_ON`: Online timeout (500-10000ms, default: 3000)
- `ESPERA_OFF`: Offline wait time (2-600s, default: 60)
- `TEMPO_PASSBACK`: Anti-passback in minutes (0-999999)
- `DIRECAO_PASSBACK`: Anti-passback direction (H/D)
- `VERIF_VALIDADE`: Verify card validity (H/D)
- `ACESSO_USUARIO`: User access type (B/V/L)

#### Readers
- `LEITOR_1`, `LEITOR_2`, `LEITOR_3`: Reader configuration
- `LEITOR_VER_DIG`: Request biometry when reading card (H/D)
- `MODO_CADASTRO`: Automatic enrollment (A/N)

## 7. Data Sending Protocol

### 7.1 Card Sending Structure

```
<SB><XXXX><II>+ECAR+00+QTY+OPERATION[INDEX[CARD[VALIDITY_START[VALIDITY_END[CODE[TYPE[VERIFY_FINGER[PASSWORD[PANIC_PASSWORD[RELAYS[SEQUENCE[POSITION[QTY_SCHEDULES[SCHEDULES[QTY_SHIFTS[SHIFTS[SECURE_PASSWORD<CS><EB>
```

Where:
- `QTY`: Card quantity
- `OPERATION`: I=Insert, E=Delete, A=Update, L=Clear list
- `INDEX`: User index
- `CARD`: Card number (3-20 characters)
- `VALIDITY_START/END`: dd/mm/yyyy hh:mm:ss
- `TYPE`: Card type
- `VERIFY_FINGER`: H/D for fingerprint verification

### 7.2 User Sending Structure

```
<SB><XXXX><II>+EU+00+QTY+OPERATION[INDEX[NAME[RESERVED[QTY_REF[CARDS<CS><EB>
```

Example:
```
+EU+00+1+I[1001[John Smith[0[2[12345}67890
```

## 8. Biometry Protocol

### 8.1 Fingerprint Addition

```
<SB><XXXX><II>+ED+00+D]ENROLLMENT}QTY_TEMPLATES}FINGER_NUM{TEMPLATE<CS><EB>
```

### 8.2 Fingerprint Deletion

```
<SB><XXXX><II>+ED+00+E]ENROLLMENT<CS><EB>
```

### 8.3 Clear All Fingerprints

```
<SB><XXXX><II>+ED+00+C]<CS><EB>
```

## 9. Event and Record Protocol

The Henry protocol supports multiple methods for retrieving access logs from devices. The command code is `RR` (for Primme SF/Argos) or `ER` (for Primme Acesso).

### 9.1 Record Collection Methods

The protocol provides **5 filter modes** for retrieving access logs, each optimized for different use cases:

#### 9.1.1 Filter by Memory Address (M)

Retrieve events from a specific memory address location.

**Command Structure**:
```
<SB><XXXX><II>+RR+00+M]QUANTITY]START_ADDRESS<CS><EB>
```

**Fields**:
- `QUANTITY`: Number of events to retrieve
- `START_ADDRESS`: Memory address to start from (0-based)

**Example**:
```
01+RR+00+M]3]0
```
Retrieves 3 events starting from memory address 0

**Use Case**: Low-level memory access, useful for debugging or complete memory scans.

#### 9.1.2 Filter by NSR (N)

Retrieve events by sequential record number (NSR - Numero Sequencial de Registro).

**Command Structure**:
```
<SB><XXXX><II>+RR+00+N]QUANTITY]START_NSR<CS><EB>
```

**Fields**:
- `QUANTITY`: Number of events to retrieve
- `START_NSR`: Starting NSR number (sequential ID)

**Example**:
```
01+RR+00+N]5]1
```
Retrieves 5 events starting from NSR 1

**Use Case**: Sequential collection with guaranteed order, ideal for batch processing.

#### 9.1.3 Filter by Date Range (D)

Retrieve events within a specific date/time range.

**Command Structure**:
```
<SB><XXXX><II>+RR+00+D]QUANTITY]START_DATETIME]END_DATETIME<CS><EB>
```

**Fields**:
- `QUANTITY`: Number of events to retrieve
- `START_DATETIME`: Start date/time (dd/mm/yyyy HH:MM:SS)
- `END_DATETIME`: End date/time (optional for Primme SF, required for Primme Acesso)

**Example (Primme SF)**:
```
01+RR+00+D]2]10/07/2012 08:00:01]
```
Retrieves 2 events after July 10, 2012 08:00:01

**Example (Primme Acesso)**:
```
01+ER+00+D]10]01/01/2024 00:00:00]31/01/2024 23:59:59
```
Retrieves 10 events between January 1-31, 2024

**Use Case**: Historical analysis, compliance reporting, specific incident investigation.

#### 9.1.4 Filter by Index (T)

Retrieve events by their sequential index position.

**Command Structure**:
```
<SB><XXXX><II>+RR+00+T]QUANTITY]START_INDEX<CS><EB>
```

**Fields**:
- `QUANTITY`: Number of events to retrieve
- `START_INDEX`: Starting index (1-based)

**Example**:
```
01+RR+00+T]5]1
```
Retrieves 5 events starting from index 1

**Use Case**: Pagination, sequential retrieval with known index positions.

#### 9.1.5 Filter by Uncollected (C)

Retrieve only events that have not yet been collected by the management software.

**Command Structure**:
```
<SB><XXXX><II>+RR+00+C]QUANTITY]START_INDEX<CS><EB>
```

**Fields**:
- `QUANTITY`: Number of uncollected events to retrieve
- `START_INDEX`: Starting index (0-based)

**Example**:
```
01+RR+00+C]5]0
```
Retrieves 5 uncollected events starting from index 0

**Use Case**: Incremental synchronization, ensuring no events are missed during network outages.

**Note**: The device internally marks events as "collected" after successful retrieval. Use this mode for reliable synchronization.

### 9.2 Collection Confirmation

After successfully retrieving events, the client must send a confirmation to mark them as collected.

**Command Structure**:
```
<SB><XXXX><II>+ER+00+QTY_COLLECTED+INDICES]<CS><EB>
```

**Fields**:
- `QTY_COLLECTED`: Number of events successfully collected
- `INDICES`: Comma-separated list of collected event indices

**Example**:
```
01+ER+00+5+1,2,3,4,5]
```
Confirms collection of 5 events with indices 1 through 5

### 9.3 Event Record Format

Each event record returned by the device contains:

**Standard Fields**:
- NSR (Sequential Record Number)
- Date/Time (dd/mm/yyyy HH:MM:SS)
- Card/Enrollment Number
- Event Type (access granted, denied, rotation completed, etc.)
- Direction (1=entry, 2=exit, 0=undefined)
- Reader Type (1=RFID, 5=Biometric)
- Additional metadata (varies by device model)

**Example Response**:
```
01+RR+00+3+1}12345}22/08/2011 08:57:01}Access Granted}1}1+2}67890}22/08/2011 09:15:22}Access Denied}2}1+3}11111}22/08/2011 10:30:45}Access Granted}1}5
```

### 9.4 Best Practices for Event Collection

1. **Use Uncollected Mode (C) for Real-time Sync**: Query periodically for uncollected events to maintain up-to-date logs.

2. **Use Date Range (D) for Historical Queries**: When investigating specific time periods or generating reports.

3. **Implement Pagination**: Request manageable batches (e.g., 50-100 events) to avoid network timeouts.

4. **Always Send Confirmation**: Mark events as collected to prevent duplicate processing.

5. **Handle Network Failures**: Implement retry logic with exponential backoff for unreliable connections.

6. **Monitor Uncollected Count**: Regularly query `RQ+00+RNC` to detect collection lag.

## 10. Quantity and Status Query

The `RQ` (Request Query) command allows the management software to query device status and capacity information. The protocol supports **12 distinct query types** for monitoring device health, memory usage, and peripheral status.

### 10.1 Available Query Types

The general command structure is:
```
<SB><XXXX><II>+RQ+00+PARAMETER<CS><EB>
```

#### 10.1.1 User Count (U)

Query the total number of registered users in device memory.

**Command**:
```
01+RQ+00+U
```

**Response Example**:
```
01+RQ+00+U]35
```
Device has 35 registered users.

**Use Case**: Monitor user database size, validate synchronization, check capacity before bulk imports.

#### 10.1.2 Card Count (C)

Query the total number of registered cards (RFID credentials).

**Command**:
```
01+RQ+00+C
```

**Response Example**:
```
01+RQ+00+C]142
```
Device has 142 registered cards.

**Typical Range**: 0-999999999 (varies by device model)

#### 10.1.3 Biometric Count (D)

Query the total number of registered fingerprint templates.

**Command**:
```
01+RQ+00+D
```

**Response Example**:
```
01+RQ+00+D]78
```
Device has 78 fingerprint templates stored.

**Typical Range**: 0-10000 (varies by device model and storage capacity)

#### 10.1.4 Total Biometric Capacity (TD)

Query the maximum number of fingerprint templates the device can store.

**Command**:
```
01+RQ+00+TD
```

**Response Example**:
```
01+RQ+00+TD]3000
```
Device supports up to 3000 fingerprint templates.

**Use Case**: Pre-enrollment capacity checks, planning biometric rollouts.

#### 10.1.5 Record Count (R)

Query the total number of access log events stored in device memory.

**Command**:
```
01+RQ+00+R
```

**Response Example**:
```
01+RQ+00+R]5427
```
Device has 5427 access log events.

**Typical Range**: 0-999999999

**Use Case**: Monitor log storage usage, schedule log collection before memory fills.

#### 10.1.6 Uncollected Record Count (RNC)

Query the number of access logs not yet collected by management software.

**Command**:
```
01+RQ+00+RNC
```

**Response Example**:
```
01+RQ+00+RNC]23
```
Device has 23 uncollected events pending synchronization.

**Use Case**: Critical for incremental sync - indicates how many events are waiting. If this number grows continuously, collection is falling behind.

#### 10.1.7 Device Lock Status (TP)

Query whether the device is administratively locked (blocked from access).

**Command**:
```
01+RQ+00+TP
```

**Response Values**:
- `A`: Device is locked (Blocked)
- `D`: Device is unlocked (Normal operation)

**Response Example**:
```
01+RQ+00+TP]D
```
Device is unlocked and operational.

**Use Case**: Security monitoring, emergency lockdown verification.

#### 10.1.8 MRP Communication Error (MRPE)

Query if there is a communication error with the MRP (printer module).

**Command**:
```
01+RQ+00+MRPE
```

**Response Values**:
- `0`: No error, printer communication OK
- `1`: Communication error detected

**Response Example**:
```
01+RQ+00+MRPE]0
```
Printer module is communicating normally.

**Use Case**: Peripheral diagnostics, printer troubleshooting.

**Note**: Only applicable to devices with integrated thermal printers (e.g., time clocks).

#### 10.1.9 Employer Status (SEMP)

Query if employer information is properly configured on the device.

**Command**:
```
01+RQ+00+SEMP
```

**Response Values**:
- `0`: Employer is registered
- `1`: Employer is NOT registered (configuration incomplete)

**Response Example**:
```
01+RQ+00+SEMP]0
```
Employer information is configured.

**Use Case**: Initial setup validation, compliance checks (Brazilian labor law requires employer registration).

#### 10.1.10 Low Paper Sensor (PP)

Query if the low paper sensor is active (paper roll running low).

**Command**:
```
01+RQ+00+PP
```

**Response Values**:
- `0`: Paper level is adequate
- `1`: Low paper warning active

**Response Example**:
```
01+RQ+00+PP]1
```
Paper is running low, refill soon.

**Use Case**: Proactive maintenance alerts for time clock systems.

#### 10.1.11 No Paper Status (SP)

Query if the device is completely out of paper.

**Command**:
```
01+RQ+00+SP
```

**Response Values**:
- `0`: Paper is available
- `1`: No paper (empty)

**Response Example**:
```
01+RQ+00+SP]0
```
Paper roll is present.

**Use Case**: Critical alerts for time clock functionality, prevent lost records.

#### 10.1.12 Paper Capacity (QP)

Query detailed paper roll capacity information.

**Command**:
```
01+RQ+00+QP
```

**Response Format**:
```
01+RQ+00+QP]TICKET_CAPACITY]CURRENT_SIZE]TOTAL_SIZE
```

**Response Example**:
```
01+RQ+00+QP]500]350]500
```
- Ticket capacity: 500 prints per roll
- Current size: 350 prints remaining
- Total size: 500 prints (full roll)

**Use Case**: Precise paper usage monitoring, predictive maintenance scheduling.

### 10.2 Uncollected Offline Records (RNCO)

Query the number of offline-mode access logs not yet collected.

**Command**:
```
01+RQ+00+RNCO
```

**Response Example**:
```
01+RQ+00+RNCO]12
```
Device has 12 uncollected offline events.

**Use Case**: Track events logged during network outages or offline validation periods.

**Note**: This parameter is separate from `RNC` and specifically tracks events validated locally when the server was unreachable.

### 10.3 Query Patterns and Best Practices

#### Health Monitoring Poll Sequence

For comprehensive device health checks, query in this order:

```
1. RQ+00+TP     (Is device locked?)
2. RQ+00+SEMP   (Is employer configured?)
3. RQ+00+RNC    (Pending events to collect?)
4. RQ+00+R      (Total events stored)
5. RQ+00+U      (User count)
6. RQ+00+C      (Card count)
7. RQ+00+D      (Biometric count)
```

#### Capacity Planning Queries

Before bulk operations:

```
1. RQ+00+TD     (Max biometric capacity)
2. RQ+00+D      (Current biometric count)
   → Available slots = TD - D

3. RQ+00+U      (User count)
4. RQ+00+C      (Card count)
   → Validate ratios, plan imports
```

#### Printer Maintenance Queries

For time clock devices:

```
1. RQ+00+SP     (Out of paper?)
2. RQ+00+PP     (Low paper warning?)
3. RQ+00+QP     (Exact capacity)
4. RQ+00+MRPE   (Printer communication OK?)
```

### 10.4 Response Handling

All `RQ` responses follow the format:
```
<SB><XXXX><II>+RQ+00+PARAMETER]VALUE<CS><EB>
```

**Error Responses**:
- If parameter is unsupported: Device may return empty response or error code
- If device is offline: No response (timeout)
- If parameter is valid but data unavailable: May return `]0` or empty value

**Timeout Recommendations**:
- Standard queries: 3000ms
- Paper/printer queries: 5000ms (hardware sensor reads can be slower)
- Capacity queries: 2000ms (fast memory lookups)

## 11. Extended Command Set

This section documents advanced commands discovered from analyzing the official manufacturer's Java client emulator. These commands provide sophisticated access control features including time-based permissions, access groups, relay automation, and display customization.

**Compatibility Note**: Most extended commands are specific to Primme Acesso and are not supported on Argos or Primme SF models. Always verify device compatibility before implementation.

### 11.1 Access Groups (EGA/RGA)

Access groups enable logical grouping of users and cards for centralized permission management. Instead of configuring each card individually, cards are assigned to groups with shared access rules, time periods, and schedules.

#### 11.1.1 Send Access Group (EGA)

Create, update, or delete access group definitions.

**Command Structure**:
```
<SB><XXXX><II>+EGA+00+QTY+MODE[GROUP_ID[GROUP_NAME[VALID_FROM[VALID_UNTIL[FIELD5[FIELD6[FIELD7[[[FIELD9[[FIELD11[[FIELD13[[<CS><EB>
```

**Fields**:
- `QTY`: Number of groups in this message (typically 1)
- `MODE`: Operation mode
  - `I`: Insert new group
  - `A`: Update existing group
  - `E`: Delete group
  - `L`: Clear all groups
- `GROUP_ID`: Unique group identifier (6 digits, zero-padded, e.g., `000023`)
- `GROUP_NAME`: Descriptive group name (max 40 characters)
- `VALID_FROM`: Start validity (dd/mm/yyyy HH:MM:SS)
- `VALID_UNTIL`: End validity (dd/mm/yyyy HH:MM:SS)
- `FIELD5-13`: Reserved/device-specific configuration fields

**Example - Insert**:
```
01+EGA+00+1+I[000023[Grupo Equipe Suporte[01/01/2010 00:00:01[30/12/2012 23:59:59[2[1[1[[[0[[0[[0[[
```
Creates access group "Grupo Equipe Suporte" (ID 000023) valid from 2010-2012.

**Example - Delete**:
```
01+EGA+00+1+E[000023
```
Deletes access group with ID 000023.

**Example - Clear All**:
```
01+EGA+00+0+L
```
Removes all access groups from device memory.

**Use Case**:
- Organize users by department (e.g., "Engineering", "HR", "Security")
- Define contractor groups with expiration dates
- Implement visitor access with time restrictions

#### 11.1.2 Receive Access Group (RGA)

Query access group definitions stored on the device.

**Command Structure**:
```
<SB><XXXX><II>+RGA+00+QTY]START_INDEX<CS><EB>
```

**Fields**:
- `QTY`: Number of groups to retrieve
- `START_INDEX`: Starting index (0-based)

**Example**:
```
01+RGA+00+2]0
```
Retrieves 2 access groups starting from index 0.

**Response Format**:
```
01+RGA+00+2+000023[Grupo Equipe Suporte[01/01/2010 00:00:01[30/12/2012 23:59:59[...]+000024[Visitantes[...]
```

**Use Case**: Audit existing group configurations, verify synchronization.

### 11.2 Card-Group Associations (ECGA/RCGA)

Link individual cards to access groups. Each card can be associated with one or more groups (device-dependent).

#### 11.2.1 Send Card-Group Association (ECGA)

Associate cards with access groups.

**Command Structure**:
```
<SB><XXXX><II>+ECGA+00+QTY+MODE[GROUP_ID[CARD_INDEX<CS><EB>
```

**Fields**:
- `QTY`: Number of associations in this message
- `MODE`:
  - `I`: Insert association
  - `E`: Delete association
  - `L`: Clear all associations
- `GROUP_ID`: Access group ID (6 digits)
- `CARD_INDEX`: Index of card in device memory (1-based)

**Example - Associate Card**:
```
01+ECGA+00+1+I[000023[1
```
Associates card at index 1 with access group 000023.

**Example - Multiple Associations**:
```
01+ECGA+00+3+I[000023[1+I[000023[2+I[000024[3
```
- Cards 1 and 2 → Group 000023
- Card 3 → Group 000024

**Example - Delete Association**:
```
01+ECGA+00+1+E[000023[1
```
Removes card 1 from group 000023.

**Use Case**: Bulk permission updates, temporary group membership, role-based access.

#### 11.2.2 Receive Card-Group Association (RCGA)

Query which cards belong to which groups.

**Command Structure**:
```
<SB><XXXX><II>+RCGA+00+QTY]START_INDEX<CS><EB>
```

**Example**:
```
01+RCGA+00+5]0
```
Retrieves 5 card-group associations starting from index 0.

**Use Case**: Validate group memberships, audit access permissions.

### 11.3 Relay Triggers (EACI/RACI)

Schedule automatic relay activations (e.g., door unlocking, alarm activation) based on time and weekday patterns. Useful for automated opening/closing schedules.

#### 11.3.1 Send Relay Trigger (EACI)

Configure scheduled relay activations.

**Command Structure**:
```
<SB><XXXX><II>+EACI+00+QTY+MODE[TRIGGER_ID[NAME[TIME[RELAY_NUM[DURATION[WEEKDAYS<CS><EB>
```

**Fields**:
- `QTY`: Number of triggers in this message
- `MODE`: `I`=Insert, `A`=Update, `E`=Delete, `L`=Clear all
- `TRIGGER_ID`: Unique trigger identifier (numeric)
- `NAME`: Descriptive name (e.g., "Sirene Almoço", "Porta Automática")
- `TIME`: Activation time (HH:MM:SS)
- `RELAY_NUM`: Relay number to activate (1-3, device-dependent)
- `DURATION`: Activation duration in seconds
- `WEEKDAYS`: Days to activate (bitmask: `2`=Mon, `3`=Tue, `4`=Wed, `5`=Thu, `6`=Fri, `7`=Sat, `1`=Sun)

**Example - Lunch Alarm**:
```
01+EACI+00+1+I[13[Sirene Almoço[12:00:00[1[5[23456
```
Activates relay 1 for 5 seconds at 12:00:00, Monday through Friday (23456).

**Example - Weekend Door Opening**:
```
01+EACI+00+1+I[20[Abertura Fim de Semana[08:00:00[2[10[17
```
Activates relay 2 for 10 seconds at 08:00:00 on Saturday and Sunday (17).

**Example - Delete Trigger**:
```
01+EACI+00+1+E[13
```
Deletes trigger with ID 13.

**Use Case**:
- Automated door unlocking during business hours
- Scheduled alarm activation/deactivation
- Break time signals (sirens, bells)
- HVAC/lighting control integration

#### 11.3.2 Receive Relay Trigger (RACI)

Query configured relay trigger schedules.

**Command Structure**:
```
<SB><XXXX><II>+RACI+00+QTY]START_INDEX<CS><EB>
```

**Example**:
```
01+RACI+00+2]0
```
Retrieves 2 relay triggers starting from index 0.

### 11.4 Time Periods (EPER/RPER)

Define time windows (start time, end time, active days) used by access groups and schedules. Time periods are reusable components referenced by schedules.

#### 11.4.1 Send Time Period (EPER)

Create time period definitions.

**Command Structure**:
```
<SB><XXXX><II>+EPER+00+QTY+MODE[PERIOD_ID[START_TIME[END_TIME[WEEKDAYS<CS><EB>
```

**Fields**:
- `QTY`: Number of periods in this message
- `MODE`: `I`=Insert, `A`=Update, `E`=Delete, `L`=Clear all
- `PERIOD_ID`: Unique period identifier (numeric)
- `START_TIME`: Period start (HH:MM:SS)
- `END_TIME`: Period end (HH:MM:SS)
- `WEEKDAYS`: Active days (e.g., `234567` = Mon-Sun, `23456` = Mon-Fri)

**Example - Business Hours**:
```
01+EPER+00+1+I[1[08:00:00[18:00:00[23456
```
Period 1: 08:00-18:00, Monday through Friday.

**Example - Night Shift**:
```
01+EPER+00+1+I[13[22:00:00[06:00:00[234567
```
Period 13: 22:00-06:00 (overnight), all week.

**Note**: For overnight periods (end time < start time), the period spans midnight.

**Example - Weekend Access**:
```
01+EPER+00+1+I[5[09:00:00[17:00:00[17
```
Period 5: 09:00-17:00, Saturday and Sunday only.

**Use Case**: Define reusable time windows for access control, shift patterns, maintenance windows.

#### 11.4.2 Receive Time Period (RPER)

Query time period definitions.

**Command Structure**:
```
<SB><XXXX><II>+RPER+00+QTY]START_INDEX<CS><EB>
```

**Example**:
```
01+RPER+00+3]0
```
Retrieves 3 time periods starting from index 0.

### 11.5 Schedules (EHOR/RHOR)

Named schedules that reference time periods. Schedules provide a human-readable name for complex time period combinations.

#### 11.5.1 Send Schedule (EHOR)

Create named schedules referencing time periods.

**Command Structure**:
```
<SB><XXXX><II>+EHOR+00+QTY+MODE[SCHEDULE_ID[NAME[FIELD3[PERIOD_ID<CS><EB>
```

**Fields**:
- `QTY`: Number of schedules in this message
- `MODE`: `I`=Insert, `A`=Update, `E`=Delete, `L`=Clear all
- `SCHEDULE_ID`: Unique schedule identifier (numeric)
- `NAME`: Schedule name (max 40 characters)
- `FIELD3`: Reserved field (typically `1`)
- `PERIOD_ID`: Reference to time period ID (from EPER)

**Example**:
```
01+EHOR+00+1+I[13[Horário da Tarde[1[13
```
Creates schedule "Horário da Tarde" (ID 13) using period 13.

**Example - Multiple Schedules**:
```
01+EHOR+00+2+I[1[Turno Diurno[1[1+I[2[Turno Noturno[1[13
```
- Schedule 1 "Turno Diurno" uses period 1
- Schedule 2 "Turno Noturno" uses period 13

**Use Case**: Organize time-based access (shift schedules, visitor hours, contractor access windows).

#### 11.5.2 Receive Schedule (RHOR)

Query schedule definitions.

**Command Structure**:
```
<SB><XXXX><II>+RHOR+00+QTY]START_INDEX<CS><EB>
```

**Example**:
```
01+RHOR+00+2]0
```
Retrieves 2 schedules starting from index 0.

### 11.6 Holidays (EFER/RFER)

Register holiday dates where normal access rules may be overridden or disabled. Holidays are year-independent (e.g., "01/01" applies to every January 1st).

#### 11.6.1 Send Holiday (EFER)

Register holiday dates.

**Command Structure**:
```
<SB><XXXX><II>+EFER+00+QTY+MODE[DATE<CS><EB>
```

**Fields**:
- `QTY`: Number of holidays in this message
- `MODE`: `I`=Insert, `E`=Delete, `L`=Clear all
- `DATE`: Holiday date in `dd/mm` format (year-independent)

**Example - New Year's Day**:
```
01+EFER+00+1+I[01/01
```
Registers January 1st as a holiday.

**Example - Multiple Holidays**:
```
01+EFER+00+3+I[01/01+I[25/12+I[07/09
```
Registers January 1st, December 25th, and September 7th (Brazilian Independence Day).

**Example - Delete Holiday**:
```
01+EFER+00+1+E[25/12
```
Removes December 25th from holiday list.

**Use Case**:
- Disable automatic door unlocking on holidays
- Override access group rules for public holidays
- Apply special schedules on non-working days

#### 11.6.2 Receive Holiday (RFER)

Query registered holidays, optionally filtered by month.

**Command Structure**:
```
<SB><XXXX><II>+RFER+00+QTY+MODE/MONTH<CS><EB>
```

**Fields**:
- `QTY`: Number of holidays to retrieve
- `MODE`: Filter mode (typically `0` for all)
- `MONTH`: Month number (1-12) or `0` for all months

**Example - All Holidays in January**:
```
01+RFER+00+1+0/1
```
Retrieves all holidays in month 1 (January).

**Example - All Holidays**:
```
01+RFER+00+10+0/0
```
Retrieves up to 10 holidays across all months.

### 11.7 Display Messages (EMSG/RMSG)

Customize the messages displayed on the device LCD during entry and exit events. Messages can include static text and dynamic fields (user name, time, etc.).

#### 11.7.1 Send Messages (EMSG)

Configure default entry/exit messages.

**Command Structure**:
```
<SB><XXXX><II>+EMSG+00+MODE[ENTRY_MSG[ENTRY_FIELD[[EXIT_MSG[[EXIT_FIELD[<CS><EB>
```

**Fields**:
- `MODE`: Message mode
  - `2`: Custom messages
  - Other values: Device-specific defaults
- `ENTRY_MSG`: Message shown on entry (max 40 characters)
- `ENTRY_FIELD`: Dynamic field code for entry
  - `5`: Display user name
  - `0`: No dynamic field
  - Other codes device-specific (time, employee ID, etc.)
- `EXIT_MSG`: Message shown on exit (max 40 characters)
- `EXIT_FIELD`: Dynamic field code for exit

**Example - Welcome Message with Name**:
```
01+EMSG+00+2[Bem Vindo[5[[Ate Logo[[5[
```
- Entry: "Bem Vindo" + user name (field 5)
- Exit: "Ate Logo" + user name (field 5)

**Example - Static Messages**:
```
01+EMSG+00+2[ACESSO AUTORIZADO[0[[BOA VIAGEM[[0[
```
- Entry: "ACESSO AUTORIZADO" (no dynamic field)
- Exit: "BOA VIAGEM" (no dynamic field)

**Example - Time-based Messages**:
```
01+EMSG+00+2[Bom Dia[3[[Saudacao[[5[
```
- Entry: "Bom Dia" + field 3 (possibly time)
- Exit: "Saudacao" + user name

**Field Code Reference** (device-dependent):
- `0`: No dynamic field (static text only)
- `3`: Current time or date
- `5`: User name
- `7`: Employee ID
- Other codes: Consult device manual

**Use Case**: Personalized user experience, multilingual messages, branding.

#### 11.7.2 Receive Messages (RMSG)

Query current entry/exit message configuration.

**Command Structure**:
```
<SB><XXXX><II>+RMSG+00<CS><EB>
```

**Example**:
```
01+RMSG+00
```
Retrieves configured entry and exit messages.

**Response Example**:
```
01+RMSG+00+2[Bem Vindo[5[[Ate Logo[[5[
```

**Use Case**: Verify message configuration, audit display settings.

### 11.8 Batch File Processing

The official client emulator supports batch command execution from text files, enabling automated testing, bulk configuration, and scripted device management.

#### 11.8.1 File Format

Plain text file (`.txt` extension) with one command per line. Commands are executed sequentially.

**Example File** (`config.txt`):
```
EC+00+IP[192.168.1.100]
EC+00+PORTA[3000]
EC+00+TIPO_VALIDA[O]
EH+00+21/10/2025 10:30:00]00/00/00]00/00/00
RQ+00+U
RQ+00+C
```

Commands:
1. Set device IP to 192.168.1.100
2. Set port to 3000
3. Set validation mode to Online
4. Set date/time
5. Query user count
6. Query card count

#### 11.8.2 Special Commands

**Pause** - Wait for user keypress:
```
*pause*
```
Execution halts until user presses any key. Useful for reviewing output before continuing.

**Sleep** - Wait for specified milliseconds:
```
*sleep*5000
```
Pauses execution for 5000ms (5 seconds). Useful for waiting after configuration changes.

**Example with Special Commands**:
```
RH+00
RE+00
*pause*
EC+00+VOLUME[9]
*sleep*3000
RQ+00+U
```

Flow:
1. Query date/time
2. Query employer
3. PAUSE - wait for user to review
4. Set volume to 9
5. SLEEP - wait 3 seconds for volume change
6. Query user count

#### 11.8.3 Batch Execution Modes

**Single Execution**:
- Load file and execute once
- Stops at end of file or on error

**Loop Mode**:
- Execute file repeatedly
- Configurable loop count (e.g., 10 iterations) or infinite
- Press ESC to stop infinite loops
- Useful for stress testing, continuous monitoring

**Use Cases**:
- Initial device provisioning (bulk config)
- Regression testing (execute test suite)
- Continuous health monitoring (loop query commands)
- Automated data migration (bulk user/card import)

**Example - Device Setup Script**:
```
# Initial configuration
EC+00+NR_EQUIP[1]
EC+00+IP[192.168.1.100]
EC+00+TIPO_VALIDA[O]
EC+00+TIMEOUT_ON[3000]
*pause*

# Set employer
EE+00+2]00000000001]]Minha Empresa]Curitiba
*sleep*2000

# Verify setup
RC+00+IP
RC+00+TIPO_VALIDA
RQ+00+SEMP
*pause*

# Add initial users
EU+00+1+I[111111111111[Admin[0[1[000001
*sleep*1000
ECAR+00+1+I[1[1[01/01/2025 00:00:01[31/12/2025 23:59:59[1[1[0[999999[000001[[BM[0[0[0[0[[0
```

## 12. Command Coverage Summary

This summary provides a comprehensive view of all Henry protocol commands discovered from the official client emulator, organized by implementation status.

### 12.1 Fully Documented Commands (Send + Receive)

These commands have complete bidirectional support with detailed field specifications:

| Category | Send Command | Receive Command | Primme Acesso | Argos | Primme SF |
|----------|--------------|-----------------|---------------|-------|-----------|
| Configuration | EC | RC | ✓ | ✓ | ✓ |
| Employer | EE | RE | ✓ | ✗ | ✗ |
| Users | EU | RU | ✓ | ✗ | ✗ |
| Date/Time | EH | RH | ✓ | ✓ | ✓ |
| Cards | ECAR | RCAR | ✓ | ✓ | ✓ |
| Access Groups | EGA | RGA | ✓ | ✗ | ✗ |
| Card-Group Associations | ECGA | RCGA | ✓ | ✗ | ✗ |
| Relay Triggers | EACI | RACI | ✓ | ✗ | ✗ |
| Time Periods | EPER | RPER | ✓ | ✗ | ✗ |
| Schedules | EHOR | RHOR | ✓ | ✗ | ✗ |
| Holidays | EFER | RFER | ✓ | ✗ | ✗ |
| Messages | EMSG | RMSG | ✓ | ✗ | ✗ |
| Online Events | REON | REON | ✓ | ✓ | ✓ |

### 12.2 Query Commands (Receive Only)

Status and data retrieval commands:

| Command | Purpose | Query Types | Compatibility |
|---------|---------|-------------|---------------|
| RQ | Quantities & Status | 12 types (U, C, D, TD, R, RNC, RNCO, TP, MRPE, SEMP, PP, SP, QP) | All devices |
| RR/ER | Access Logs | 5 filter modes (M, N, D, T, C) | All devices |

**RQ Query Types**:
1. U - User count
2. C - Card count
3. D - Biometric count
4. TD - Total biometric capacity
5. R - Record count
6. RNC - Uncollected record count
7. RNCO - Uncollected offline records
8. TP - Device lock status
9. MRPE - MRP communication error
10. SEMP - Employer status
11. PP - Low paper sensor
12. SP - No paper status
13. QP - Paper capacity (detailed)

**RR/ER Filter Modes**:
1. M - Filter by memory address
2. N - Filter by NSR (sequential number)
3. D - Filter by date range
4. T - Filter by index
5. C - Filter by uncollected status

### 12.3 Send Only / Partially Documented

| Command | Purpose | Documentation Level | Note |
|---------|---------|---------------------|------|
| ED | Send Biometrics | Partial | Template format proprietary, device-specific |
| RD | Receive Biometric List | Partial | Lists users with fingerprints, template data not exposed |
| EFUN | Send Functions | Incomplete | Device-specific advanced features |

### 12.4 Real-time Validation Commands

Online event flow commands (REON protocol):

| Code | Direction | Purpose | Triggered By |
|------|-----------|---------|--------------|
| 000+0 | Device → Server | Access request | Card/biometric/keypad input |
| 00+1 | Server → Device | Grant both directions | Server validation logic |
| 00+5 | Server → Device | Grant entry | Server validation logic |
| 00+6 | Server → Device | Grant exit | Server validation logic |
| 00+30 | Server → Device | Deny access | Server validation logic |
| 000+80 | Device → Server | Waiting for rotation | After access granted |
| 000+81 | Device → Server | Rotation completed | User passed through |
| 000+82 | Device → Server | Rotation timeout | User did not pass |

### 12.5 Implementation Phases for Turnkey Emulator

**Phase 1: Core Protocol** (Complete)
- [x] Message parsing (STX, ID, PROTOCOL, COMMAND, DATA, ETX, CHECKSUM)
- [x] Message building with XOR checksum calculation
- [x] TCP server/client connections
- [x] Command code recognition and categorization

**Phase 2: Configuration & Status** (In Progress)
- [ ] EC/RC - Configuration get/set
- [ ] RQ - Device status queries (all 12 types)
- [ ] EH/RH - Date/time synchronization
- [ ] Offline database for local validation

**Phase 3: User & Access Management**
- [ ] EE/RE - Employer configuration
- [ ] EU/RU - User CRUD operations
- [ ] ECAR/RCAR - Card CRUD operations
- [ ] RR - Access log retrieval (all 5 filter modes)
- [ ] Event marking (collected vs uncollected)

**Phase 4: Advanced Features**
- [ ] EGA/RGA/ECGA/RCGA - Access groups and associations
- [ ] EPER/RPER/EHOR/RHOR - Time-based access control
- [ ] EFER/RFER - Holiday management
- [ ] EACI/RACI - Scheduled relay triggers
- [ ] EMSG/RMSG - Display message customization

**Phase 5: Biometrics** (Optional)
- [ ] ED/RD - Biometric template handling
- [ ] Control iD SDK integration
- [ ] Digital Persona SDK integration
- [ ] Mock biometric reader implementation

**Phase 6: Real-time Validation** (Priority)
- [ ] REON+000+0 - Access request handling
- [ ] REON+00+1/5/6 - Grant access responses with relay control
- [ ] REON+00+30 - Deny access response with logging
- [ ] REON+000+80/81/82 - Rotation event tracking
- [ ] Timeout management (online/offline fallback)

**Phase 7: Batch Processing**
- [ ] File-based command execution
- [ ] Special commands (*pause*, *sleep*)
- [ ] Loop mode with ESC interrupt
- [ ] Progress tracking and error reporting

### 12.6 Testing Strategy

**Unit Tests**:
- Message parsing/building for each command type
- Checksum calculation validation
- Field separator handling
- Date/time format validation

**Integration Tests**:
- Complete access flow (request → grant → rotation → log)
- Offline fallback when server unavailable
- Access group permission resolution
- Time-based access rule enforcement

**Hardware Tests** (requires physical devices):
- ACR122U RFID reader integration
- Relay activation timing
- Display message rendering
- Keypad input handling

**Protocol Compliance Tests**:
- Compatibility with Primme Acesso firmware versions
- Argos command subset validation
- Primme SF basic command support

## 13. Emulator Implementation

### 13.1 Turnstile States

```python
class TurnstileState(Enum):
    IDLE = 0
    WAITING_VALIDATION = 1
    WAITING_ROTATION = 2
    ROTATION_IN_PROGRESS = 3
    ROTATION_COMPLETED = 4
    TIMEOUT = 5
    BLOCKED = 6
```

### 13.2 State Flow Diagram

```
[IDLE] → Card Presented → [WAITING_VALIDATION]
          ↓
    Validation OK?
    Yes → [WAITING_ROTATION] → Timeout → [TIMEOUT] → [IDLE]
                ↓                             ↓
          Rotation Started            Manual Release
                ↓                             ↓
      [ROTATION_IN_PROGRESS]         [WAITING_ROTATION]
                ↓
        [ROTATION_COMPLETED] → [IDLE]

    No → [BLOCKED] → Timeout → [IDLE]
```

### 13.3 Important Timeouts

| Event | Default Time | Configurable |
|-------|--------------|--------------|
| Online Response | 3000ms | Yes (500-10000ms) |
| Waiting for Rotation | 5s | Yes (via command) |
| Offline Mode | 60s | Yes (2-600s) |
| Anti-passback | 0min | Yes (0-999999min) |

### 13.4 Critical Validations

1. **Card Format**: 3-20 ASCII characters
2. **Date/Time Format**: dd/mm/yyyy hh:mm:ss
3. **Direction**: Valid values 0, 1, 2
4. **Reader Type**: 1=RFID, 5=Biometry
5. **Checksum**: Calculate and validate in all messages

## 14. Testing and Validation

### 14.1 Required Test Scenarios

#### Normal Flow
1. Valid card presentation
2. Software release
3. Complete rotation
4. Event recording

#### Flow with Timeout
1. Valid card presentation
2. Software release
3. No rotation performed
4. Timeout and return to initial state

#### Flow with Denial
1. Invalid card presentation
2. Software denial
3. Block maintained
4. Denied event recording

#### Offline Mode
1. Server communication failure
2. Local validation
3. Event storage
4. Later synchronization

### 14.2 Protocol Validations

- [ ] Correct message format
- [ ] Separators in correct places
- [ ] Required fields present
- [ ] Correct data types
- [ ] Value ranges respected
- [ ] ASCII encoding maintained
- [ ] Checksum calculated correctly

## 15. Troubleshooting

### 15.1 Common Problems

| Problem | Probable Cause | Solution |
|---------|----------------|----------|
| Turnstile not responding | Incorrect ID | Verify NR_EQUIP configuration |
| Constant timeout | TIMEOUT_ON too low | Adjust to 3000-5000ms |
| Rotation not detected | Sensor problem | Verify code 81 after rotation |
| Card not read | Incorrect format | Validate card number |
| Display without message | Empty MSG_DISPLAY | Configure default message |

### 15.2 Recommended Logs

```
[TIMESTAMP] [LEVEL] [COMPONENT] [MESSAGE]
2024-01-15 10:23:45 INFO TURNSTILE_01 Card presented: 12651543
2024-01-15 10:23:45 DEBUG PROTOCOL TX: 01+REON+00+0]12651543]...
2024-01-15 10:23:46 DEBUG PROTOCOL RX: 01+REON+00+1]5]Access granted]
2024-01-15 10:23:46 INFO TURNSTILE_01 Access granted, waiting for rotation
2024-01-15 10:23:48 INFO TURNSTILE_01 Rotation completed, direction: ENTRY
```

## 16. References and Versions

### 16.1 Protocol Versions

| Version | Date | Main Changes |
|---------|------|--------------|
| 1.0.0.7 | - | Quantity and Status command |
| 1.0.0.8 | - | Keyboard support, numbered readers |
| 1.0.0.9 | - | Automatic enrollment mode |
| 1.0.0.10 | - | Improved online response |
| 1.0.0.23 | - | Stable Primme Acesso version |
| 8.0.0.50 | - | Current version with all features |

### 16.2 Compatibility

- **Primme Acesso**: Complete protocol with all extended commands
- **Argos**: Simplified protocol (without groups, periods, schedules, holidays)
- **Primme SF**: Basic protocol (online validation, configuration, cards, biometrics)

### 16.3 Related Documentation

This protocol guide should be used in conjunction with the following Turnkey project documentation:

**Command Catalog**:
- **henry-client-emulator-commands.md** - Complete command reference discovered from official Java client emulator
  - All 17 command categories with examples
  - Batch file processing syntax
  - Field-level specifications

**Emulator Configuration**:
- **emulator-configuration.md** - Device configuration parameters
  - All EC/RC configuration keys
  - Validation modes (ONLINE/OFFLINE/AUTO/SEMI-AUTO)
  - Reader types and hardware settings
  - Network and security configuration

**Emulator Modes**:
- **emulator-modes.md** - Operational mode details
  - ONLINE mode validation flow
  - OFFLINE mode local validation
  - Automatic fallback behavior
  - Timeout handling and retries

**Architecture**:
- **henry-complete-architecture.md** - System design overview
  - Workspace structure and crate organization
  - Hardware abstraction layer patterns
  - Repository pattern for storage
  - Async I/O design with Tokio

**Data Formats**:
- **data-formats.md** - Message format specifications
  - Field separator usage (], [, }, {, +)
  - Date/time format requirements
  - Checksum calculation (XOR)
  - STX/ETX frame structure

**TUI Specification**:
- **tui-specification.md** - Terminal User Interface design
  - Display layout (2x40 LCD simulation)
  - Keypad interaction patterns
  - Status indicators
  - Log panel behavior

## 17. Final Notes

This document represents the complete consolidation of the Henry protocol documentation, enhanced with discoveries from the official manufacturer's Java client emulator. The protocol supports three device classes with varying feature sets:

**Primme Acesso** (Full Protocol):
- All commands documented in sections 1-11
- Access groups, time periods, schedules, holidays
- Relay triggers and display customization
- Advanced biometric features
- Comprehensive status monitoring

**Argos** (Simplified Protocol):
- Core validation (REON)
- Basic configuration (EC/RC)
- Cards and biometrics (ECAR/ED)
- Access logs (RR)
- Status queries (RQ)

**Primme SF** (Basic Protocol):
- Online validation only
- Minimal configuration
- Card management
- Basic biometric support

### Implementation Guidelines

For emulator implementation, it is essential to:

1. **Strictly Follow Message Formats**: Field separators, date formats, and checksum calculations must match exactly.

2. **Respect Configured Timeouts**: Default 3000ms for online validation, configurable 500-10000ms range.

3. **Maintain Compatibility**: Implement feature detection to support different device models gracefully.

4. **Handle Edge Cases**: Timeout fallbacks, rotation abandonment, network failures, offline mode transitions.

5. **Implement Pagination**: For bulk data operations (cards, users, logs) to avoid memory exhaustion.

6. **Validate Input**: Card numbers (3-20 chars), dates (dd/mm/yyyy HH:MM:SS), directions (0/1/2), etc.

7. **Test Extensively**: Use the batch file processing feature to create comprehensive test suites.

### Development Approach

The implementation should be modular to support different functionality levels:

- **Core Module**: Message parsing, checksum, TCP/IP (all devices)
- **Validation Module**: Online/offline validation logic (all devices)
- **Configuration Module**: EC/RC parameter handling (all devices)
- **User Management Module**: EU/ECAR operations (Primme Acesso, Argos)
- **Access Control Module**: Groups, periods, schedules (Primme Acesso only)
- **Biometric Module**: ED/RD template handling (device-specific SDKs)

This layered architecture enables:
- Easy feature toggling based on device type
- Clean separation of concerns
- Testability with mock implementations
- Future extensibility for new commands

The Turnkey emulator aims to provide a complete, faithful implementation of the Henry protocol for development, testing, and integration purposes.
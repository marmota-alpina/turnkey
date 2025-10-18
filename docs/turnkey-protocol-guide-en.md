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

| Code | Name | Description |
|------|------|-------------|
| RC | Settings | Receive equipment settings |
| RE | Employer | Receive equipment employer |
| RQ | Quantity/Status | Receive quantities and status |

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

### 9.1 Record Collection

#### All Records
```
<SB><XXXX><II>+ER+00+T]QUANTITY]INITIAL_INDEX<CS><EB>
```

#### Only Uncollected
```
<SB><XXXX><II>+ER+00+C]QUANTITY]INITIAL_INDEX<CS><EB>
```

#### Filtered by Date/Time
```
<SB><XXXX><II>+ER+00+D]QUANTITY]INITIAL_DATE]FINAL_DATE<CS><EB>
```

### 9.2 Collection Confirmation

```
<SB><XXXX><II>+ER+00+QTY_COLLECTED+INDICES]<CS><EB>
```

## 10. Quantity and Status Query

### 10.1 Available Parameters

| Parameter | Description | Values |
|-----------|-------------|--------|
| D | Fingerprint quantity | 0-10000 |
| U | User quantity | 0-50000 |
| R | Record quantity | 0-999999999 |
| RNC | Uncollected records | 0-999999999 |
| RNCO | Uncollected offline records | 0-999999999 |
| C | Card quantity | 0-999999999 |
| TP | Blocked status | A/D |
| TD | Maximum supported biometrics | 300+ |

### 10.2 Usage Example

```
// Request user quantity
<SB><XXXX><II>+RQ+00+U<CS><EB>

// Response: 35 users
<SB><XXXX><II>+RQ+00+U]35<CS><EB>
```

## 11. Custom Messages

### 11.1 Sending Structure

```
<SB><XXXX><II>+EMSG+00+MODE_ENT_L1[MSG_ENT_L1[MODE_ENT_L2[MSG_ENT_L2[MODE_EXI_L1[MSG_EXI_L1[MODE_EXI_L2[MSG_EXI_L2<CS><EB>
```

Where:
- `MODE_*`: Message operation mode
- `MSG_*`: Message text
- `ENT`: Entry
- `EXI`: Exit
- `L1/L2`: Display line 1 or 2

## 12. Emulator Implementation

### 12.1 Turnstile States

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

### 12.2 State Flow Diagram

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

### 12.3 Important Timeouts

| Event | Default Time | Configurable |
|-------|--------------|--------------|
| Online Response | 3000ms | Yes (500-10000ms) |
| Waiting for Rotation | 5s | Yes (via command) |
| Offline Mode | 60s | Yes (2-600s) |
| Anti-passback | 0min | Yes (0-999999min) |

### 12.4 Critical Validations

1. **Card Format**: 3-20 ASCII characters
2. **Date/Time Format**: dd/mm/yyyy hh:mm:ss
3. **Direction**: Valid values 0, 1, 2
4. **Reader Type**: 1=RFID, 5=Biometry
5. **Checksum**: Calculate and validate in all messages

## 13. Testing and Validation

### 13.1 Required Test Scenarios

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

### 13.2 Protocol Validations

- [ ] Correct message format
- [ ] Separators in correct places
- [ ] Required fields present
- [ ] Correct data types
- [ ] Value ranges respected
- [ ] ASCII encoding maintained
- [ ] Checksum calculated correctly

## 14. Troubleshooting

### 14.1 Common Problems

| Problem | Probable Cause | Solution |
|---------|----------------|----------|
| Turnstile not responding | Incorrect ID | Verify NR_EQUIP configuration |
| Constant timeout | TIMEOUT_ON too low | Adjust to 3000-5000ms |
| Rotation not detected | Sensor problem | Verify code 81 after rotation |
| Card not read | Incorrect format | Validate card number |
| Display without message | Empty MSG_DISPLAY | Configure default message |

### 14.2 Recommended Logs

```
[TIMESTAMP] [LEVEL] [COMPONENT] [MESSAGE]
2024-01-15 10:23:45 INFO TURNSTILE_01 Card presented: 12651543
2024-01-15 10:23:45 DEBUG PROTOCOL TX: 01+REON+00+0]12651543]...
2024-01-15 10:23:46 DEBUG PROTOCOL RX: 01+REON+00+1]5]Access granted]
2024-01-15 10:23:46 INFO TURNSTILE_01 Access granted, waiting for rotation
2024-01-15 10:23:48 INFO TURNSTILE_01 Rotation completed, direction: ENTRY
```

## 15. References and Versions

### 15.1 Protocol Versions

| Version | Date | Main Changes |
|---------|------|--------------|
| 1.0.0.7 | - | Quantity and Status command |
| 1.0.0.8 | - | Keyboard support, numbered readers |
| 1.0.0.9 | - | Automatic enrollment mode |
| 1.0.0.10 | - | Improved online response |
| 1.0.0.23 | - | Stable Primme Acesso version |
| 8.0.0.50 | - | Current version with all features |

### 15.2 Compatibility

- **Primme Acesso**: Complete protocol
- **Argos**: Simplified protocol (without groups, periods, etc.)
- **Primme SF**: Basic protocol (online validation only)

## Final Notes

This document represents the complete consolidation of the Henry protocol documentation. For emulator implementation, it is essential to strictly follow message formats, respect configured timeouts, and maintain compatibility with different equipment firmware versions.

The implementation should be modular to support different functionality levels between Primme Acesso, Argos, and Primme SF, maintaining a common communication and validation core.
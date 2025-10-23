# Henry Protocol - Client Emulator Commands Reference

## Overview

This document catalogs all Henry protocol commands discovered from analyzing the official manufacturer's Java client emulator (`Cliente 8X`). These commands represent the complete API for interacting with Primme SF, Primme Acesso, and Prisma access control equipment.

**Source**: `client-emulator/Cliente 8X/Java` (Official manufacturer implementation)

---

## Command Categories

The client emulator organizes commands into 17 functional categories:

1. **Configurações** (Configuration)
2. **Empregador** (Employer)
3. **Usuários** (Users)
4. **Data Hora** (Date/Time)
5. **Biometria** (Biometrics)
6. **Registros** (Access Logs)
7. **Quantidade e Status** (Quantities and Status)
8. **Cartão** (Cards)
9. **Grupo de Acesso** (Access Groups)
10. **Cartão Grupo de Acesso** (Card Access Groups)
11. **Acionamento** (Relay Triggers)
12. **Período** (Time Periods)
13. **Horário** (Schedules)
14. **Feriado** (Holidays)
15. **Mensagem** (Messages)
16. **Evento Online** (Online Events - Real-time access validation)
17. **Arquivo** (File Processing - Batch operations)

---

## 1. Configurações (Configuration)

### Send Configuration (EC)

**Command**: `01+EC+00+<PARAMETER>[<VALUE>]`

**Purpose**: Send configuration parameters to the device

**Example**:
```
01+EC+00+HAB_TECLADO[H]
```
Enables the keypad (`H` = enabled)

**Common Parameters**:
- `HAB_TECLADO` - Enable/disable keypad
- `IP` - IP address
- `PORTA` - TCP port
- `TIPO_VALIDA` - Validation mode (O=online, F=offline, A=auto, S=semi-auto)
- `HAB_BIO` - Enable/disable biometrics
- `HAB_CARTAO` - Enable/disable card reader

### Receive Configuration (RC)

**Command**: `01+RC+00+<PARAMETER>`

**Purpose**: Query current configuration value

**Example**:
```
01+RC+00+IP
```
Retrieves the device IP address

---

## 2. Empregador (Employer)

### Send Employer (EE)

**Command**: `01+EE+00+<MODE>]<CNPJ>]]<COMPANY_NAME>]<LOCATION>`

**Purpose**: Configure employer/company information

**Fields**:
- `MODE`: `2` = set employer data
- `CNPJ`: Brazilian company ID (11 digits)
- `COMPANY_NAME`: Company name
- `LOCATION`: Company location/city

**Example**:
```
01+EE+00+2]00000000001]]Empresa Teste]Pinhais
```
Sets employer as "Empresa Teste" in Pinhais

### Receive Employer (RE)

**Command**: `01+RE+00`

**Purpose**: Query current employer information

**Response Example**:
```
01+RE+00+]07794589908]]Henry Equipamentos]Pinhais
```

---

## 3. Usuários (Users)

### Send User (EU)

**Command**: `01+EU+00+<QTY>+<MODE>[<PIS>[<NAME>[<FIELD1>[<FIELD2>[<MATRICULA>`

**Purpose**: Add/update/delete user records

**Fields**:
- `QTY`: Number of users in this message
- `MODE`: `I` = insert, `A` = update, `E` = delete
- `PIS`: Brazilian social ID (11 digits)
- `NAME`: User full name
- `FIELD1`: Reserved
- `FIELD2`: Reserved
- `MATRICULA`: Employee ID/badge number (6 digits, zero-padded)

**Example**:
```
01+EU+00+1+I[123123123123[TESTE[0[1[000023
```
Inserts user "TESTE" with PIS 123123123123 and employee ID 000023

### Receive User (RU)

**Command**: `01+RU+00+<QTY>]<START_INDEX>`

**Purpose**: Query user records

**Fields**:
- `QTY`: Number of users to retrieve
- `START_INDEX`: Starting index (1-based)

**Example**:
```
01+RU+00+3]1
```
Retrieves 3 users starting from index 1

---

## 4. Data Hora (Date/Time)

### Send Date/Time (EH)

**Command**: `01+EH+00+<DATETIME>]<DST_START>]<DST_END>`

**Purpose**: Set device date/time and daylight saving time (DST)

**Fields**:
- `DATETIME`: `dd/mm/yy HH:MM:SS`
- `DST_START`: DST start date `dd/mm/yy` (or `00/00/00` if not used)
- `DST_END`: DST end date `dd/mm/yy` (or `00/00/00` if not used)

**Example**:
```
01+EH+00+09/07/12 16:44:00]00/00/00]00/00/00
```
Sets date to July 9, 2012 at 16:44:00 without DST

**Note**: The client emulator has a "System Time" button that auto-fills current time

### Receive Date/Time (RH)

**Command**: `01+RH+00`

**Purpose**: Query current device date/time

**Response Example**:
```
01+RH+00+30/08/12 12:01:24]00/00/00]00/00/00
```

---

## 5. Biometria (Biometrics)

### Send Biometric Template (ED)

**Command**: `01+ED+00+<TEMPLATE_DATA>`

**Purpose**: Upload fingerprint templates to device

**Note**: Template data format is proprietary and device-specific. Not documented in client emulator.

### Receive Biometric List (RD)

**Command**: `01+RD+00+L]<QTY>}<START_INDEX>`

**Purpose**: Query list of users with registered fingerprints

**Fields**:
- `L`: List mode
- `QTY`: Number of records to retrieve
- `START_INDEX`: Starting index (0-based)

**Example**:
```
01+RD+00+L]2}0
```
Retrieves list of 2 users with fingerprints starting from index 0

---

## 6. Registros (Access Logs)

Access logs can be queried using multiple filter modes.

### Filter by Memory Address (M)

**Command**: `01+RR+00+M]<QTY>]<START_ADDRESS>`

**Purpose**: Retrieve events from specific memory address

**Example**:
```
01+RR+00+M]3]0
```
Retrieves 3 events starting from memory address 0

### Filter by NSR (N)

**Command**: `01+RR+00+N]<QTY>]<START_NSR>`

**Purpose**: Retrieve events by sequential record number (NSR)

**Fields**:
- `QTY`: Number of events
- `START_NSR`: Starting NSR number

**Example**:
```
01+RR+00+N]5]1
```
Retrieves 5 events starting from NSR 1

### Filter by Date (D)

**Command**: `01+RR+00+D]<QTY>]<START_DATETIME>]`

**Purpose**: Retrieve events after a specific date/time

**Example**:
```
01+RR+00+D]2]10/07/2012 08:00:01]
```
Retrieves 2 events after July 10, 2012 08:00:01

**Note**: For PrimmeAcesso, must include both start and end datetime

### Filter by Index (T)

**Command**: `01+RR+00+T]<QTY>]<START_INDEX>`

**Purpose**: Retrieve events by index

**Example**:
```
01+RR+00+T]5]1
```
Retrieves 5 events starting from index 1

### Filter by Uncollected (C)

**Command**: `01+RR+00+C]<QTY>]<START_INDEX>`

**Purpose**: Retrieve only events not yet collected

**Example**:
```
01+RR+00+C]5]0
```
Retrieves 5 uncollected events starting from index 0

---

## 7. Quantidade e Status (Quantities and Status)

Query device status and capacity information using `RQ+00+<PARAMETER>`.

### User Count (U)

**Command**: `01+RQ+00+U`

**Purpose**: Get total number of registered users

### Card Count (C)

**Command**: `01+RQ+00+C`

**Purpose**: Get total number of registered cards

### Biometric Count (D)

**Command**: `01+RQ+00+D`

**Purpose**: Get total number of registered fingerprints

### Total Biometric Capacity (TD)

**Command**: `01+RQ+00+TD`

**Purpose**: Get maximum fingerprint capacity of device

### Record Count (R)

**Command**: `01+RQ+00+R`

**Purpose**: Get total number of access logs in memory

### Uncollected Record Count (RNC)

**Command**: `01+RQ+00+RNC`

**Purpose**: Get count of access logs not yet collected

### Device Lock Status (TP)

**Command**: `01+RQ+00+TP`

**Purpose**: Check if device is locked

### MRP Communication Error (MRPE)

**Command**: `01+RQ+00+MRPE`

**Purpose**: Check for MRP (printer module) communication errors

### Employer Status (SEMP)

**Command**: `01+RQ+00+SEMP`

**Purpose**: Check if employer is not registered

### Low Paper Sensor (PP)

**Command**: `01+RQ+00+PP`

**Purpose**: Check if low paper sensor is active

### No Paper Status (SP)

**Command**: `01+RQ+00+SP`

**Purpose**: Check if device is out of paper

### Paper Capacity (QP)

**Command**: `01+RQ+00+QP`

**Purpose**: Get ticket capacity, current roll size, and total roll size

---

## 8. Cartão (Cards)

### Send Card (ECAR)

**Command**: `01+ECAR+00+<QTY>+<MODE>[<FIELD1>[<FIELD2>[<VALID_FROM>[<VALID_UNTIL>[<FIELD5>[<FIELD6>[<FIELD7>[<CARD_NUM>[<MATRICULA>[[<GROUP>[[<FIELD11>[<FIELD12>[<FIELD13>[[<FIELD15>`

**Purpose**: Add/update/delete card records

**Fields** (key ones):
- `QTY`: Number of cards
- `MODE`: `I` = insert, `A` = update, `E` = delete
- `CARD_NUM`: Card number
- `MATRICULA`: Associated employee ID
- `VALID_FROM`: Start validity date/time
- `VALID_UNTIL`: End validity date/time

**Example**:
```
01+ECAR+00+1+A[1[1[09/07/2012 08:00:01[09/07/2012 17:00:01[1[1[0[123[321[[BM[2[1[1[0[[0
```
Updates card with number 123, employee ID 321, valid from 09/07/2012 08:00:01 to 17:00:01

### Receive Card (RCAR)

**Command**: `01+RCAR+00+<QTY>]<START_INDEX>`

**Purpose**: Query card records

**Example**:
```
01+RCAR+00+2]0
```
Retrieves 2 cards starting from index 0

---

## 9. Grupo de Acesso (Access Groups)

### Send Access Group (EGA)

**Command**: `01+EGA+00+<QTY>+<MODE>[<GROUP_ID>[<GROUP_NAME>[<VALID_FROM>[<VALID_UNTIL>[<FIELD5>[<FIELD6>[<FIELD7>[[[<FIELD9>[[<FIELD11>[[<FIELD13>[[`

**Purpose**: Add/update/delete access groups

**Example**:
```
01+EGA+00+1+I[000023[Grupo Equipe Suporte[01/01/2010 00:00:01[30/12/2012 23:59:59[2[1[1[[[0[[0[[0[[
```
Inserts access group "Grupo Equipe Suporte" with ID 000023

### Receive Access Group (RGA)

**Command**: `01+RGA+00+<QTY>]<START_INDEX>`

**Purpose**: Query access group records

**Example**:
```
01+RGA+00+2]0
```
Retrieves 2 access groups starting from index 0

---

## 10. Cartão Grupo de Acesso (Card Access Groups)

### Send Card-Group Association (ECGA)

**Command**: `01+ECGA+00+<QTY>+<MODE>[<GROUP_ID>[<CARD_INDEX>`

**Purpose**: Associate cards with access groups

**Example**:
```
01+ECGA+00+1+I[000023[1
```
Associates card at index 1 with access group 000023

### Receive Card-Group Association (RCGA)

**Command**: `01+RCGA+00+<QTY>]<START_INDEX>`

**Purpose**: Query card-group associations

**Example**:
```
01+RCGA+00+2]0
```
Retrieves 2 associations starting from index 0

---

## 11. Acionamento (Relay Triggers)

### Send Relay Trigger (EACI)

**Command**: `01+EACI+00+<QTY>+<MODE>[<TRIGGER_ID>[<NAME>[<TIME>[<RELAY_NUM>[<DURATION>[<WEEKDAYS>`

**Purpose**: Schedule automatic relay activations

**Fields**:
- `TRIGGER_ID`: Trigger identifier
- `NAME`: Descriptive name
- `TIME`: Activation time `HH:MM:SS`
- `RELAY_NUM`: Relay number (1-3)
- `DURATION`: Duration in seconds
- `WEEKDAYS`: Bitmask (e.g., `23456` = Mon-Fri)

**Example**:
```
01+EACI+00+1+I[13[Sirene Almoço[12:00:00[1[5[23456
```
Activates relay 1 for 5 seconds at 12:00:00, Monday through Friday

### Receive Relay Trigger (RACI)

**Command**: `01+RACI+00+<QTY>]<START_INDEX>`

**Purpose**: Query relay trigger schedules

**Example**:
```
01+RACI+00+2]0
```
Retrieves 2 relay triggers starting from index 0

---

## 12. Período (Time Periods)

### Send Time Period (EPER)

**Command**: `01+EPER+00+<QTY>+<MODE>[<PERIOD_ID>[<START_TIME>[<END_TIME>[<WEEKDAYS>`

**Purpose**: Define time periods for access control

**Fields**:
- `PERIOD_ID`: Period identifier
- `START_TIME`: Start time `HH:MM:SS`
- `END_TIME`: End time `HH:MM:SS`
- `WEEKDAYS`: Days when period is active (e.g., `234567` = Mon-Sun)

**Example**:
```
01+EPER+00+1+I[13[13:00:00[19:00:00[234567
```
Defines period 13 from 13:00 to 19:00, Monday through Sunday

### Receive Time Period (RPER)

**Command**: `01+RPER+00+<QTY>]<START_INDEX>`

**Purpose**: Query time period definitions

**Example**:
```
01+RPER+00+2]0
```
Retrieves 2 time periods starting from index 0

---

## 13. Horário (Schedules)

### Send Schedule (EHOR)

**Command**: `01+EHOR+00+<QTY>+<MODE>[<SCHEDULE_ID>[<NAME>[<FIELD3>[<PERIOD_ID>`

**Purpose**: Create named schedules referencing time periods

**Example**:
```
01+EHOR+00+1+I[13[Horário da Tarde[1[13
```
Creates schedule "Horário da Tarde" using period 13

### Receive Schedule (RHOR)

**Command**: `01+RHOR+00+<QTY>]<START_INDEX>`

**Purpose**: Query schedule definitions

**Example**:
```
01+RHOR+00+2]0
```
Retrieves 2 schedules starting from index 0

---

## 14. Feriado (Holidays)

### Send Holiday (EFER)

**Command**: `01+EFER+00+<QTY>+<MODE>[<DATE>`

**Purpose**: Register holidays (access control may be disabled)

**Fields**:
- `DATE`: Holiday date `dd/mm` (year-independent)

**Example**:
```
01+EFER+00+1+I[01/01
```
Registers January 1st as a holiday

### Receive Holiday (RFER)

**Command**: `01+RFER+00+<QTY>+<MODE>/<MONTH>`

**Purpose**: Query registered holidays

**Example**:
```
01+RFER+00+1+0/1
```
Retrieves all holidays in month 1 (January)

---

## 15. Mensagem (Messages)

### Send Messages (EMSG)

**Command**: `01+EMSG+00+<MODE>[<ENTRY_MSG>[<ENTRY_FIELD>[[<EXIT_MSG>[[<EXIT_FIELD>[`

**Purpose**: Configure default entry/exit messages shown on device display

**Fields**:
- `MODE`: Message mode (`2` = custom messages)
- `ENTRY_MSG`: Message for entry (e.g., "Bem Vindo")
- `ENTRY_FIELD`: Field code (e.g., `5` = show employee name)
- `EXIT_MSG`: Message for exit (e.g., "Saudação")
- `EXIT_FIELD`: Field code for exit

**Example**:
```
01+EMSG+00+2[Bem Vindo[5[[3[[5[
```
Sets entry message to "Bem Vindo" + employee name

### Receive Messages (RMSG)

**Command**: `01+RMSG+00`

**Purpose**: Query current entry/exit messages

---

## 16. Evento Online (Online Event Validation)

**Special Mode**: This is the interactive real-time access control flow

### Device Sends Access Request

When a user presents credentials (card, keypad code, fingerprint), the device sends:

```
01+REON+000+0]<CARD_NUMBER>]<DATETIME>]<DIRECTION>]0]
```

**Fields**:
- `000+0`: Access request command code
- `CARD_NUMBER`: Card number or user code
- `DATETIME`: Current timestamp `dd/mm/yyyy HH:MM:SS`
- `DIRECTION`: `1` = entry, `2` = exit

### Client Responds - Grant Entry (00+5)

```
01+REON+00+5]<SECONDS>]<MESSAGE>]
```
Grants **entry** access

### Client Responds - Grant Exit (00+6)

```
01+REON+00+6]<SECONDS>]<MESSAGE>]
```
Grants **exit** access

### Client Responds - Grant Both (00+1)

```
01+REON+00+1]<SECONDS>]<MESSAGE>]
```
Grants access in **both directions**

### Client Responds - Deny (00+30)

```
01+REON+00+30]0]<MESSAGE>]
```
Denies access

**Configurable Parameters** (from emulator UI):
- `SECONDS`: Time to display message and activate relay (1-99)
- `MESSAGE`: Text shown on device display
- `RELAY_GROUP`: Which relays to activate (checkboxes for relay 1, 2, 3)

### Subsequent Events

After granting access, device may send:

**Waiting for Rotation**:
```
01+REON+000+80]]<DATETIME>]0]0]
```

**Rotation Complete**:
```
01+REON+000+81]]<DATETIME>]1]0]
```

**Rotation Timeout**:
```
01+REON+000+82]]<DATETIME>]0]0]
```

---

## 17. Arquivo (File Processing)

The client emulator supports batch processing commands from text files.

### File Format

Plain text file (`.txt`) with one command per line.

**Special Commands**:
- `*pause*` - Wait for user to press a key
- `*sleep*<MILLISECONDS>` - Wait for specified time

**Example** (`teste.txt`):
```
RH+00
RH+00
RE+00
*pause*
RH+00
RE+00
*sleep*8000
RH+00
```

### Batch Execution

- **Read File**: Execute commands once
- **Loop Mode**: Execute commands repeatedly
  - Configure loop count or infinite loop
  - Press ESC to stop infinite loop

### Progress Display

The emulator shows a progress window during batch execution:
- Current command index (X of Y)
- Percentage complete
- Current loop number

---

## Protocol Format Details

### Message Structure

All commands follow this format:
```
<STX><ID>+<PROTOCOL>+<COMMAND>+<DATA><ETX><CHECKSUM>
```

**Components**:
- `STX` (Start of Text): `0x02`
- `ID`: Device ID (01-99, zero-padded)
- `PROTOCOL`: Protocol identifier (e.g., `REON`, `EC`, `RE`)
- `COMMAND`: Command code (e.g., `00`, `000+0`)
- `DATA`: Command-specific data fields
- `ETX` (End of Text): `0x03`
- `CHECKSUM`: XOR checksum of all bytes between STX and ETX

### Field Separators

- `]` - Primary field separator
- `[` - Alternative field separator (used in some commands)
- `}` - Special separator (biometrics)
- `{` - Reserved separator

### Client Implementation Notes

From the Java source code:

**Text Formatting** (`textFormat()`):
- Adds STX header
- Calculates XOR checksum
- Adds ETX trailer

**Hex Display** (`stringHexFormat()`):
- Converts each character to hexadecimal
- Displayed for debugging purposes

**Connection Handling**:
- TCP client connects to device IP:PORT
- Background thread polls for incoming data every 500ms
- Displays sent/received data in ASCII and hexadecimal

---

## Command Coverage Summary

### Fully Documented (Send + Receive)
✅ Configuration (EC/RC)
✅ Employer (EE/RE)
✅ Users (EU/RU)
✅ Date/Time (EH/RH)
✅ Cards (ECAR/RCAR)
✅ Access Groups (EGA/RGA)
✅ Card-Group Associations (ECGA/RCGA)
✅ Relay Triggers (EACI/RACI)
✅ Time Periods (EPER/RPER)
✅ Schedules (EHOR/RHOR)
✅ Holidays (EFER/RFER)
✅ Messages (EMSG/RMSG)
✅ Online Events (REON)

### Receive Only
✅ Access Logs (RR) - 5 filter modes
✅ Quantities/Status (RQ) - 12 query types

### Send Only / Partially Documented
⚠️ Biometrics (ED/RD) - Template format proprietary

---

## Integration Checklist for Turnkey Emulator

Based on the client emulator analysis, our Turnkey emulator should implement:

### Phase 1: Core Protocol
- [x] Message parsing (STX, ID, PROTOCOL, COMMAND, DATA, ETX, CHECKSUM)
- [x] Message building with checksum calculation
- [x] TCP server/client connections
- [x] Command code recognition

### Phase 2: Configuration & Status
- [ ] EC/RC - Configuration get/set
- [ ] RQ - Device status queries (12 types)
- [ ] EH/RH - Date/time synchronization

### Phase 3: User & Access Management
- [ ] EE/RE - Employer configuration
- [ ] EU/RU - User CRUD operations
- [ ] ECAR/RCAR - Card CRUD operations
- [ ] RR - Access log retrieval (5 filter modes)

### Phase 4: Advanced Features
- [ ] EGA/RGA/ECGA/RCGA - Access groups
- [ ] EPER/RPER/EHOR/RHOR - Time-based access control
- [ ] EFER/RFER - Holiday management
- [ ] EACI/RACI - Scheduled relay triggers
- [ ] EMSG/RMSG - Display messages

### Phase 5: Biometrics (Optional)
- [ ] ED/RD - Biometric template handling
- [ ] Control iD SDK integration
- [ ] Digital Persona SDK integration

### Phase 6: Real-time Validation
- [ ] REON+000+0 - Access request handling
- [ ] REON+00+1/5/6 - Grant access responses
- [ ] REON+00+30 - Deny access response
- [ ] REON+000+80/81/82 - Rotation events

---

## References

- **Source Code**: `client-emulator/Cliente 8X/Java/Projeto/Cliente 8X/src/cliente8x/FrmClient.java`
- **Sample Files**:
  - `Resposta.txt` - Sample command/response pairs
  - `teste.txt` - Batch command file example
- **Related Docs**:
  - [Emulator Modes](emulator-modes.md) - ONLINE vs OFFLINE validation
  - [Emulator Configuration](emulator-configuration.md) - Device settings
  - [Henry Protocol Guide](turnkey-protocol-guide-en.md) - Protocol specification

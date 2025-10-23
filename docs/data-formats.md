# Turnkey Data Formats

## Overview

The Turnkey emulator supports multiple data import/export formats for bulk operations, backups, and interoperability with external systems. This document defines all supported file formats based on Brazilian access control industry standards.

## File Format Standards

All text files follow these conventions:
- **Encoding**: UTF-8 (with BOM optional)
- **Line Ending**: CRLF (`\r\n`) or LF (`\n`)
- **Field Separator**: Pipe character (`|`)
- **Date Format**: `dd/mm/yyyy` or `dd/mm/yyyy HH:MM:SS`
- **Empty Fields**: Represented by empty string between separators
- **Comments**: Lines starting with `#` are ignored
- **Case**: Field names are case-insensitive

---

## 1. User Import/Export (`colaborador.txt`)

### Format Specification

```
PIS|NOME|MATRICULA|CPF|VALIDADE_INICIO|VALIDADE_FIM|ATIVO|ALLOW_CARD|ALLOW_BIO|ALLOW_KEYPAD|CODIGO
```

### Field Definitions

| Field             | Type    | Length   | Required   | Description                            |
|-------------------|---------|----------|------------|----------------------------------------|
| `PIS`             | Numeric | 11       | No         | PIS/PASEP number (Brazilian social ID) |
| `NOME`            | Text    | 100      | Yes        | Full name                              |
| `MATRICULA`       | Text    | 20       | Yes        | Employee ID / badge number             |
| `CPF`             | Numeric | 11       | No         | CPF number (Brazilian ID)              |
| `VALIDADE_INICIO` | Date    | -        | No         | Access valid from (dd/mm/yyyy)         |
| `VALIDADE_FIM`    | Date    | -        | No         | Access valid until (dd/mm/yyyy)        |
| `ATIVO`           | Boolean | 1        | Yes        | Active status (1=active, 0=inactive)   |
| `ALLOW_CARD`      | Boolean | 1        | Yes        | Allow RFID card access                 |
| `ALLOW_BIO`       | Boolean | 1        | Yes        | Allow biometric access                 |
| `ALLOW_KEYPAD`    | Boolean | 1        | Yes        | Allow keypad code access               |
| `CODIGO`          | Text    | 20       | No         | Keypad access code                     |

### Example File

```
# Arquivo de colaboradores - Turnkey Emulator
# Data: 20/10/2025
12345678901|João da Silva|1001|12345678901|01/01/2025|31/12/2025|1|1|1|1|1234
98765432101|Maria Santos|1002|98765432101|01/01/2025||1|1|0|1|5678
11122233344|Pedro Oliveira|1003||15/03/2025|15/03/2026|1|0|1|1|
|Ana Costa|1004|22233344455|||0|1|1|1|9999
```

### Validation Rules

- `PIS`: Must be 11 digits or empty
- `MATRICULA`: Must be unique, 3-20 characters
- `CPF`: Must be 11 digits or empty
- At least one access method must be enabled (`ALLOW_*` = 1)
- If `ALLOW_KEYPAD` = 1, `CODIGO` must be provided
- `VALIDADE_FIM` must be after `VALIDADE_INICIO`

---

## 2. Card Import/Export (`cartoes.txt`)

### Format Specification

```
NUMERO_CARTAO|MATRICULA|VALIDADE_INICIO|VALIDADE_FIM|ATIVO
```

### Field Definitions

| Field             | Type    | Length | Required | Description                       |
|-------------------|---------|--------|----------|-----------------------------------|
| `NUMERO_CARTAO`   | Text    | 20     | Yes      | RFID card number (decimal or hex) |
| `MATRICULA`       | Text    | 20     | Yes      | Associated employee ID            |
| `VALIDADE_INICIO` | Date    | -      | No       | Card valid from                   |
| `VALIDADE_FIM`    | Date    | -      | No       | Card valid until                  |
| `ATIVO`           | Boolean | 1      | Yes      | Active status                     |

### Example File

```
# Cartões RFID
00000000000011912322|1001|01/01/2025|31/12/2025|1
00000000000022823433|1002|01/01/2025||1
ABCDEF123456|1003|||1
```

### Validation Rules

- `NUMERO_CARTAO`: Must be unique, 3-20 characters
- `MATRICULA`: Must reference existing user in `colaborador.txt`
- Multiple cards can be assigned to the same user

---

## 3. Biometric Templates (`biometria.txt`)

### Format Specification

```
MATRICULA|POSICAO|TEMPLATE_BASE64
```

### Field Definitions

| Field             | Type    | Length   | Required   | Description                         |
|-------------------|---------|----------|------------|-------------------------------------|
| `MATRICULA`       | Text    | 20       | Yes        | Employee ID                         |
| `POSICAO`         | Integer | -        | Yes        | Finger position (0-9)               |
| `TEMPLATE_BASE64` | Text    | Variable | Yes        | Base64-encoded fingerprint template |

### Finger Position Codes

| Code   | Finger       |
|--------|--------------|
| 0      | Right thumb  |
| 1      | Right index  |
| 2      | Right middle |
| 3      | Right ring   |
| 4      | Right pinky  |
| 5      | Left thumb   |
| 6      | Left index   |
| 7      | Left middle  |
| 8      | Left ring    |
| 9      | Left pinky   |

### Example File

```
# Templates biométricos (Control iD format)
1001|1|AQIDBAUG...BASE64...ENCODED==
1001|6|BQYHCQ0K...BASE64...ENCODED==
1002|0|CQMNDRE0...BASE64...ENCODED==
```

### Validation Rules

- `MATRICULA`: Must reference existing user with `ALLOW_BIO = 1`
- `POSICAO`: Must be 0-9
- Maximum 2 templates per user (configurable in `[biometrics]` section)
- Template size: 500-2000 bytes (device-dependent)

---

## 4. Access Logs Export (`eventos.txt`)

### Format Specification

```
NSR|DATA_HORA|MATRICULA|CARTAO|TIPO_EVENTO|DIRECAO|DISPOSITIVO|VALIDACAO|NOME
```

### Field Definitions

| Field         | Type     | Length   | Required   | Description                            |
|---------------|----------|----------|------------|----------------------------------------|
| `NSR`         | Integer  | -        | Yes        | Sequential record number               |
| `DATA_HORA`   | DateTime | -        | Yes        | Timestamp (dd/mm/yyyy HH:MM:SS)        |
| `MATRICULA`   | Text     | 20       | No         | Employee ID                            |
| `CARTAO`      | Text     | 20       | No         | Card number used                       |
| `TIPO_EVENTO` | Integer  | -        | Yes        | Event type code (see below)            |
| `DIRECAO`     | Integer  | 1        | Yes        | Direction (1=entry, 2=exit, 0=unknown) |
| `DISPOSITIVO` | Integer  | -        | Yes        | Device ID (01-99)                      |
| `VALIDACAO`   | Text     | 1        | Yes        | Validation mode (O=online, F=offline)  |
| `NOME`        | Text     | 100      | No         | User name                              |

### Event Type Codes

| Code   | Description                         |
|--------|-------------------------------------|
| 0      | Access granted (entry)              |
| 1      | Access granted (exit)               |
| 2      | Access granted (both directions)    |
| 10     | Access denied (unknown user)        |
| 11     | Access denied (expired card)        |
| 12     | Access denied (inactive user)       |
| 13     | Access denied (invalid time window) |
| 20     | Rotation timeout                    |
| 21     | Rotation cancelled by user          |
| 30     | Invalid card read                   |
| 31     | Biometric verification failed       |
| 40     | Device boot                         |
| 41     | Device shutdown                     |
| 50     | Configuration changed               |

### Example File

```
# Log de eventos - Dispositivo 01
# Período: 01/10/2025 a 20/10/2025
1|20/10/2025 08:15:23|1001|00000000000011912322|0|1|01|O|João da Silva
2|20/10/2025 08:15:45|1001||1|2|01|O|João da Silva
3|20/10/2025 09:30:12|1002|00000000000022823433|0|1|01|F|Maria Santos
4|20/10/2025 10:45:00||ABCDEF999999|10|1|01|O|
5|20/10/2025 12:30:15|1003||11|1|01|F|Pedro Oliveira
```

### Export Filters

**By NSR Range:**
```bash
# Export events NSR 1000-2000
turnkey-cli export eventos --nsr-start 1000 --nsr-end 2000
```

**By Date Range:**
```bash
# Export events from last 7 days
turnkey-cli export eventos --start-date "13/10/2025" --end-date "20/10/2025"
```

**By Event Type:**
```bash
# Export only access denials
turnkey-cli export eventos --type 10,11,12,13
```

---

## 5. Configuration Export (`configuracoes.txt`)

### Format Specification

Configuration is exported as TOML format (same as `config/default.toml`).

```toml
# Turnkey Configuration Export
# Device: 01
# Date: 20/10/2025 14:30:00

[device]
id = 1
model = "Turnkey Emulator"
firmware_version = "1.0.0"
protocol_version = "1.0.0.15"
display_message = "DIGITE SEU CÓDIGO"

[mode]
online = true
status_online = true
event_online = true
smart_mode = true
local_registration = false
save_reference = true
fallback_offline = true
fallback_timeout = 3000

# ... (all other sections)
```

### Import/Export Commands

```bash
# Export current configuration
turnkey-cli config export > config-backup-20251020.toml

# Import configuration
turnkey-cli config import config-backup-20251020.toml

# Validate configuration without applying
turnkey-cli config validate config-backup-20251020.toml
```

---

## 6. AFD Format (Arquivo Fonte de Dados)

### Overview

AFD is the Brazilian legal standard for time-tracking and access control logs (Portaria 1510/2009). While primarily used for timekeeping, the Turnkey emulator can export access logs in AFD format for compliance.

### Format Specification

AFD is a fixed-width text format with record types.

#### Record Type 1: Header

```
1|CNPJ|CEI|RAZAO_SOCIAL|INICIO|FIM|DATA_GERACAO
```

**Example:**
```
1|12345678000190||EMPRESA EXEMPLO LTDA|01102025|20102025|20102025143000
```

#### Record Type 2: Device Identification

```
2|TIPO|SERIE|MODELO|VERSAO
```

**Example:**
```
2|1|TK00000001|Turnkey Emulator|1.0.0
```

#### Record Type 3: Access Event

```
3|NSR|DATA|HORA|PIS
```

**Example:**
```
3|000001|20102025|081523|12345678901
3|000002|20102025|093012|98765432101
```

#### Record Type 9: Trailer

```
9|TOTAL_REGISTROS
```

**Example:**
```
9|4
```

### Complete AFD Example

```
1|12345678000190||EMPRESA EXEMPLO LTDA|01102025|20102025|20102025143000
2|1|TK00000001|Turnkey Emulator|1.0.0
3|000001|20102025|081523|12345678901
3|000002|20102025|093012|98765432101
3|000003|20102025|103000|11122233344
9|5
```

### Export AFD

```bash
# Export AFD for date range
turnkey-cli export afd \
  --cnpj "12345678000190" \
  --razao-social "Empresa Exemplo LTDA" \
  --start-date "01/10/2025" \
  --end-date "20/10/2025" \
  --output afd-outubro-2025.txt
```

### Validation

AFD files must:
- Start with record type 1 (header)
- End with record type 9 (trailer)
- Have sequential NSR numbers
- Include only valid PIS numbers (11 digits)
- Have correct record count in trailer

---

## 7. Employer Data (`empregador.txt`)

### Format Specification

```
CNPJ|RAZAO_SOCIAL|CEI|ENDERECO|CIDADE|ESTADO|CEP|TELEFONE
```

### Field Definitions

| Field          | Type    | Length   | Required   | Description    |
|----------------|---------|----------|------------|----------------|
| `CNPJ`         | Numeric | 14       | Yes        | Company CNPJ   |
| `RAZAO_SOCIAL` | Text    | 200      | Yes        | Company name   |
| `CEI`          | Numeric | 12       | No         | CEI number     |
| `ENDERECO`     | Text    | 200      | No         | Street address |
| `CIDADE`       | Text    | 100      | No         | City           |
| `ESTADO`       | Text    | 2        | No         | State (UF)     |
| `CEP`          | Numeric | 8        | No         | Postal code    |
| `TELEFONE`     | Text    | 20       | No         | Phone number   |

### Example File

```
# Dados do empregador
12345678000190|Empresa Exemplo LTDA||Rua das Flores, 123|São Paulo|SP|01234567|(11) 3456-7890
```

---

## 8. Bulk Import Command

### Import All Data

```bash
# Import all data files from directory
turnkey-cli import bulk ./import-data/
```

**Directory Structure:**
```
import-data/
├── empregador.txt
├── colaborador.txt
├── cartoes.txt
├── biometria.txt
└── configuracoes.toml
```

**Import Order:**
1. `empregador.txt` (employer data)
2. `colaborador.txt` (users)
3. `cartoes.txt` (cards)
4. `biometria.txt` (biometric templates)
5. `configuracoes.toml` (configuration - optional)

### Transaction Behavior

- All imports are wrapped in database transaction
- If any file fails validation, entire import is rolled back
- Duplicate records are skipped with warning (based on unique constraints)
- Progress is shown for large files (>1000 records)

### Validation Report

```bash
# Validate files without importing
turnkey-cli import validate ./import-data/

# Output:
Validating empregador.txt... OK
Validating colaborador.txt... 3 errors found:
  Line 15: Invalid PIS number '123'
  Line 42: MATRICULA '1002' duplicated
  Line 58: Missing required field NOME
Validating cartoes.txt... OK
Validating biometria.txt... WARNING: 2 templates exceed recommended size

Validation: FAILED (3 errors, 1 warning)
```

---

## 9. Export Commands Reference

### Export Users

```bash
# Export all users
turnkey-cli export colaborador --output colaborador-backup.txt

# Export only active users
turnkey-cli export colaborador --filter active --output active-users.txt

# Export with validity filter
turnkey-cli export colaborador --valid-on "20/10/2025" --output valid-users.txt
```

### Export Cards

```bash
# Export all cards
turnkey-cli export cartoes --output cartoes-backup.txt

# Export cards for specific users
turnkey-cli export cartoes --matricula 1001,1002,1003 --output selected-cards.txt
```

### Export Biometrics

```bash
# Export all biometric templates
turnkey-cli export biometria --output bio-backup.txt

# Export for specific users
turnkey-cli export biometria --matricula 1001,1002 --output selected-bio.txt
```

### Export Access Logs

```bash
# Export all events
turnkey-cli export eventos --output eventos-full.txt

# Export with date range
turnkey-cli export eventos \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output eventos-outubro.txt

# Export with NSR range
turnkey-cli export eventos \
  --nsr-start 1000 \
  --nsr-end 2000 \
  --output eventos-range.txt

# Export specific event types
turnkey-cli export eventos \
  --type 10,11,12,13 \
  --output eventos-denials.txt

# Export AFD format
turnkey-cli export afd \
  --cnpj "12345678000190" \
  --razao-social "Empresa Exemplo" \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output afd-outubro.txt
```

---

## 10. Import Error Handling

### Error Codes

| Code   | Description            | Recovery                            |
|--------|------------------------|-------------------------------------|
| `E001` | Invalid file format    | Check field separators and encoding |
| `E002` | Missing required field | Add required field value            |
| `E003` | Duplicate key          | Remove duplicate or update existing |
| `E004` | Invalid reference      | Ensure referenced record exists     |
| `E005` | Validation failed      | Check field format and constraints  |
| `E006` | File not found         | Verify file path                    |
| `E007` | Permission denied      | Check file permissions              |

### Dry-Run Mode

```bash
# Simulate import without writing to database
turnkey-cli import colaborador colaborador.txt --dry-run

# Output:
Would import 150 users:
  - New: 120
  - Updated: 25
  - Skipped (duplicate): 5
  - Errors: 0
```

---

## 11. Data Migration

### From Legacy Systems

```bash
# Convert from legacy format (CSV with semicolon separator)
turnkey-cli convert \
  --from-format csv \
  --separator ';' \
  --input legacy-users.csv \
  --output colaborador.txt

# Map field names
turnkey-cli convert \
  --from-format csv \
  --field-mapping "id:MATRICULA,name:NOME,active:ATIVO" \
  --input legacy.csv \
  --output colaborador.txt
```

### To External Systems

```bash
# Export to generic CSV
turnkey-cli export colaborador \
  --format csv \
  --separator ',' \
  --output users.csv

# Export to JSON
turnkey-cli export colaborador \
  --format json \
  --output users.json
```

**JSON Format:**
```json
{
  "export_date": "2025-10-20T14:30:00",
  "device_id": 1,
  "users": [
    {
      "pis": "12345678901",
      "nome": "João da Silva",
      "matricula": "1001",
      "cpf": "12345678901",
      "validade_inicio": "2025-01-01",
      "validade_fim": "2025-12-31",
      "ativo": true,
      "allow_card": true,
      "allow_bio": true,
      "allow_keypad": true,
      "codigo": "1234"
    }
  ]
}
```

---

## 12. Schema Definitions

### JSON Schema for Validation

See `schemas/` directory for complete JSON Schema definitions:
- `schemas/colaborador.schema.json` - User import validation
- `schemas/cartoes.schema.json` - Card import validation
- `schemas/biometria.schema.json` - Biometric import validation
- `schemas/eventos.schema.json` - Event export validation

### Example JSON Schema Usage

```bash
# Validate JSON export against schema
turnkey-cli validate \
  --schema schemas/colaborador.schema.json \
  --data users.json

# Output:
Schema validation: PASSED
Records validated: 150
```

---

## References

- [Emulator Configuration](emulator-configuration.md) - Configuration file format
- [Emulator Modes](emulator-modes.md) - ONLINE/OFFLINE operation
- [Emulator Architecture](emulator-architecture.md) - Database schema
- Portaria 1510/2009 - AFD legal requirements (Brazil)

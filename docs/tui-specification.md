# Turnkey TUI Specification

## Overview

The Turnkey Terminal User Interface (TUI) provides a realistic, interactive simulation of a physical turnstile using `ratatui`. It mimics the appearance and behavior of real Brazilian access control devices (Primme SF, Henry Lumen).

## Technology Stack

- **Framework**: [ratatui](https://github.com/ratatui-org/ratatui) 0.27+
- **Backend**: crossterm (for terminal control and input)
- **Terminal**: Supports all modern terminals (xterm, gnome-terminal, iTerm2, Windows Terminal, etc.)
- **Minimum Size**: 80 columns × 24 lines (120×30 recommended)

---

## Layout Overview

The TUI is divided into two main columns:

```
┌─────────────────────────────────────────────────────┬─────────────────────┐
│                                                     │                     │
│            EMULATOR COLUMN (70%)                    │   LOGS COLUMN (30%) │
│                                                     │                     │
└─────────────────────────────────────────────────────┴─────────────────────┘
```

### Column Proportions

- **Emulator Column**: 70% of terminal width
  - Display LCD
  - Keypad
  - Status Bar

- **Logs Column**: 30% of terminal width
  - Real-time event log
  - Scrollable history
  - Timestamp for each entry

---

## Emulator Column (70%)

### Layout Structure

```
┌─────────────────────────────────────────────────────┐
│                  DISPLAY LCD (30%)                  │
├─────────────────────────────────────────────────────┤
│                  KEYPAD (60%)                       │
├─────────────────────────────────────────────────────┤
│                  STATUS BAR (10%)                   │
└─────────────────────────────────────────────────────┘
```

---

### Display LCD Component

#### Specifications

- **Size**: 2 lines × 40 columns (matches real Primme SF display)
- **Border**: Double border (`BorderType::Double`)
- **Background**: Blue (`Color::Blue`)
- **Foreground**: White (`Color::White`)
- **Font Style**: Bold for messages
- **Alignment**: Centered horizontally

#### Visual Design

```
┌═════════════════════════════════════════════════════════════════════════╗
║                              DISPLAY LCD                                ║
╠═════════════════════════════════════════════════════════════════════════╣
║                          DIGITE SEU CÓDIGO                              ║
║                          Entrada: [1234____]                            ║
║                                                                         ║
║              ONLINE | MOCK | IP: 192.168.0.100:3000                     ║
╚═════════════════════════════════════════════════════════════════════════╝
```

#### Layout

| Line   | Content                                              | Alignment   |
|--------|------------------------------------------------------|-------------|
| 1      | Main message (e.g., "DIGITE SEU CÓDIGO")             | Centered    |
| 2      | Input buffer or status (e.g., "Entrada: [1234____]") | Centered    |
| 3      | Empty (spacing)                                      | -           |
| 4      | Status info (mode, readers, IP)                      | Centered    |

#### Color Themes

**Default Theme:**
```rust
Style::default()
    .bg(Color::Blue)
    .fg(Color::White)
    .add_modifier(Modifier::BOLD)
```

**Dark Theme:**
```rust
Style::default()
    .bg(Color::Black)
    .fg(Color::Green)
    .add_modifier(Modifier::BOLD)
```

**Light Theme:**
```rust
Style::default()
    .bg(Color::White)
    .fg(Color::Black)
```

**Green (LCD-style) Theme:**
```rust
Style::default()
    .bg(Color::Black)
    .fg(Color::Green)
    .add_modifier(Modifier::BOLD)
```

#### Display States

**IDLE State:**
```
┌═══════════════════════════════════════════════════╗
║              DIGITE SEU CÓDIGO                    ║
║                                                   ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**ENTERING Code State:**
```
┌═══════════════════════════════════════════════════╗
║              DIGITE SEU CÓDIGO                    ║
║            Entrada: [1234____]                    ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**VALIDATING State:**
```
┌═══════════════════════════════════════════════════╗
║                 AGUARDE...                        ║
║            Validando código                       ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**GRANTED State:**
```
┌═══════════════════════════════════════════════════╗
║             ACESSO LIBERADO                       ║
║            Bem-vindo, João                        ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**DENIED State (Red background):**
```
┌═══════════════════════════════════════════════════╗
║             ACESSO NEGADO                         ║  (Red BG)
║           Código inválido                         ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**WAITING_ROTATION State:**
```
┌═══════════════════════════════════════════════════╗
║           AGUARDANDO PASSAGEM                     ║
║            Gire a catraca                         ║
║      ONLINE | MOCK | IP: 192.168.0.100            ║
╚═══════════════════════════════════════════════════╝
```

**OFFLINE State (Yellow background):**
```
┌═══════════════════════════════════════════════════╗
║              MODO OFFLINE                         ║  (Yellow BG)
║         Servidor indisponível                     ║
║     OFFLINE | MOCK | Última sincr: 14:30          ║
╚═══════════════════════════════════════════════════╝
```

---

### Keypad Component

#### Specifications

- **Layout**: 4 rows × 3 columns (numeric keypad)
- **Border**: Single border (`BorderType::Plain`)
- **Background**: Gray (`Color::Gray`)
- **Keys**: 0-9, *, #
- **Buttons**: ENTER, CANCEL, CLEAR

#### Visual Design

```
┌─────────────────────────────────────────────────────┐
│                    TECLADO NUMÉRICO                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│              ┌─────┬─────┬─────┐                    │
│              │  1  │  2  │  3  │                    │
│              ├─────┼─────┼─────┤                    │
│              │  4  │  5  │  6  │                    │
│              ├─────┼─────┼─────┤                    │
│              │  7  │  8  │  9  │                    │
│              ├─────┼─────┼─────┤                    │
│              │  *  │  0  │  #  │                    │
│              └─────┴─────┴─────┘                    │
│                                                     │
│        [ENTER]      [CANCEL]      [CLEAR]           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

#### Key Layout

```rust
const KEYPAD_LAYOUT: [[char; 3]; 4] = [
    ['1', '2', '3'],
    ['4', '5', '6'],
    ['7', '8', '9'],
    ['*', '0', '#'],
];
```

#### Key Mapping

| Key       | Physical Keyboard   | Function           |
|-----------|---------------------|--------------------|
| `1` - `9` | 1-9 (numeric)       | Digit entry        |
| `0`       | 0 (numeric)         | Digit entry        |
| `*`       | * or 8 (Shift+8)    | Clear / Cancel     |
| `#`       | # or 3 (Shift+3)    | Enter              |
| ENTER     | Enter / Return      | Confirm input      |
| CANCEL    | Escape              | Cancel input       |
| CLEAR     | Backspace           | Clear input buffer |

#### Key States

**Normal State:**
```
┌─────┐
│  5  │  (Gray background, black text)
└─────┘
```

**Pressed State (highlighted):**
```
┌─────┐
│  5  │  (Yellow background, black text, bold)
└─────┘
```

**Disabled State:**
```
┌─────┐
│  5  │  (Dark gray background, dim text)
└─────┘
```

#### Button Styling

```rust
// Normal button
Style::default()
    .bg(Color::Gray)
    .fg(Color::Black)

// Pressed button
Style::default()
    .bg(Color::Yellow)
    .fg(Color::Black)
    .add_modifier(Modifier::BOLD)

// Action buttons (ENTER, CANCEL, CLEAR)
Style::default()
    .bg(Color::DarkGray)
    .fg(Color::White)
    .add_modifier(Modifier::BOLD)
```

---

### Status Bar Component

#### Specifications

- **Position**: Bottom of emulator column
- **Height**: 2 lines
- **Background**: Dark Gray
- **Foreground**: White

#### Visual Design

```
┌─────────────────────────────────────────────────────┐
│ Leitoras: RFID✓ BIO✗ KEYPAD✓ WIEGAND✗               │
│ Eventos: 68 | Colaboradores: 16 | Espaço: 95.9%     │
└─────────────────────────────────────────────────────┘
```

#### Information Display

**Line 1:** Reader Status
- RFID: ✓ (enabled) or ✗ (disabled)
- BIO: ✓ or ✗
- KEYPAD: ✓ or ✗
- WIEGAND: ✓ or ✗

**Line 2:** Statistics
- Eventos: Total access events
- Colaboradores: Total registered users
- Espaço: Available storage percentage

#### Color Coding

- **Reader Enabled**: Green checkmark (`✓`)
- **Reader Disabled**: Red cross (`✗`)
- **Storage > 90%**: Green
- **Storage 50-90%**: Yellow
- **Storage < 50%**: Red (warning)

---

## Logs Column (30%)

### Specifications

- **Type**: Scrollable list
- **Border**: Single border
- **Title**: "LOGS"
- **Auto-scroll**: Yes (configurable)
- **Max Entries**: 1000 (configurable)
- **Timestamp**: HH:MM:SS format

### Visual Design

```
┌─────────────────────┐
│       LOGS          │
├─────────────────────┤
│ [14:30:00] Boot OK  │
│ [14:30:01] TCP: ON  │
│ [14:30:02] ONLINE   │
│ [14:30:05] Card OK  │
│ [14:30:06] REQ→SRV  │
│ [14:30:07] GRANT    │
│ [14:30:08] Liberado │
│ [14:30:10] Girou    │
│ [14:30:11] IDLE     │
│ [14:30:15] Card OK  │
│ [14:30:16] REQ→SRV  │
│ [14:30:17] DENY     │
│ [14:30:18] Negado   │
│ [14:30:19] IDLE     │
│ ...                 │
├─────────────────────┤
│ Tab: Focus | ↑↓:Nav │
└─────────────────────┘
```

### Log Entry Format

```rust
struct LogEntry {
    timestamp: DateTime<Utc>,
    level: LogLevel,
    message: String,
}

enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}
```

### Log Entry Colors

| Level   | Color   | Example              |
|---------|---------|----------------------|
| Info    | White   | `[14:30:00] Boot OK` |
| Success | Green   | `[14:30:07] GRANT`   |
| Warning | Yellow  | `[14:30:05] TIMEOUT` |
| Error   | Red     | `[14:30:10] ERROR`   |

### Scrollbar

```
┌───────┐
│ Entry │ ◄── Scrollbar thumb
│ Entry │
│ Entry │
│ Entry │
│ Entry │
│ Entry │
└───────┘
```

**Scrollbar Indicator:**
- Visible when log entries > visible lines
- Position indicates current scroll position
- Style: `│` (vertical bar)

### Log Messages

**Boot Sequence:**
```
[14:30:00] Inicializando...
[14:30:00] Config carregada
[14:30:01] DB conectado
[14:30:01] Leitoras OK
[14:30:02] TCP server: 0.0.0.0:3000
[14:30:02] MODO ONLINE
[14:30:02] Pronto
```

**Access Granted:**
```
[14:30:05] Cartão: 12345678
[14:30:06] REQ→SRV (código)
[14:30:07] ←SRV GRANT EXIT
[14:30:07] Liberado (João)
[14:30:08] Aguardando rotação
[14:30:10] Rotação OK
[14:30:11] Estado: IDLE
```

**Access Denied:**
```
[14:30:15] Código: 9999
[14:30:16] REQ→SRV (código)
[14:30:17] ←SRV DENY
[14:30:17] Negado (inválido)
[14:30:19] Estado: IDLE
```

**Connection Lost:**
```
[14:35:00] Conexão perdida
[14:35:01] Tentando reconectar...
[14:35:03] Falha (tentativa 1/3)
[14:35:06] Falha (tentativa 2/3)
[14:35:09] Falha (tentativa 3/3)
[14:35:09] MODO OFFLINE ativado
```

---

## Keyboard Shortcuts

### Global Shortcuts

| Key             | Action                        |
|-----------------|-------------------------------|
| `q` or `Ctrl+C` | Quit emulator                 |
| `Tab`           | Switch focus (Display ↔ Logs) |
| `F1`            | Show help                     |
| `F5`            | Reload configuration          |
| `F8`            | Export events                 |
| `F9`            | Import data                   |
| `F10`           | Settings menu                 |

### Keypad Shortcuts (when Display focused)

| Key         | Action             |
|-------------|--------------------|
| `0`-`9`     | Digit entry        |
| `*`         | Star key           |
| `#`         | Hash key           |
| `Enter`     | Confirm input      |
| `Escape`    | Cancel input       |
| `Backspace` | Clear last digit   |
| `Ctrl+U`    | Clear entire input |

### Logs Panel Shortcuts (when Logs focused)

| Key           | Action                 |
|---------------|------------------------|
| `↑` or `k`    | Scroll up              |
| `↓` or `j`    | Scroll down            |
| `Page Up`     | Scroll page up         |
| `Page Down`   | Scroll page down       |
| `Home` or `g` | Scroll to top          |
| `End` or `G`  | Scroll to bottom       |
| `/`           | Search in logs         |
| `n`           | Next search result     |
| `N`           | Previous search result |

---

## Animation and Feedback

### Visual Feedback

**Key Press Animation:**
1. Key background: Gray → Yellow (100ms)
2. Key returns to Gray (100ms)

**Access Granted Animation:**
1. Display background: Blue → Green (200ms)
2. Display message: "ACESSO LIBERADO" (bold)
3. Hold for 3-5 seconds
4. Fade back to Blue (200ms)

**Access Denied Animation:**
1. Display background: Blue → Red (200ms)
2. Display message: "ACESSO NEGADO" (bold)
3. Display flashes Red/Blue 3 times (500ms each)
4. Return to Blue (200ms)

### Audio Feedback

**Beep Sounds (simulated via text):**
- **Key Press**: Short beep (♪)
- **Access Granted**: Double beep (♪♪)
- **Access Denied**: Long beep (♪♪♪)
- **Error**: Triple beep (♪♪♪)

**Note:** Actual audio can be played using system beep or audio library.

---

## Responsive Design

### Minimum Terminal Size

- **Width**: 80 columns
- **Height**: 24 lines

**Behavior when terminal < minimum:**
- Display warning message
- Disable TUI
- Wait for resize

### Optimal Terminal Size

- **Width**: 120 columns
- **Height**: 30 lines

### Layout Adaptation

| Terminal Width   | Emulator Column   | Logs Column   |
|------------------|-------------------|---------------|
| < 80 cols        | Warning           | Warning       |
| 80-99 cols       | 75%               | 25%           |
| 100-119 cols     | 70%               | 30%           |
| ≥ 120 cols       | 70%               | 30%           |

---

## Implementation Notes

### Rendering Loop

```rust
loop {
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),  // Emulator
                Constraint::Percentage(30),  // Logs
            ])
            .split(f.size());

        render_emulator(f, chunks[0], &app_state);
        render_logs(f, chunks[1], &logs);
    })?;

    if event::poll(Duration::from_millis(16))? {
        handle_input(&mut app_state)?;
    }
}
```

### Frame Rate

- **Target**: 60 FPS (16ms per frame)
- **Actual**: 30-60 FPS depending on terminal

### Performance

- **Idle CPU**: < 1%
- **Active Rendering**: < 5%
- **Memory**: < 5 MB

---

## Accessibility

### Color Blindness Support

- Provide "High Contrast" theme
- Use symbols (✓, ✗) in addition to colors
- Text-based status indicators

### Screen Reader Support

- Alt text for visual elements (future enhancement)
- Keyboard-only navigation

---

## Testing

### Manual Testing

1. Launch emulator: `cargo run --bin turnkey-cli`
2. Verify display renders correctly
3. Test all keyboard shortcuts
4. Enter codes and verify validation
5. Test log scrolling
6. Resize terminal and verify responsiveness

### Automated Testing

```bash
# UI snapshot testing (future)
cargo test --test tui_snapshots

# Input simulation
cargo test --test keyboard_input

# Rendering tests
cargo test --test tui_rendering
```

---

## References

- [ratatui Documentation](https://ratatui.rs/)
- [Emulator Architecture](emulator-architecture.md)
- [Emulator Configuration](emulator-configuration.md)
- [Primme SF Manual](Catraca Sf item 01.pdf)

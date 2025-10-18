# Arquitetura Completa - Emulador de Controle de Acesso Turnkey

## 1. Estrutura de Diretórios e Organização do Projeto

```
turnkey/
├── Cargo.toml                          # Raiz do workspace
├── Cargo.lock
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .gitignore
├── .env.example
├── rust-toolchain.toml                 # Versão fixada do Rust (1.90+)
├── deny.toml                           # Cargo deny para auditoria de segurança
├── Cross.toml                          # Configuração de compilação cruzada
├── Makefile                            # Automação de build
├── build.rs                            # Script de build principal
│
├── .cargo/
│   └── config.toml                    # Configuração local do cargo
│
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # Pipeline de CI/CD
│   │   ├── security-audit.yml         # Auditoria de segurança
│   │   └── release.yml                # Automação de release
│   └── dependabot.yml                 # Atualizações automatizadas de dependências
│
├── crates/                             # Membros do workspace
│   ├── turnkey-core/                  # Funcionalidade principal do emulador
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs               # Tratamento centralizado de erros
│   │       ├── types.rs               # Tipos compartilhados
│   │       └── constants.rs           # Constantes do sistema
│   │
│   ├── turnkey-protocol/              # Implementação do protocolo Henry
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── message.rs             # Estruturas de mensagem
│   │       ├── parser.rs              # Analisador de mensagens
│   │       ├── builder.rs             # Construtor de mensagens
│   │       ├── codec.rs               # Codec Tokio
│   │       ├── checksum.rs            # Cálculo de soma de verificação
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── access.rs          # Comandos de acesso
│   │           ├── config.rs          # Comandos de configuração
│   │           └── management.rs      # Comandos de gerenciamento
│   │
│   ├── turnkey-hardware/              # Abstração de hardware
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # Script de build para SDKs
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Traits base
│   │       ├── manager.rs             # Gerenciador de hardware
│   │       ├── discovery.rs           # Auto-descoberta USB
│   │       └── events.rs              # Sistema de eventos
│   │
│   ├── turnkey-rfid/                  # Leitores RFID/NFC
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Trait CardReader
│   │       ├── acr122u/
│   │       │   ├── mod.rs
│   │       │   ├── driver.rs          # Driver ACR122U
│   │       │   ├── commands.rs        # Comandos APDU
│   │       │   └── monitor.rs         # Monitoramento de cartões
│   │       ├── rc522/
│   │       │   └── driver.rs          # Suporte RC522 (SPI)
│   │       └── mock/
│   │           └── mock_reader.rs     # Mock para testes
│   │
│   ├── turnkey-biometric/             # Leitores biométricos
│   │   ├── Cargo.toml
│   │   ├── build.rs                   # Build do SDK iDBio
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Trait BiometricReader
│   │       ├── idbio/
│   │       │   ├── mod.rs
│   │       │   ├── driver.rs          # Driver iDBio
│   │       │   ├── sdk.rs             # Bindings FFI
│   │       │   └── protocol.rs        # Protocolo iDBio
│   │       ├── digital_persona/       # Suporte futuro
│   │       │   └── driver.rs
│   │       ├── template_manager.rs    # Gerenciamento de templates
│   │       └── mock/
│   │           └── mock_biometric.rs  # Mock para testes
│   │
│   ├── turnkey-keypad/                # Teclados numéricos
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs              # Trait Keypad
│   │       ├── usb_hid/
│   │       │   └── driver.rs          # Teclados USB HID
│   │       ├── matrix/
│   │       │   └── driver.rs          # Teclado matricial GPIO
│   │       ├── wiegand/
│   │       │   └── driver.rs          # Teclados Wiegand
│   │       └── mock/
│   │           └── mock_keypad.rs     # Mock para testes
│   │
│   ├── turnkey-turnstile/             # Controle de catraca
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── controller.rs          # Controlador principal
│   │       ├── gpio/
│   │       │   └── raspberry_pi.rs    # GPIO Raspberry Pi
│   │       ├── relay/
│   │       │   ├── usb_relay.rs       # Placas de relé USB
│   │       │   └── modbus.rs          # Relés Modbus
│   │       ├── sensors/
│   │       │   ├── rotation.rs        # Sensor de rotação
│   │       │   └── position.rs        # Sensor de posição
│   │       └── state_machine.rs       # Máquina de estados
│   │
│   ├── turnkey-storage/               # Camada de persistência
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── database.rs            # Abstração de banco de dados
│   │       ├── sqlite/
│   │       │   ├── mod.rs
│   │       │   ├── connection.rs      # Pool de conexões
│   │       │   └── migrations.rs      # Sistema de migrações
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── user.rs            # Modelo de usuário
│   │       │   ├── card.rs            # Modelo de cartão
│   │       │   ├── access_log.rs      # Logs de acesso
│   │       │   └── device_state.rs    # Estado do dispositivo
│   │       ├── repository/
│   │       │   ├── mod.rs
│   │       │   ├── user_repo.rs       # Repositório de usuários
│   │       │   ├── card_repo.rs       # Repositório de cartões
│   │       │   └── log_repo.rs        # Repositório de logs
│   │       └── cache/
│   │           ├── mod.rs
│   │           └── memory.rs          # Cache em memória
│   │
│   ├── turnkey-network/               # Camada de rede
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs              # Servidor TCP
│   │       ├── connection.rs          # Gerenciamento de conexões
│   │       ├── tls.rs                 # Suporte TLS
│   │       └── protocol_handler.rs    # Manipulador de protocolo
│   │
│   ├── turnkey-emulator/              # Emuladores de dispositivos
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── device_trait.rs        # Trait de dispositivo
│   │       ├── primme_acesso/
│   │       │   ├── mod.rs
│   │       │   ├── device.rs          # Emulador Primme
│   │       │   └── features.rs        # Recursos específicos
│   │       ├── argos/
│   │       │   └── device.rs          # Emulador Argos
│   │       ├── primme_sf/
│   │       │   └── device.rs          # Emulador Primme SF
│   │       └── bridge/
│   │           ├── mod.rs
│   │           └── hardware_bridge.rs # Ponte Hardware->Protocolo
│   │
│   └── turnkey-cli/                   # Aplicação CLI
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── server.rs          # Comando servidor
│           │   ├── test.rs            # Comando teste
│           │   └── config.rs          # Comando configuração
│           └── ui/
│               ├── mod.rs
│               └── terminal.rs        # Interface TUI
│
├── vendor/                             # SDKs de terceiros
│   ├── controlid/
│   │   ├── linux-x86_64/
│   │   │   └── libidbio.so           # SDK iDBio x64
│   │   ├── linux-aarch64/
│   │   │   └── libidbio_arm64.so     # SDK iDBio ARM64
│   │   └── include/
│   │       └── idbio.h               # Headers
│   └── README.md                      # Instruções dos SDKs
│
├── config/                             # Arquivos de configuração
│   ├── default.toml                   # Configuração padrão
│   ├── development.toml               # Configuração de desenvolvimento
│   ├── production.toml                # Configuração de produção
│   ├── hardware.toml                  # Configuração de hardware
│   └── logging.toml                   # Configuração de logging
│
├── migrations/                         # Migrações SQLite
│   ├── 001_initial_schema.sql
│   ├── 002_add_users.sql
│   ├── 003_add_cards.sql
│   ├── 004_add_biometrics.sql
│   ├── 005_add_access_logs.sql
│   └── 006_add_device_states.sql
│
├── scripts/                            # Scripts auxiliares
│   ├── install-deps.sh                # Instalar dependências
│   ├── setup-hardware.sh              # Configurar hardware
│   ├── generate-keys.sh               # Gerar chaves TLS
│   └── cross-compile.sh               # Compilação cruzada
│
├── tests/                              # Testes de integração
│   ├── common/
│   │   └── mod.rs                     # Utilitários de teste
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
├── docs/                               # Documentação
│   ├── architecture.md
│   ├── hardware-setup.md
│   ├── api-reference.md
│   └── troubleshooting.md
│
└── examples/                           # Exemplos de uso
    ├── basic_server.rs
    ├── hardware_discovery.rs
    ├── biometric_enrollment.rs
    └── stress_test.rs
```

## 2. Cargo.toml Principal (Raiz do Workspace)

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
authors = ["Equipe Turnkey <team@turnkey-emulator.com>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/marmota-alpina/turnkey"

[workspace.dependencies]
# Runtime Assíncrono - Tokio é a escolha padrão para async em Rust
tokio = { version = "1.40", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec", "net"] }
async-trait = "0.1"

# Serialização - Serde é o padrão de facto
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_repr = "0.1"
bincode = "1.3"

# Tratamento de Erros - thiserror para erros tipados, anyhow para aplicações
thiserror = "1.0"
anyhow = "1.0"

# Logging - tracing é mais moderno e estruturado que log
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"

# Banco de Dados - SQLx com SQLite para simplicidade e performance
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }

# Configuração - config-rs é flexível e bem mantido
config = { version = "0.14", features = ["toml"] }
toml = "0.8"

# Data/Hora - chrono é o padrão
chrono = { version = "0.4", features = ["serde"] }

# Hardware/USB - rusb para USB, serialport para serial
rusb = "0.9"
serialport = "4.5"
hidapi = "2.6"

# Cartão Inteligente - pcsc para leitores de cartão
pcsc = "2.8"

# GPIO - rppal para Raspberry Pi
rppal = { version = "0.19", optional = true }

# Rede
bytes = "1.7"
futures = "0.3"

# Utilitários
uuid = { version = "1.10", features = ["v4", "serde"] }
dashmap = "6.1"
parking_lot = "0.12"
crossbeam-channel = "0.5"

# Testes
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

/// Informação de versão
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIME: &str = env!("VERGEN_BUILD_TIMESTAMP");
```

### crates/turnkey-core/src/error.rs

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Erro de protocolo: {0}")]
    Protocol(String),

    #[error("Erro de hardware: {0}")]
    Hardware(String),

    #[error("Erro de banco de dados: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Erro de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de configuração: {0}")]
    Config(String),

    #[error("Falha na autenticação")]
    AuthenticationFailed,

    #[error("Dispositivo não encontrado: {0}")]
    DeviceNotFound(String),

    #[error("Tempo limite excedido")]
    Timeout,

    #[error("Transição de estado inválida")]
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

/// Trait base para todos os dispositivos de hardware
#[async_trait]
pub trait HardwareDevice: Send + Sync + Any {
    /// Identificador único do dispositivo
    fn device_id(&self) -> &str;

    /// Tipo do dispositivo
    fn device_type(&self) -> DeviceType;

    /// Conectar ao dispositivo
    async fn connect(&mut self) -> Result<()>;

    /// Desconectar do dispositivo
    async fn disconnect(&mut self) -> Result<()>;

    /// Verificar se está conectado
    async fn is_connected(&self) -> bool;

    /// Obter informações do dispositivo
    async fn get_info(&self) -> DeviceInfo;

    /// Reiniciar dispositivo
    async fn reset(&mut self) -> Result<()>;

    /// Para downcast
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

## 9. Configuração de Hardware (config/hardware.toml)

```toml
# Configuração de Hardware Turnkey
# Suporta múltiplos dispositivos simultâneos

[general]
auto_discovery = true
discovery_interval = 10  # segundos
mock_mode = false  # true para testes sem hardware

# === Leitores RFID/NFC ===
[[rfid_readers]]
id = "leitor_entrada"
type = "acr122u"
enabled = true
auto_connect = true

[rfid_readers.config]
port = "auto"  # auto-detectar ou especificar porta USB
led_mode = "auto"
buzzer = true
poll_interval = 250  # ms

[rfid_readers.mifare]
default_key_a = "FFFFFFFFFFFF"
default_key_b = "FFFFFFFFFFFF"
sector = 1
block = 4

[[rfid_readers]]
id = "leitor_saida"
type = "rc522"
enabled = false

[rfid_readers.config]
spi_bus = 0
spi_device = 0
reset_pin = 25
irq_pin = 24

# === Leitores Biométricos ===
[[biometric_readers]]
id = "biometrico_principal"
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
capture = "azul"
success = "verde"
failure = "vermelho"

# === Teclados ===
[[keypads]]
id = "teclado_principal"
type = "usb_hid"
enabled = true

[keypads.config]
vendor_id = 0x1234
product_id = 0x5678
timeout = 30000  # ms para tempo limite de digitação
min_pin_length = 4
max_pin_length = 8

[keypads.feedback]
beep_on_press = true
mask_display = true
mask_char = "*"

[[keypads]]
id = "teclado_wiegand"
type = "wiegand"
enabled = false

[keypads.config]
data0_pin = 17
data1_pin = 18
bits = 26  # 26 ou 34 bits Wiegand

# === Controladores de Catraca ===
[turnstile]
enabled = true
type = "placa_rele"  # placa_rele, gpio, modbus

[turnstile.relay_board]
port = "/dev/ttyUSB0"
baudrate = 9600
entry_relay = 1
exit_relay = 2
timeout = 5  # segundos

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

# === Mapeamento de Dispositivos ===
# Mapeia IDs físicos para IDs do sistema
[mapping]
cards = [
    { uid = "04A1B2C3D4E5F6", system_id = "12345678" },
    { uid = "04D5E6F7A8B9C0", system_id = "87654321" }
]

# === Segurança ===
[security]
encrypt_templates = true
secure_storage = true
wipe_on_tamper = false
```

## 10. Configuração de Logging (config/logging.toml)

```toml
# Configuração de Logging

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
# Níveis de log por módulo
turnkey_core = "debug"
turnkey_protocol = "debug"
turnkey_hardware = "info"
turnkey_rfid = "debug"
turnkey_biometric = "debug"
turnkey_storage = "warn"
turnkey_network = "info"

[structured]
# Campos estruturados adicionais
add_hostname = true
add_process_id = true
add_thread_id = true
add_module_path = true

[events]
# Eventos específicos para log
card_read = true
biometric_capture = true
access_granted = true
access_denied = true
device_connected = true
device_error = true
protocol_error = true

[performance]
# Métricas de desempenho
log_slow_queries = true
slow_query_threshold = 100  # ms
log_memory_usage = true
memory_log_interval = 60  # segundos

[audit]
# Trilha de auditoria
enabled = true
file = "logs/audit.log"
include_all_access = true
include_config_changes = true
include_admin_actions = true
```

## 11. Makefile

```makefile
# Emulador de Controle de Acesso Turnkey - Makefile
RUST_VERSION := 1.90
CARGO := cargo
CROSS := cross
TARGET_LINUX_X64 := x86_64-unknown-linux-gnu
TARGET_LINUX_ARM := aarch64-unknown-linux-gnu
TARGET_RPI := armv7-unknown-linux-gnueabihf

.PHONY: all build release test clean install-deps

# Alvo padrão
all: build

# Verificar versão do Rust
check-rust:
	@rustc --version | grep -E "1.(90|9[1-9]|[1-9][0-9][0-9])" || \
		(echo "Rust 1.90+ necessário" && exit 1)

# Medir tempo de build
build-timed: check-rust
	@echo "Iniciando build cronometrado..."
	@time $(CARGO) build --workspace

# Verificar versão do Rust
version:
	@rustc --version
	@cargo --version

# Instalar dependências do sistema (Debian/Ubuntu)
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
	# Instalar ferramentas Rust
	cargo install sqlx-cli
	cargo install cargo-watch
	cargo install cargo-audit
	cargo install cargo-tarpaulin
	cargo install cross

# Configurar permissões de hardware
setup-hardware:
	# PCSC para leitores de cartão
	sudo systemctl enable pcscd
	sudo systemctl start pcscd
	# Permissões USB
	sudo cp scripts/99-turnkey.rules /etc/udev/rules.d/
	sudo udevadm control --reload-rules
	sudo usermod -aG dialout,plugdev $$USER

# Comandos de build
build: check-rust
	$(CARGO) build --workspace

release:
	$(CARGO) build --release --workspace

# Compilação cruzada
build-arm:
	$(CROSS) build --release --target $(TARGET_LINUX_ARM)

build-rpi:
	$(CROSS) build --release --target $(TARGET_RPI) \
		--features raspberry-pi

# Comandos de teste
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

# Teste PCSC
pcsc-test:
	pcsc_scan

# Benchmarks
bench:
	$(CARGO) bench --workspace

# Banco de dados
db-create:
	sqlx database create

db-migrate:
	sqlx migrate run --source migrations

db-reset: db-drop db-create db-migrate

db-drop:
	sqlx database drop -y

# Documentação
docs:
	$(CARGO) doc --workspace --no-deps --open

# Linting e formatação
lint:
	$(CARGO) clippy --workspace -- -D warnings

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

# Auditoria de segurança
audit:
	$(CARGO) audit

# Cobertura
coverage:
	$(CARGO) tarpaulin --workspace --out Html

# Limpar
clean:
	$(CARGO) clean
	rm -rf logs/*.log

# Executar servidor de desenvolvimento
run:
	RUST_LOG=debug $(CARGO) run --bin turnkey-cli -- server

# Executar com hardware
run-hw:
	sudo RUST_LOG=debug $(CARGO) run --bin turnkey-cli \
		--features "hardware" -- server

# Modo watch para desenvolvimento
watch:
	$(CARGO) watch -x 'run --bin turnkey-cli'

# Instalar
install: release
	sudo cp target/release/turnkey-cli /usr/local/bin/turnkey

# Desinstalar
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

# Ajuda
help:
	@echo "Emulador de Controle de Acesso Turnkey - Sistema de Build"
	@echo ""
	@echo "Configuração:"
	@echo "  install-deps     Instalar dependências do sistema"
	@echo "  setup-hardware   Configurar permissões de hardware"
	@echo ""
	@echo "Build:"
	@echo "  build           Build versão debug"
	@echo "  release         Build versão release"
	@echo "  build-arm       Compilação cruzada para ARM64"
	@echo "  build-rpi       Compilação cruzada para Raspberry Pi"
	@echo ""
	@echo "Teste:"
	@echo "  test            Executar todos os testes"
	@echo "  test-hardware   Executar testes de hardware (requer sudo)"
	@echo "  bench           Executar benchmarks"
	@echo ""
	@echo "Banco de Dados:"
	@echo "  db-create       Criar banco de dados"
	@echo "  db-migrate      Executar migrações"
	@echo "  db-reset        Reiniciar banco de dados"
	@echo ""
	@echo "Desenvolvimento:"
	@echo "  run             Executar servidor de desenvolvimento"
	@echo "  run-hw          Executar com suporte a hardware"
	@echo "  watch           Modo watch"
	@echo "  docs            Gerar documentação"
	@echo ""
	@echo "Qualidade:"
	@echo "  lint            Executar clippy"
	@echo "  fmt             Formatar código"
	@echo "  audit           Auditoria de segurança"
	@echo "  coverage        Gerar relatório de cobertura"
```

## 12. rust-toolchain.toml

```toml
[toolchain]
channel = "1.90.0"
components = ["rustfmt", "clippy", "rust-analyzer"]
profile = "default"
```

## 13. Scripts de Configuração (scripts/install-deps.sh)

```bash
#!/bin/bash
set -e

echo "=== Emulador de Controle de Acesso Turnkey - Instalação de Dependências ==="

# Detectar SO
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    DISTRO=$(lsb_release -si)
    VERSION=$(lsb_release -sr)
    KERNEL=$(uname -r)

    echo "Detectado: $DISTRO $VERSION (Kernel $KERNEL)"

    # Verificar versão do kernel (6.1+)
    KERNEL_MAJOR=$(echo $KERNEL | cut -d. -f1)
    KERNEL_MINOR=$(echo $KERNEL | cut -d. -f2)

    if [ "$KERNEL_MAJOR" -lt 6 ] || ([ "$KERNEL_MAJOR" -eq 6 ] && [ "$KERNEL_MINOR" -lt 1 ]); then
        echo "Aviso: Kernel 6.1+ recomendado, você tem $KERNEL"
    fi

    # Instalar baseado na distribuição
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
            echo "Distribuição não suportada: $DISTRO"
            exit 1
            ;;
    esac
else
    echo "Este script é apenas para Linux"
    exit 1
fi

echo "=== Instalando ferramentas Rust ==="
cargo install sqlx-cli --no-default-features --features sqlite
cargo install cargo-watch
cargo install cargo-audit
cargo install cross

echo "=== Configuração completa! ==="
```

## 14. Justificativa Técnica

### **Bibliotecas Selecionadas:**

1. **Tokio** - Runtime assíncrono mais maduro e performático do ecossistema Rust
2. **SQLx** - SQL type-safe com verificação em tempo de compilação, suporte assíncrono nativo
3. **Tracing** - Logging estruturado moderno, superior ao crate `log`
4. **Serde** - Padrão de facto para serialização em Rust
5. **PCSC** - Interface padrão para leitores de cartão inteligente no Linux
6. **HidAPI** - Acesso multiplataforma a dispositivos USB HID
7. **DashMap** - HashMap concorrente sem bloqueio para alto desempenho

### **Arquitetura:**

- **Workspace Cargo**: Modularidade máxima, compilação otimizada
- **Separação de Responsabilidades**: Cada crate tem responsabilidade única
- **Design Baseado em Traits**: Extensibilidade via traits, facilita mocking
- **Async/Await**: Desempenho e escalabilidade
- **Type Safety**: Uso extensivo do sistema de tipos do Rust

### **Armazenamento:**

- **SQLite**: Embarcado, configuração zero, perfeito para dispositivos edge
- **Migrações**: Versionamento controlado de esquema
- **Padrão Repository**: Abstração da camada de dados

### **Compatibilidade:**

- **Kernel Linux 6.1+**: Suporte moderno a USB, GPIO, hidraw
- **Compilação Cruzada**: Suporta x64, ARM64, ARMv7 (Raspberry Pi)
- **Rust 1.90+**: Recursos modernos, traits assíncronos estáveis
- **Edição Rust**: 2024
```
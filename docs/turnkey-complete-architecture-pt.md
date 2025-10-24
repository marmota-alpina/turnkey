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

## 5. Interface de Usuário Terminal (TUI)

### Visão Geral

O emulador Turnkey fornece uma Interface de Usuário Terminal realística construída com `ratatui` que imita a aparência e comportamento de catracas físicas brasileiras (Primme SF, Henry Lumen).

### Componentes Principais

#### Display LCD (2 linhas × 40 colunas)
- **Visual**: Fundo azul, texto branco, fonte em negrito
- **Propósito**: Mostra mensagens do sistema, entrada do usuário e status
- **Estados**: IDLE, VALIDATING, GRANTED, DENIED, WAITING_ROTATION
- **Temas**: Padrão (azul), Escuro (preto/verde), Claro (branco), Verde (estilo LCD)

#### Teclado Numérico
- **Layout**: 4 linhas × 3 colunas (0-9, *, #)
- **Botões**: ENTER, CANCEL, CLEAR
- **Feedback Visual**: Destaque de teclas ao pressionar (cinza → amarelo)
- **Entrada**: Entrada de código em buffer com validação de comprimento máximo

#### Painel de Logs
- **Tipo**: Log de eventos rolável, auto-atualizado
- **Formato**: `[HH:MM:SS] Mensagem`
- **Capacidade**: 1000 entradas (configurável)
- **Cores**: Info (branco), Sucesso (verde), Aviso (amarelo), Erro (vermelho)
- **Recursos**: Busca, navegação por rolagem, filtragem por timestamp

#### Barra de Status
- **Status das Leitoras**: RFID✓ BIO✗ KEYPAD✓ WIEGAND✗
- **Estatísticas**: Contagem de eventos, contagem de usuários, uso de armazenamento
- **Rede**: Status de conexão, endereço IP, modo (ONLINE/OFFLINE)

### Atalhos de Teclado

| Tecla           | Ação                          |
|-----------------|-------------------------------|
| `0`-`9`, `*`, `#` | Entrada do teclado              |
| `Enter`         | Confirmar / Enviar            |
| `Escape`        | Cancelar entrada              |
| `Backspace`     | Apagar último dígito          |
| `Tab`           | Alternar foco (Display ↔ Logs) |
| `F1`            | Mostrar ajuda                 |
| `F5`            | Recarregar configuração       |
| `F8`            | Exportar eventos              |
| `F10`           | Menu de configurações         |
| `q` ou `Ctrl+C` | Sair do emulador              |

### Design Responsivo

- **Mínimo**: 80 colunas × 24 linhas
- **Ótimo**: 120 colunas × 30 linhas
- **Layout**: 70% coluna emulador, 30% coluna logs
- **Adaptação**: Ajusta proporções automaticamente com base no tamanho do terminal

**Veja**: [Especificação TUI](tui-specification.md) para detalhes completos de design.

---

## 6. Modos de Operação

O emulador suporta dois modos primários de operação que determinam como a validação de acesso é realizada.

### Modo ONLINE (Testes de Produção)

**Propósito**: Emular uma catraca física que envia solicitações de acesso para um cliente TCP externo para validação.

**Características Principais**:
- Emulador não tem lógica de validação (imita hardware real)
- Cliente TCP toma todas as decisões de acesso
- Relatórios de eventos e status em tempo real
- Tratamento configurável de timeout e fallback

**Configuração**:
```toml
[mode]
online = true
status_online = true       # Enviar heartbeat periódico
event_online = true        # Enviar eventos em tempo real
fallback_offline = true    # Mudar para offline em timeout

[network]
type = "tcp"
tcp_mode = "server"        # Esperar conexões de clientes
ip = "192.168.0.100"
port = 3000
```

**Fluxo de Mensagens**:
1. **Solicitação de Acesso** (Emulador → Cliente):
   ```
   01+REON+000+0]12345678]20/10/2025 14:30:00]1]0]
   ```
   - Número do cartão: `12345678`
   - Timestamp: `20/10/2025 14:30:00`
   - Direção: `1` (entrada), `2` (saída)

2. **Resposta de Liberação** (Cliente → Emulador):
   ```
   01+REON+00+6]5]Acesso liberado]
   ```
   - Comando: `00+6` (liberar saída)
   - Tempo de exibição: `5` segundos
   - Mensagem: `Acesso liberado`

3. **Eventos de Rotação**:
   - **Aguardando**: `01+REON+000+80]...]`
   - **Completo**: `01+REON+000+81]...]`
   - **Timeout**: `01+REON+000+82]...]`

**Tratamento de Timeout**:
- Timeout padrão: 3000ms
- Se `fallback_offline = true`: Mudar para modo OFFLINE
- Se `fallback_offline = false`: Negar acesso, retornar ao IDLE

### Modo OFFLINE (Operação Autônoma)

**Propósito**: Validação local usando banco de dados SQLite sem dependências de rede.

**Características Principais**:
- Toda lógica de validação no emulador
- Sem requisitos de rede
- Banco de dados local de usuários/cartões
- Adequado para testes e desenvolvimento

**Configuração**:
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

**Fluxo de Validação**:
1. **Busca de Usuário**: Consultar por código, número de cartão ou biometria
2. **Verificação de Validade**: Status ativo, período de validade
3. **Verificação de Método de Acesso**: Verificar métodos permitidos (cartão, bio, teclado)
4. **Liberar/Negar**: Exibir mensagem, registrar evento, simular rotação

**Tabelas do Banco de Dados**:
- `users` - Credenciais de usuário e permissões de acesso
- `cards` - Associações de cartões RFID
- `biometric_templates` - Dados de impressão digital
- `access_logs` - Histórico completo de acesso

### Modo Híbrido (Fallback)

**Configuração**:
```toml
[mode]
online = true
fallback_offline = true
fallback_timeout = 3000
```

**Comportamento**:
1. Iniciar em modo ONLINE
2. Em timeout de validação: Mudar para OFFLINE
3. Consultar banco de dados local, liberar/negar localmente
4. Exibir "MODO OFFLINE"
5. Quando conexão restaurada: Sincronizar eventos, retornar ao ONLINE

**Veja**: [Modos do Emulador](emulator-modes.md) para documentação detalhada de fluxo.

---

## 7. Sistema de Configuração

### Arquivos de Configuração

O emulador usa configuração baseada em TOML com um sistema de prioridade:

**Locais dos Arquivos**:
```
config/
├── default.toml         # Configuração padrão (commitado no git)
├── local.toml           # Sobrescritas locais (gitignored)
├── hardware.toml        # Configurações específicas de hardware
└── logging.toml         # Configuração de logging
```

**Prioridade de Carregamento** (maior para menor):
1. **Variáveis de Ambiente**: `TURNKEY_SECTION_KEY=value`
2. **config/local.toml**: Sobrescritas de desenvolvimento
3. **config/default.toml**: Configuração base

### Seções Principais de Configuração

#### [device] - Identidade do Dispositivo
```toml
id = 1                          # ID do dispositivo (01-99)
model = "Turnkey Emulator"
firmware_version = "1.0.0"
protocol_version = "1.0.0.15"
display_message = "DIGITE SEU CÓDIGO"  # Máx 40 caracteres
```

#### [mode] - Modo de Operação
```toml
online = true                   # true = ONLINE, false = OFFLINE
status_online = true            # Enviar heartbeat
event_online = true             # Enviar eventos em tempo real
fallback_offline = true         # Fallback em timeout
fallback_timeout = 3000         # Milissegundos
```

#### [network] - Configurações de Rede
```toml
type = "tcp"                    # tcp ou serial
ip = "192.168.0.100"
port = 3000
tcp_mode = "server"             # server ou client
dhcp = false
```

#### [readers] - Configuração das Leitoras
```toml
reader1 = "rfid"                # rfid, keypad, biometric, wiegand, disabled
reader2 = "keypad"
reader3 = "disabled"
reader4 = "disabled"
keypad_enabled = true
keypad_timeout = 30             # segundos
```

#### [biometrics] - Configurações Biométricas
```toml
verify_card_with_bio = false    # Requerer impressão digital após cartão
treat_1n = true                 # Modo de identificação 1:N
auto_on = false                 # Auto-identificar por impressão digital
sensitivity = 55                # 48-55 (maior = mais sensível)
security_level = 80             # 48-82 (maior = mais rigoroso)
```

#### [storage] - Configurações de Banco de Dados
```toml
database_path = "data/turnkey.db"
max_events = 100000
max_users = 10000
wal_enabled = true
synchronous = "NORMAL"          # OFF, NORMAL, FULL
backup_interval = 60            # minutos
```

#### [ui] - Configurações da Interface Terminal
```toml
enabled = true
display_lines = 2
display_cols = 40
theme = "default"               # default, dark, light, green
log_panel_height = 30           # porcentagem
```

### Recarga de Configuração em Tempo de Execução

**Métodos**:
- **Sinal**: `kill -HUP <pid>` (Linux)
- **TUI**: Pressione `F5`
- **API**: `POST /api/reload-config` (futuro)

**Configurações Recarregáveis**: Mensagens de exibição, habilitar/desabilitar leitoras, níveis de log, tema da UI, timeouts

**Requer Reinicialização**: IP/porta de rede, caminho do banco de dados, ID do dispositivo, modo de operação

**Veja**: [Configuração do Emulador](emulator-configuration.md) para referência completa.

---

## 8. Importação/Exportação de Dados

O emulador suporta múltiplos formatos de arquivo para operações em massa, backups e interoperabilidade com sistemas externos.

### Formatos de Arquivo Suportados

#### 1. Importação/Exportação de Usuários (`colaborador.txt`)
**Formato**: Valores separados por pipe
**Campos**: PIS, NOME, MATRICULA, CPF, VALIDADE_INICIO, VALIDADE_FIM, ATIVO, ALLOW_CARD, ALLOW_BIO, ALLOW_KEYPAD, CODIGO

**Exemplo**:
```
12345678901|João da Silva|1001|12345678901|01/01/2025|31/12/2025|1|1|1|1|1234
98765432101|Maria Santos|1002|98765432101|01/01/2025||1|1|0|1|5678
```

#### 2. Importação/Exportação de Cartões (`cartoes.txt`)
**Formato**: Valores separados por pipe
**Campos**: NUMERO_CARTAO, MATRICULA, VALIDADE_INICIO, VALIDADE_FIM, ATIVO

**Exemplo**:
```
00000000000011912322|1001|01/01/2025|31/12/2025|1
00000000000022823433|1002|01/01/2025||1
```

#### 3. Templates Biométricos (`biometria.txt`)
**Formato**: Valores separados por pipe
**Campos**: MATRICULA, POSICAO (índice de dedo 0-9), TEMPLATE_BASE64

**Exemplo**:
```
1001|1|AQIDBAUG...BASE64...ENCODED==
1001|6|BQYHCQ0K...BASE64...ENCODED==
```

#### 4. Exportação de Logs de Acesso (`eventos.txt`)
**Formato**: Valores separados por pipe
**Campos**: NSR, DATA_HORA, MATRICULA, CARTAO, TIPO_EVENTO, DIRECAO, DISPOSITIVO, VALIDACAO, NOME

**Exemplo**:
```
1|20/10/2025 08:15:23|1001|00000000000011912322|0|1|01|O|João da Silva
2|20/10/2025 09:30:12|1002|00000000000022823433|0|1|01|F|Maria Santos
```

#### 5. Formato AFD (Padrão Legal Brasileiro)
**Propósito**: Conformidade com controle de ponto e controle de acesso (Portaria 1510/2009)
**Estrutura**:
- Registro Tipo 1: Cabeçalho (CNPJ, nome da empresa, período)
- Registro Tipo 2: Identificação do dispositivo
- Registro Tipo 3: Eventos de acesso
- Registro Tipo 9: Trailer (contagem de registros)

**Exemplo**:
```
1|12345678000190||EMPRESA EXEMPLO LTDA|01102025|20102025|20102025143000
2|1|TK00000001|Turnkey Emulator|1.0.0
3|000001|20102025|081523|12345678901
9|3
```

### Comandos de Importação/Exportação em Massa

**Importar Todos os Dados**:
```bash
turnkey-cli import bulk ./import-data/
```

**Comandos de Exportação**:
```bash
# Exportar usuários
turnkey-cli export colaborador --output colaborador-backup.txt

# Exportar cartões
turnkey-cli export cartoes --output cartoes-backup.txt

# Exportar eventos com intervalo de datas
turnkey-cli export eventos \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output eventos-outubro.txt

# Exportar formato AFD
turnkey-cli export afd \
  --cnpj "12345678000190" \
  --razao-social "Empresa Exemplo" \
  --start-date "01/10/2025" \
  --end-date "31/10/2025" \
  --output afd-outubro.txt
```

### Comportamento Transacional

- Todas as importações envolvidas em transação de banco de dados
- Rollback em qualquer falha de validação
- Registros duplicados ignorados com aviso
- Progresso mostrado para arquivos grandes (>1000 registros)

**Veja**: [Formatos de Dados](data-formats.md) para especificações completas de formato.

---

## 14. Justificativa Técnica

### **Bibliotecas Selecionadas:**

1. **Tokio** - Runtime assíncrono mais maduro e performático do ecossistema Rust
2. **SQLx** - SQL type-safe com verificação em tempo de compilação, suporte assíncrono nativo
3. **Tracing** - Logging estruturado moderno, superior ao crate `log`
4. **Serde** - Padrão de facto para serialização em Rust
5. **PCSC** - Interface padrão para leitores de cartão inteligente no Linux
6. **HidAPI** - Acesso multiplataforma a dispositivos USB HID
7. **DashMap** - HashMap concorrente sem bloqueio para alto desempenho
8. **Ratatui** - Framework TUI moderno, ativamente mantido com excelente documentação

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

---

## 15. Referências

### Documentação Principal
- [README.md](../README.md) - Visão geral do projeto e início rápido

### Arquitetura e Design
- [Arquitetura do Emulador](emulator-architecture.md) - Arquitetura do sistema e design de componentes
- [Modos do Emulador](emulator-modes.md) - Modos de operação ONLINE vs OFFLINE
- [Especificação TUI](tui-specification.md) - Design da Interface de Usuário Terminal

### Configuração e Dados
- [Configuração do Emulador](emulator-configuration.md) - Referência completa de configuração TOML
- [Formatos de Dados](data-formats.md) - Especificações de formato de arquivo de importação/exportação

### Protocolo
- [Guia do Protocolo Henry](turnkey-protocol-guide-en.md) - Especificação do protocolo Henry
- [Comandos do Cliente Henry](henry-client-emulator-commands.md) - Comandos descobertos do cliente oficial

### Hardware
- [Guia de Configuração de Hardware](hardware-setup.md) - Integração de hardware físico
- [Manual Primme SF](Catraca Sf item 01.pdf) - Documentação do equipamento original
```
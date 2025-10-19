.PHONY: build test run clean help

# Rust 1.90 - Builds 20-40% mais rápidos com LLD linker!

build:
	@echo "🦀 Building with Rust 1.90 (LLD linker para performance máxima)..."
	cargo build --release

test:
	@echo "🧪 Running tests..."
	cargo test --all

test-verbose:
	@echo "🧪 Running tests com output detalhado..."
	cargo test --all -- --nocapture

run:
	cargo run --bin turnkey-cli

clean:
	cargo clean

pcsc-test:
	pcsc_scan

# Verificar versão do Rust
version:
	@echo "Rust version:"
	@rustc --version
	@echo "\nCargo version:"
	@cargo --version

# Build com timing para medir performance
build-timed:
	@echo "⏱️  Medindo tempo de build..."
	@/usr/bin/time -f "Tempo total: %E" cargo build --release

# Check code quality
check:
	cargo check --workspace
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	cargo fmt --all -- --check

# Format code
fmt:
	cargo fmt --all

# Help
help:
	@echo "Turnkey Access Control Emulator - Makefile"
	@echo "Rust 1.90 com LLD linker - 20-40% builds mais rápidos! 🚀"
	@echo ""
	@echo "Comandos disponíveis:"
	@echo "  make build        - Build release otimizado"
	@echo "  make build-timed  - Build com medição de tempo"
	@echo "  make test         - Executar todos os testes"
	@echo "  make test-verbose - Testes com output completo"
	@echo "  make run          - Executar CLI"
	@echo "  make check        - Verificar qualidade do código"
	@echo "  make fmt          - Formatar código"
	@echo "  make clean        - Limpar build artifacts"
	@echo "  make version      - Mostrar versão do Rust/Cargo"
	@echo "  make pcsc-test    - Testar PCSC daemon"

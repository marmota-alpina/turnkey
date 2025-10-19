.PHONY: build test run clean help

build:
	@echo "Building with Rust 1.90 (LLD linker for maximum performance)..."
	cargo build --release

test:
	@echo "Running tests..."
	cargo test --all

test-verbose:
	@echo "Running tests with detailed output..."
	cargo test --all -- --nocapture

run:
	cargo run --bin turnkey-cli

clean:
	cargo clean

pcsc-test:
	pcsc_scan

# Check Rust version
version:
	@echo "Rust version:"
	@rustc --version
	@echo "\nCargo version:"
	@cargo --version

# Build with timing to measure performance
build-timed:
	@echo "Measuring build time..."
	@/usr/bin/time -f "Total time: %E" cargo build --release

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
	@echo "Rust 1.90 with LLD linker - 20-40% faster builds"
	@echo ""
	@echo "Available commands:"
	@echo "  make build        - Build optimized release"
	@echo "  make build-timed  - Build with time measurement"
	@echo "  make test         - Run all tests"
	@echo "  make test-verbose - Run tests with full output"
	@echo "  make run          - Run CLI application"
	@echo "  make check        - Check code quality"
	@echo "  make fmt          - Format code"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make version      - Show Rust/Cargo version"
	@echo "  make pcsc-test    - Test PCSC daemon"

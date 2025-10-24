# Contributing to Turnkey

Thank you for your interest in contributing to Turnkey! This document provides guidelines and workflows for contributing.

## Table of Contents
- [Development Workflow](#development-workflow)
- [Branch Naming](#branch-naming)
- [Commit Messages](#commit-messages)
- [Pull Requests](#pull-requests)
- [Code Standards](#code-standards)
- [Testing](#testing)

## Development Workflow

We follow **GitHub Flow** for development:

1. **Pick an issue** from the [issue tracker](https://github.com/marmota-alpina/turnkey/issues)
2. **Create a branch** from `main`
3. **Make your changes** with clear commits
4. **Push** your branch
5. **Open a Pull Request**
6. **Address review feedback**
7. **Merge** after CI passes and approval

### Quick Start

```bash
# Fork and clone the repository
git clone https://github.com/YOUR_USERNAME/turnkey.git
cd turnkey

# Create a feature branch
git checkout -b feature/1-message-structures

# Make your changes
# ... code, test, commit ...

# Push and create PR
git push origin feature/1-message-structures
```

## Branch Naming

Use descriptive branch names with the following prefixes:

- `feature/N-description` - New features (e.g., `feature/1-message-structures`)
- `fix/N-description` - Bug fixes (e.g., `fix/42-parser-crash`)
- `docs/N-description` - Documentation only
- `refactor/N-description` - Code refactoring
- `test/N-description` - Adding tests
- `perf/N-description` - Performance improvements

Where `N` is the issue number.

## Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>: <description>

[optional body]

[optional footer]
```

**Types:**
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `test:` - Adding or updating tests
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `chore:` - Maintenance tasks

**Examples:**
```bash
feat: implement Message and Frame structures

Add core protocol structures for Henry protocol message
representation, including high-level Message and low-level
Frame types with conversion traits.

Closes #1

---

fix: handle malformed checksum in parser

The parser was panicking on invalid checksums. Now it
returns a proper ProtocolError::ChecksumError.

Closes #15

---

docs: add rustdoc comments to AccessRequest

Add comprehensive documentation with examples for all
AccessRequest struct fields and methods.
```

## Pull Requests

### Before Creating a PR

- [ ] Run `cargo fmt --all` to format code
- [ ] Run `cargo clippy --workspace -- -D warnings` (no warnings)
- [ ] Run `cargo test --workspace` (all tests pass)
- [ ] Update documentation if needed
- [ ] Add tests for new functionality

### PR Description

Use the PR template to provide:
- Clear description of changes
- Link to related issue(s)
- Type of change
- Testing performed
- Checklist completion

### Review Process

1. **CI must pass** - All checks must be green
2. **Code review** - At least one approval required
3. **Address feedback** - Respond to all review comments
4. **Keep updated** - Merge `main` if conflicts arise

## Code Standards

### Rust Style

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` with default settings
- Address all `clippy` warnings
- Write idiomatic Rust code

### Documentation

- Add rustdoc comments for all public items
- Include examples in documentation
- Document error conditions

### Error Handling

- Use `Result<T, E>` for fallible operations
- Create specific error types (avoid `anyhow` in libraries)
- Provide context in error messages
- Use `thiserror` for error definitions

### Testing

- Write unit tests for all modules
- Add integration tests for complete flows
- Test error paths, not just happy paths
- Use property-based testing where appropriate

## Testing

### Run All Tests

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --workspace --test '*'

# With coverage
cargo tarpaulin --workspace --all-features
```

### Test Organization

- Unit tests: `#[cfg(test)]` modules in source files
- Integration tests: `tests/integration/` directory
- Test helpers: `tests/common/` directory
- Benchmarks: `benches/` directory

### Hardware Tests

Hardware tests require physical devices:

```bash
# Requires sudo and connected hardware
sudo cargo test --features hardware --test hardware_integration
```

## Project Structure

```
turnkey/
├── turnkey-core/        # Shared types and errors
├── turnkey-protocol/    # Henry protocol implementation
├── turnkey-hardware/    # Hardware abstraction layer
├── turnkey-rfid/        # RFID reader drivers
├── turnkey-biometric/   # Biometric scanner drivers
├── turnkey-keypad/      # Keypad drivers
├── turnkey-turnstile/   # Turnstile controller drivers
├── turnkey-storage/     # Database layer
├── turnkey-network/     # TCP/IP server
├── turnkey-emulator/    # Device emulators
└── turnkey-cli/         # CLI application
```

## Building

```bash
# Debug build
cargo build --workspace

# Release build (optimized)
cargo build --workspace --release

# Check without building
cargo check --workspace
```

## Questions?

- Open a [Discussion](https://github.com/marmota-alpina/turnkey/discussions)
- Ask in pull request comments
- Check existing [issues](https://github.com/marmota-alpina/turnkey/issues)

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.

# GitHub Workflow Guide

This document outlines the recommended GitHub workflow for the Turnkey project.

## Workflow Model: GitHub Flow

We use **GitHub Flow**, a lightweight branch-based workflow:

```
main (always deployable)
  ├─ feature/1-message-structures
  ├─ feature/2-command-codes
  ├─ fix/15-parser-error
  └─ docs/18-api-documentation
```

## Why GitHub Flow?

- ✅ **Simple**: Only one main branch
- ✅ **Fast**: Quick iteration cycles
- ✅ **Safe**: CI checks on every PR
- ✅ **Clear**: Linear history with descriptive PRs

## Step-by-Step Workflow

### 1. Select an Issue

Browse [open issues](https://github.com/marmota-alpina/turnkey/issues) and pick one:
- Check if it's assigned
- Comment that you're working on it
- Understand acceptance criteria

### 2. Create a Branch

```bash
# Update main
git checkout main
git pull origin main

# Create feature branch
git checkout -b feature/1-message-structures
```

**Branch naming convention:**
```
feature/N-short-description    # New features
fix/N-short-description        # Bug fixes
docs/N-short-description       # Documentation
refactor/N-short-description   # Refactoring
test/N-short-description       # Tests only
perf/N-short-description       # Performance
```

### 3. Develop

**Write code:**
```bash
# Make changes
vim turnkey-protocol/src/message.rs

# Test frequently
cargo test --package turnkey-protocol

# Check style
cargo fmt
cargo clippy
```

**Commit often:**
```bash
git add turnkey-protocol/src/message.rs
git commit -m "feat: add Message struct with builder pattern"
```

### 4. Keep Branch Updated

```bash
# If main has new changes
git checkout main
git pull origin main
git checkout feature/1-message-structures
git merge main

# Or use rebase for cleaner history
git rebase main
```

### 5. Push and Create PR

```bash
# Push branch
git push origin feature/1-message-structures

# Go to GitHub and create Pull Request
# Use the PR template provided
```

**PR Title Examples:**
- `feat: implement Message and Frame structures (#1)`
- `fix: handle malformed checksums in parser (#15)`
- `docs: add rustdoc comments to protocol module (#18)`

### 6. Code Review

- **Wait for CI**: All checks must pass
- **Address feedback**: Push new commits or amend
- **Update if needed**: Merge main if conflicts arise
- **Get approval**: At least one reviewer approves

### 7. Merge

Once approved and CI passes:
- **Squash and merge** (recommended for features)
- **Rebase and merge** (for clean history)
- **Regular merge** (preserves all commits)

The related issue closes automatically via `Closes #N` in PR description.

## Branch Protection Rules

**Recommended settings for `main` branch:**

```yaml
Protection Rules:
  ✓ Require pull request before merging
  ✓ Require approvals: 1
  ✓ Dismiss stale reviews
  ✓ Require status checks to pass
    - fmt (Rustfmt)
    - clippy (Clippy)
    - test (Test Suite)
    - build (Build)
  ✓ Require branches to be up to date
  ✓ Require linear history (optional)
  ✓ Include administrators
```

## CI/CD Pipeline

### On Every Push/PR to Main

```yaml
Jobs:
  1. Format Check (cargo fmt --check)
  2. Lint (cargo clippy)
  3. Test (cargo test --workspace)
  4. Build (cargo build --release)
  5. Documentation (cargo doc)
  6. Coverage (cargo tarpaulin)
```

### On Tagged Release (v*.*.*)

```yaml
Jobs:
  1. Create GitHub Release
  2. Build binaries (Linux x86_64, ARM64, ARMv7)
  3. Upload release assets
  4. Publish to crates.io
```

## Release Process

### Semantic Versioning

We follow [SemVer](https://semver.org/):
- `v1.0.0` - Major release (breaking changes)
- `v1.1.0` - Minor release (new features)
- `v1.1.1` - Patch release (bug fixes)

### Creating a Release

```bash
# Update version in all Cargo.toml files
vim turnkey-*/Cargo.toml

# Commit version bump
git add .
git commit -m "chore: bump version to 1.0.0"

# Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin main
git push origin v1.0.0

# GitHub Actions will automatically:
# - Create GitHub release
# - Build binaries
# - Publish to crates.io
```

## Project Milestones

Use GitHub Milestones to track progress:

**Current Milestones:**
- **v0.1.0 - Core Protocol** (Issues #1-9)
  - Foundation structures
  - Access control commands
  - Basic parsing and building

- **v0.2.0 - Device Management** (Issues #10-14)
  - Configuration commands
  - Card/user management
  - Status queries

- **v0.3.0 - Production Ready** (Issues #15-20)
  - Biometric support
  - Performance optimization
  - Complete documentation
  - Compatibility validation

## Labels Strategy

**Type Labels:**
- `protocol` - Protocol implementation
- `hardware` - Hardware integration
- `documentation` - Documentation
- `testing` - Testing
- `performance` - Performance

**Status Labels:**
- `good first issue` - Good for newcomers
- `help wanted` - Need community help
- `blocked` - Waiting for something
- `wip` - Work in progress

**Priority Labels:**
- `priority: high` - Urgent
- `priority: medium` - Normal
- `priority: low` - Nice to have

**Phase Labels:**
- `foundation` - Core structures
- `access-control` - Access control
- `device-management` - Device management
- `advanced` - Advanced features

**Week Labels:**
- `week-1`, `week-2`, ... `week-8` - Timeline

## Best Practices

### DO ✅

- Keep PRs focused and small
- Write descriptive commit messages
- Add tests for new code
- Update documentation
- Run CI locally before pushing
- Respond to review feedback promptly
- Link PRs to issues

### DON'T ❌

- Push directly to `main`
- Create huge PRs (>500 lines)
- Ignore CI failures
- Skip tests
- Leave commented code
- Commit secrets or credentials
- Force push to shared branches

## Example Workflow

```bash
# 1. Pick issue #1
# 2. Create branch
git checkout -b feature/1-message-structures

# 3. Develop
# ... coding ...

# 4. Test
cargo test --package turnkey-protocol
cargo clippy --package turnkey-protocol
cargo fmt

# 5. Commit
git add .
git commit -m "feat: implement Message and Frame structures

Add core protocol structures with builder pattern,
conversion traits, and comprehensive unit tests.

Closes #1"

# 6. Push
git push origin feature/1-message-structures

# 7. Create PR on GitHub
# 8. Address review feedback
# 9. Merge after approval

# 10. Clean up
git checkout main
git pull origin main
git branch -d feature/1-message-structures
```

## Resources

- [GitHub Flow Guide](https://guides.github.com/introduction/flow/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Contributing Guide](.github/CONTRIBUTING.md)

## Questions?

Open a [Discussion](https://github.com/marmota-alpina/turnkey/discussions) or ask in your PR!

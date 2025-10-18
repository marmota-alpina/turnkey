# GitHub Workflow Summary

Quick reference for the recommended GitHub workflow for Turnkey project.

## 🔄 Workflow Model: GitHub Flow

**Simple, fast, and safe:**
```
main (always stable)
  └─ feature/N-description → PR → Review → Merge → Close Issue
```

## 📋 Quick Start Checklist

### Starting Work on an Issue

- [ ] Pick an issue from [issues page](https://github.com/marmota-alpina/turnkey/issues)
- [ ] Create branch: `git checkout -b feature/N-description`
- [ ] Make changes with clear commits
- [ ] Run checks: `cargo fmt`, `cargo clippy`, `cargo test`
- [ ] Push: `git push origin feature/N-description`
- [ ] Create Pull Request with template
- [ ] Wait for CI and review
- [ ] Address feedback
- [ ] Merge when approved

## 🏷️ Branch Naming

| Type | Format | Example |
|------|--------|---------|
| Feature | `feature/N-description` | `feature/1-message-structures` |
| Bug Fix | `fix/N-description` | `fix/15-parser-crash` |
| Docs | `docs/N-description` | `docs/18-api-docs` |
| Refactor | `refactor/N-description` | `refactor/3-parser-cleanup` |
| Test | `test/N-description` | `test/9-integration-tests` |
| Performance | `perf/N-description` | `perf/17-optimize-parser` |

## 💬 Commit Messages

Follow Conventional Commits:

```
<type>: <description>

[optional body]

Closes #N
```

**Types:** `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

**Examples:**
```bash
feat: implement Message struct with builder pattern

fix: handle malformed checksums in parser

docs: add rustdoc comments to AccessRequest
```

## 🤖 CI/CD Checks

Every PR runs these checks automatically:

| Check | Command | Required |
|-------|---------|----------|
| **Format** | `cargo fmt --check` | ✅ Yes |
| **Lint** | `cargo clippy -- -D warnings` | ✅ Yes |
| **Test** | `cargo test --workspace` | ✅ Yes |
| **Build** | `cargo build --release` | ✅ Yes |
| **Docs** | `cargo doc --no-deps` | ✅ Yes |
| **Coverage** | `cargo tarpaulin` | ℹ️ Info only |

## 📝 Files Created

### Workflows
- `.github/workflows/ci.yml` - Main CI pipeline
- `.github/workflows/release.yml` - Release automation

### Templates
- `.github/PULL_REQUEST_TEMPLATE.md` - PR template
- `.github/ISSUE_TEMPLATE/bug_report.md` - Bug report template
- `.github/ISSUE_TEMPLATE/feature_request.md` - Feature request template
- `.github/ISSUE_TEMPLATE/config.yml` - Issue template config

### Documentation
- `.github/CONTRIBUTING.md` - Full contribution guide
- `.github/WORKFLOW.md` - Detailed workflow documentation
- `.github/dependabot.yml` - Dependency update automation

## 🎯 Project Phases

| Phase | Issues | Timeline | Focus |
|-------|--------|----------|-------|
| **Phase 1: Foundation** | #1-5 | Week 1-2 | Core structures, parser, codec |
| **Phase 2: Access Control** | #6-9 | Week 3-4 | Access requests, responses, states |
| **Phase 3: Device Mgmt** | #10-14 | Week 5-6 | Config, cards, users, records |
| **Phase 4: Advanced** | #15-20 | Week 7-8 | Biometric, perf, docs, validation |

## 🔒 Branch Protection

Recommended settings for `main`:

- ✅ Require PR before merging
- ✅ Require 1 approval
- ✅ Require status checks to pass
- ✅ Require branches to be up to date
- ✅ Include administrators

## 🏷️ Labels in Use

**Type:**
- `protocol`, `hardware`, `documentation`, `testing`, `performance`

**Status:**
- `good first issue`, `help wanted`, `blocked`, `wip`

**Phase:**
- `foundation`, `access-control`, `device-management`, `advanced`

**Week:**
- `week-1`, `week-2`, ... `week-8`

## 📦 Release Process

### Version Format: SemVer

- `v1.0.0` - Major (breaking changes)
- `v1.1.0` - Minor (new features)
- `v1.1.1` - Patch (bug fixes)

### Creating Release

```bash
# 1. Update version in Cargo.toml files
# 2. Commit version bump
git commit -m "chore: bump version to 1.0.0"

# 3. Create tag
git tag -a v1.0.0 -m "Release v1.0.0"

# 4. Push
git push origin main --tags

# 5. GitHub Actions automatically:
#    - Creates GitHub release
#    - Builds binaries (Linux x86_64, ARM64, ARMv7)
#    - Publishes to crates.io
```

## ✅ Pre-Push Checklist

Before pushing your branch:

```bash
# Format code
cargo fmt --all

# Check for warnings
cargo clippy --workspace -- -D warnings

# Run all tests
cargo test --workspace

# Build release (optional)
cargo build --workspace --release

# Check documentation
cargo doc --workspace --no-deps
```

## 🚀 Example Complete Workflow

```bash
# 1. Pick issue #1: Implement Message and Frame structures
# 2. Create branch
git checkout main
git pull origin main
git checkout -b feature/1-message-structures

# 3. Develop
# ... write code in turnkey-protocol/src/message.rs ...

# 4. Test frequently
cargo test --package turnkey-protocol

# 5. Commit
git add turnkey-protocol/src/message.rs
git commit -m "feat: add Message struct with builder pattern

Implement core Message and Frame structures for Henry
protocol with conversion traits and unit tests.

Closes #1"

# 6. Run checks
cargo fmt
cargo clippy --package turnkey-protocol
cargo test --package turnkey-protocol

# 7. Push
git push origin feature/1-message-structures

# 8. Create PR on GitHub
#    - Use PR template
#    - Link to issue #1
#    - Wait for CI to pass

# 9. Address review feedback
#    - Make requested changes
#    - Push new commits

# 10. Merge after approval
#     - Issue #1 closes automatically
#     - Branch can be deleted

# 11. Start next issue
git checkout main
git pull origin main
git branch -d feature/1-message-structures
```

## 🆘 Common Issues

### CI Failing?

```bash
# Format issues
cargo fmt --all

# Clippy warnings
cargo clippy --workspace --fix

# Test failures
cargo test --workspace -- --nocapture

# Build errors
cargo clean
cargo build --workspace
```

### Merge Conflicts?

```bash
git checkout feature/N-description
git fetch origin
git merge origin/main
# Resolve conflicts
git add .
git commit -m "chore: merge main and resolve conflicts"
git push
```

### Need to Update PR?

```bash
# Make changes
# ... edit files ...

# Amend last commit (if just fixing previous commit)
git add .
git commit --amend --no-edit
git push --force-with-lease

# Or add new commit (preferred)
git add .
git commit -m "fix: address review feedback"
git push
```

## 📚 Additional Resources

- [Full Workflow Guide](.github/WORKFLOW.md)
- [Contributing Guide](.github/CONTRIBUTING.md)
- [Project Documentation](../docs/)
- [Issues](https://github.com/marmota-alpina/turnkey/issues)
- [Pull Requests](https://github.com/marmota-alpina/turnkey/pulls)

## 💡 Tips

- **Keep PRs small**: Easier to review, faster to merge
- **Test locally**: Don't rely on CI to catch issues
- **Communicate**: Comment on issues, ask questions in PRs
- **Be patient**: Reviews take time, feedback is valuable
- **Have fun**: You're building something cool! 🚀

---

**Questions?** Open a [Discussion](https://github.com/marmota-alpina/turnkey/discussions)

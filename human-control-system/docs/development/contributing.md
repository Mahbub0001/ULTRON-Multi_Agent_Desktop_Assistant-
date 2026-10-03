# Contributing Guide

## Welcome

Thank you for contributing to the Human Control System! This guide will help you get started with contributing code, documentation, and ideas.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct:
- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- No harassment, discrimination, or offensive behavior

## Ways to Contribute

1. **Code Contributions** - Bug fixes, features, refactoring
2. **Documentation** - Guides, API docs, examples
3. **Testing** - Unit tests, integration tests, bug reports
4. **Design** - Architecture proposals, UI/UX improvements
5. **Community** - Answering questions, reviewing PRs, triaging issues

## Getting Started

### 1. Fork & Clone
```bash
# Fork on GitHub, then clone your fork
git clone https://github.com/YOUR_USERNAME/human-control-system.git
cd human-control-system

# Add upstream remote
git remote add upstream https://github.com/your-org/human-control-system.git
```

### 2. Set Up Development Environment
```bash
# Install prerequisites (see building.md)
# Rust, Python, CMake, ARM toolchain

# Install pre-commit hooks
pip install pre-commit
pre-commit install

# Build to verify setup
cargo build --workspace
cd vision && source venv/bin/activate && pip install -r requirements.txt
```

### 3. Create a Branch
```bash
git fetch upstream
git checkout upstream/main
git checkout -b feature/your-feature-name
# or: fix/bug-description, docs/topic, refactor/component
```

## Development Workflow

### Branch Naming
| Type | Pattern | Example |
|------|---------|---------|
| Feature | `feature/<short-description>` | `feature/add-voice-commands` |
| Bug Fix | `fix/<issue-number>-<description>` | `fix/123-memory-leak-driver` |
| Documentation | `docs/<topic>` | `docs/update-api-reference` |
| Refactor | `refactor/<component>` | `refactor/auth-service` |
| Chore | `chore/<task>` | `chore/update-dependencies` |

### Commit Guidelines

#### Commit Message Format
```
<type>(<scope>): <subject>

<body>

<footer>
```

#### Types
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation only
- `style` - Formatting, missing semicolons, etc.
- `refactor` - Code restructuring without behavior change
- `perf` - Performance improvement
- `test` - Adding/updating tests
- `chore` - Maintenance, dependencies, build
- `ci` - CI/CD changes

#### Examples
```
feat(input): add synthetic driver for testing

Adds a new SyntheticDriver implementation that records injected
events in memory for unit testing without requiring kernel drivers.

Closes #45
```

```
fix(vision): handle empty detection results

The YOLO detector crashed when ONNX returned empty results.
Added proper empty array handling.

Fixes #234
```

```
docs(api): update agent service reference

Added examples for Python, Rust, and Go clients.
Documented all error codes and rate limits.
```

### Code Style

#### Rust
```bash
# Format
cargo fmt --all

# Lint
cargo clippy --workspace -- -D warnings

# Check
cargo check --workspace
```

**Style Rules:**
- Use `rustfmt` default style
- Maximum line width: 100 chars
- Prefer `?` over `unwrap()` in production code
- Use `anyhow::Result` for error handling
- Document public APIs with `///` comments
- Use `snake_case` for functions/variables, `PascalCase` for types

#### Python
```bash
# Format
black src/ tests/

# Lint
ruff check src/ tests/

# Type check
mypy src/
```

**Style Rules:**
- Black formatter (line length 100)
- Type hints on all functions
- Google-style docstrings
- Use `async/await` for async code
- Prefer `pathlib` over `os.path`

#### C (Firmware)
```bash
# Format
clang-format -i firmware/**/*.c firmware/**/*.h

# Static analysis
cppcheck --enable=all --std=c99 firmware/
```

**Style Rules:**
- C99 standard
- 4-space indentation
- `snake_case` for functions/variables
- `UPPER_CASE` for constants/macros
- Prefix static functions with `static`
- Document hardware-specific code

### Pre-commit Checks
```bash
# Run all checks locally
pre-commit run --all-files

# Or individually
cargo fmt --check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace --lib
cd vision && black --check src/ tests/ && ruff check src/ tests/
```

## Pull Request Process

### 1. Before Submitting
- [ ] All tests pass: `cargo test --workspace && cd vision && pytest tests/`
- [ ] Code formatted: `cargo fmt --check && black --check`
- [ ] No clippy warnings: `cargo clippy --workspace -- -D warnings`
- [ ] Documentation updated if needed
- [ ] Changelog entry added (see below)

### 2. PR Template
```markdown
## Description
Brief description of changes

## Type
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Refactor
- [ ] Performance
- [ ] Test

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing performed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Changelog entry added
- [ ] No breaking changes (or documented)

## Related Issues
Closes #123
```

### 3. Review Process
1. **Automated checks** - CI runs tests, linting, formatting
2. **Code review** - At least one maintainer review required
3. **Approval** - Maintainer approves
4. **Merge** - Squash and merge to main

### 4. Review Guidelines for Reviewers
- Be constructive and specific
- Focus on correctness, maintainability, security
- Ask questions rather than demanding changes
- Approve when code meets standards
- Request changes with clear reasoning

## Changelog

Add entry to `CHANGELOG.md` under `## [Unreleased]`:

```markdown
## [Unreleased]

### Added
- New synthetic input driver for testing (#45)

### Fixed
- Vision service crash on empty detection results (#234)

### Changed
- Updated agent API to v1.2.0

### Security
- Updated dependencies to fix CVE-2024-XXXX
```

## Issue Reporting

### Bug Report Template
```markdown
**Describe the bug**
Clear description of the issue

**To Reproduce**
1. Step 1
2. Step 2
3. Step 3

**Expected behavior**
What should happen

**Actual behavior**
What actually happens

**Environment**
- OS: [Windows 11 / Ubuntu 22.04 / macOS 14]
- Rust version: `rustc --version`
- Python version: `python --version`
- Hardware: [Teensy 4.0 / CPU / GPU]

**Logs/Output**
```
Paste relevant logs here
```

**Additional context**
Screenshots, config files, etc.
```

### Feature Request Template
```markdown
**Problem**
What problem does this solve?

**Proposed Solution**
Describe the feature

**Alternatives Considered**
Other approaches

**Implementation Ideas**
Technical approach if known

**Additional Context**
Mockups, references, etc.
```

## Architecture Decisions

For significant changes, create an ADR (Architecture Decision Record):

```
docs/adr/
├── 001-use-rust-for-core.md
├── 002-grpc-for-ipc.md
├── 003-wasm-for-plugins.md
└── 004-yaml-dsl-for-tasks.md
```

### ADR Template
```markdown
# ADR 00X: Title

## Status
Proposed / Accepted / Superseded

## Context
What problem are we solving?

## Decision
What we decided to do.

## Consequences
### Positive
- Benefit 1
- Benefit 2

### Negative
- Drawback 1

## Alternatives Considered
- Alternative 1: Why not chosen
- Alternative 2: Why not chosen
```

## Security

### Reporting Vulnerabilities
**Do not** open public issues for security vulnerabilities.

Email: security@your-org.com

Include:
- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Security Checklist for Contributors
- [ ] No hardcoded secrets/tokens
- [ ] Input validation on all boundaries
- [ ] Capability checks on all gRPC methods
- [ ] No unsafe code without justification
- [ ] Dependencies updated regularly

## Release Process

### Versioning
We use [Semantic Versioning](https://semver.org/):
- `MAJOR.MINOR.PATCH`
- Breaking changes → MAJOR
- New features (backward compatible) → MINOR
- Bug fixes → PATCH

### Release Checklist (Maintainers)
1. Update version in `Cargo.toml` workspace
2. Update `CHANGELOG.md` with release notes
3. Create release branch: `release/v1.2.0`
4. Run full test suite
5. Build release artifacts
6. Create GitHub release with binaries
7. Publish to crates.io / PyPI
8. Merge release branch to main
9. Tag: `git tag v1.2.0`

## Community

### Communication Channels
- **GitHub Issues** - Bug reports, feature requests
- **GitHub Discussions** - Questions, ideas, general discussion
- **Discord** - Real-time chat (link in repo)
- **Email** - security@your-org.com for security issues

### Getting Help
- Check existing issues/discussions first
- Provide minimal reproduction case
- Include environment details
- Be patient - maintainers are volunteers

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- GitHub contributors graph
- Annual contributor spotlight

## License

By contributing, you agree that your contributions will be licensed under the project's license (MIT/Apache-2.0 dual license).

## Questions?

Open a GitHub Discussion or ask in Discord. We're happy to help!
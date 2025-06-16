# Contributing to Tide Tracker

Thank you for your interest in contributing to Tide Tracker! This document provides guidelines and information for contributors.

## 🚀 Quick Start

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/tide-tracker.git
   cd tide-tracker
   ```
3. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
4. **Make your changes**
5. **Test your changes**:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy
   ```
6. **Commit and push**:
   ```bash
   git commit -m "feat: add your feature description"
   git push origin feature/your-feature-name
   ```
7. **Create a Pull Request** on GitHub

## 🛠️ Development Setup

### Prerequisites
- Rust 1.75 or later
- Git

### Optional (for testing e-ink functionality)
- Raspberry Pi with Waveshare 4.2" e-ink display
- Linux environment for cross-compilation testing

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Cross-compilation for Raspberry Pi
cargo install cross
cross build --target aarch64-unknown-linux-gnu --release
```

### Testing
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Test documentation examples
cargo test --doc
```

## 📋 Code Style

- **Formatting**: Use `cargo fmt` for consistent formatting
- **Linting**: Address all `cargo clippy` warnings
- **Documentation**: Document public APIs with `///` comments
- **Testing**: Add tests for new functionality
- **Error Handling**: Use proper error types and handling

### Commit Convention
We use conventional commits:
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation changes
- `style:` - Code style changes (formatting, etc.)
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `chore:` - Maintenance tasks

## 🧪 Testing Guidelines

### Unit Tests
- Add tests for new functions in the same file
- Use descriptive test names
- Test both success and failure cases

### Integration Tests
- Test complete workflows
- Verify configuration handling
- Test with real NOAA data when possible

### Hardware Testing
- Test on actual Raspberry Pi hardware when possible
- Verify e-ink display functionality
- Check memory usage and performance

## 📦 Release Process

Releases are automated via GitHub Actions:

1. **Create a release** using the local script:
   ```bash
   ./scripts/release.sh 1.2.3
   ```
2. **Push the changes**:
   ```bash
   git push origin main && git push origin v1.2.3
   ```
3. **GitHub Actions** will automatically:
   - Run tests
   - Build cross-platform binaries
   - Create a GitHub release
   - Upload release assets

## 🔧 Configuration

The project uses:
- **Cargo.toml**: Rust package configuration
- **Cross.toml**: Cross-compilation settings
- **tide-config.toml**: Runtime configuration
- **GitHub Actions**: CI/CD workflows

## 📚 Project Structure

```
tide-tracker/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library root
│   ├── config.rs        # Configuration handling
│   ├── tide_data.rs     # NOAA API and data processing
│   ├── renderer.rs      # ASCII and e-ink rendering
│   ├── fallback.rs      # Offline tide calculations
│   └── tests/           # Integration tests
├── .github/
│   ├── workflows/       # GitHub Actions
│   └── ISSUE_TEMPLATE/  # Issue templates
├── docs/                # Documentation
└── scripts/             # Utility scripts
```

## 🤝 Areas for Contribution

### High Priority
- Additional NOAA station support
- Performance optimizations
- Memory usage improvements
- Better error messages

### Medium Priority
- Additional display formats
- Configuration validation
- More comprehensive tests
- Documentation improvements

### Low Priority
- Code refactoring
- Developer tooling
- CI/CD improvements

## 📝 Documentation

- **Code comments**: Use `///` for public APIs
- **README**: Keep installation and usage instructions updated
- **Configuration**: Document all config options
- **Examples**: Provide working examples

## 🐛 Reporting Issues

Please use the GitHub issue templates:
- **Bug Report**: For reporting bugs
- **Feature Request**: For suggesting new features

Include:
- Operating system and hardware
- Tide Tracker version
- Configuration file (sanitized)
- Steps to reproduce
- Expected vs actual behavior

## 💬 Getting Help

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Code Review**: Request reviews on pull requests

## 📄 License

By contributing to Tide Tracker, you agree that your contributions will be licensed under the MIT license.

---

Thank you for contributing to Tide Tracker! 🌊

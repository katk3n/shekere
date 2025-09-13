# Suggested Commands for Shekere Development

## Build and Run
```bash
# Build the project
cargo build

# Run with a configuration file
cargo run -- examples/spectrum/spectrum.toml

# Build release version
cargo build --release
```

## Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Code Quality (MANDATORY)
```bash
# Check code formatting (run before commits)
cargo fmt --check

# Format code (MANDATORY before commits)
cargo fmt

# Run clippy lints
cargo clippy

# Run clippy with all targets
cargo clippy --all-targets
```

## Development Workflow
```bash
# Run example projects for testing
cargo run -- examples/basic/basic.toml
cargo run -- examples/spectrum/spectrum.toml
cargo run -- examples/osc/osc.toml

# Install from local source
cargo install --path .
```

## System Commands (Darwin)
```bash
# File operations
ls -la
find . -name "*.rs" -type f
grep -r "pattern" src/

# Git operations
git status
git add .
git commit -m "message"
git log --oneline

# Process management
ps aux | grep shekere
kill -9 <pid>
```

## Project-specific Commands
```bash
# Run with debug logging
RUST_LOG=debug cargo run -- examples/spectrum/spectrum.toml

# Run with specific log level
RUST_LOG=shekere=info cargo run -- examples/persistent/persistent_accumulate.toml

# Test with timeout for long-running tests
timeout 10s cargo run -- examples/ping_pong/reaction_diffusion.toml
```
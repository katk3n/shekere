# Task Completion Checklist

## MANDATORY Steps Before Committing

### 1. Code Formatting (CRITICAL)
```bash
# ALWAYS run and ensure it passes
cargo fmt

# Check formatting without making changes
cargo fmt --check
```

### 2. Code Quality Checks
```bash
# Run clippy lints
cargo clippy

# Run clippy on all targets
cargo clippy --all-targets
```

### 3. Testing
```bash
# Run all tests
cargo test

# Run tests with output for debugging
cargo test -- --nocapture

# Run specific integration tests if relevant
cargo test hot_reload
cargo test multi_pass
```

### 4. Build Verification
```bash
# Ensure project builds successfully
cargo build

# Test release build
cargo build --release
```

## Development Methodology (TDD)

### Red-Green-Refactor Cycle
1. **Red**: Write a failing test first
2. **Green**: Write minimal code to make test pass
3. **Refactor**: Improve code while keeping tests passing

### Testing Requirements
- **Unit Tests**: >90% coverage for new features
- **Integration Tests**: For configuration parsing and feature interactions
- **Mock External Dependencies**: File system, network, audio devices
- **Descriptive Test Names**: Explain the scenario being tested

## Documentation Updates

### When Adding Features
1. **Update README.md**: Configuration options, uniforms, examples
2. **Update API Reference**: New uniforms and helper functions
3. **Update CLAUDE.md**: Architectural patterns and design decisions
4. **Maintain Consistency**: Between docs, tests, and examples

## Shader Development
- **Check `shaders/common.wgsl`**: Don't redefine existing structures
- **Follow WGSL Standards**: Proper uniform binding and alignment
- **Test Hot Reload**: Ensure shader changes reload properly
- **Update Examples**: Provide working example configurations
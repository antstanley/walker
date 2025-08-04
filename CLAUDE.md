# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run directly
cargo run -- [OPTIONS]
```

### Testing
```bash
# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests in a specific module
cargo test --lib module_name
```

### Linting and Format
```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy

# Run linter with all targets
cargo clippy --all-targets --all-features
```

### Coverage
```bash
# Generate coverage report (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

## Architecture

Walker is a Rust CLI application that analyzes Node.js packages in directory structures. The codebase follows a modular architecture:

### Core Components

1. **CLI Layer** (`src/cli/`)
   - `args.rs`: Command-line argument parsing using clap
   - `commands.rs`: Command execution logic
   - Entry point converts arguments to commands for execution

2. **Core Analysis** (`src/core/`)
   - `walker.rs`: Main directory traversal logic
   - `parallel_walker.rs`: Parallel processing implementation using rayon
   - `analyzer.rs`: Package analysis logic (ESM/CommonJS detection)
   - `cache.rs`: Caching system for performance optimization
   - `streaming.rs`: Streaming analysis for large directories

3. **Configuration** (`src/config/`)
   - `settings.rs`: Main settings structure
   - `parser.rs`: TOML configuration parsing
   - `file.rs`: Configuration file discovery and loading
   - Supports hierarchical config: CLI args > file config > defaults

4. **Error Handling** (`src/error/`)
   - `types.rs`: Custom error types with severity levels
   - `context.rs`: Error context and user-friendly messages
   - All errors include suggestions for resolution

5. **Models** (`src/models/`)
   - `package.rs`: Package data structures (ModuleSupport, PackageDetails)
   - `analysis.rs`: Analysis results and summary structures
   - `config.rs`: Configuration-related models

6. **Output** (`src/output/`)
   - `formatters.rs`: Text, JSON, and CSV formatters
   - `progress.rs`: Progress reporting using indicatif
   - `writers.rs`: File and stdout output handling

7. **Parsers** (`src/parsers/`)
   - `package_json.rs`: package.json parsing and analysis
   - `exports.rs`: Module exports detection logic

### Key Design Patterns

- **Error Recovery**: Uses `try_with_recovery` for graceful error handling
- **Parallel Processing**: Default parallel analysis with opt-out via `--no-parallel`
- **Builder Pattern**: ConfigBuilder for flexible configuration
- **Strategy Pattern**: Pluggable formatters and writers
- **Progress Reporting**: Non-blocking progress updates during analysis

### Module System Detection Logic

The analyzer checks for ESM support by examining:
- `"type": "module"` in package.json
- Presence of .mjs files
- Export maps in package.json

CommonJS support is detected through:
- Default behavior (absence of `"type": "module"`)
- Presence of .cjs files
- CommonJS-specific fields in package.json

### Performance Considerations

- Parallel processing by default using rayon
- Optional caching system to avoid re-analyzing unchanged packages
- Size calculation can be disabled with `--no-size` for faster scans
- Progress reporting designed to not impact performance
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build
```bash
# Development build
cargo build

# Release build
cargo build --release

# Run directly with options
cargo run -- [OPTIONS]

# Run with common options
cargo run -- --path ./target-dir --output json
cargo run -- --exclude node_modules --no-size
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output visible
cargo test -- --nocapture

# Run specific test by name
cargo test test_cli_args_parsing

# Run tests in specific module
cargo test --lib core
cargo test --lib config

# Run only integration tests
cargo test --test integration_tests

# Run only unit tests
cargo test --lib

# Run benchmarks
cargo test --test integration_tests benchmark
```

### Linting and Format
```bash
# Format code
cargo fmt

# Check formatting without changes
cargo fmt -- --check

# Run clippy linter
cargo clippy

# Run clippy with all targets and features
cargo clippy --all-targets --all-features

# Fix clippy warnings automatically
cargo clippy --fix
```

### Coverage
```bash
# Generate coverage report (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out html
```

## Architecture

Walker is a Rust CLI application that analyzes Node.js packages in directory structures. The codebase follows a modular architecture with clear separation of concerns.

### Core Components

1. **CLI Layer** (`src/cli/`)
   - `args.rs`: Command-line argument parsing using clap derive macros
   - `commands.rs`: Command execution logic and orchestration
   - Converts parsed arguments into executable commands

2. **Core Analysis** (`src/core/`)
   - `walker.rs`: Main directory traversal logic using walkdir
   - `parallel_walker.rs`: Parallel processing implementation using rayon thread pool
   - `analyzer.rs`: Package analysis logic for module system detection
   - `cache.rs`: File-based caching system with timestamp validation
   - `streaming.rs`: Streaming analysis for memory-efficient large directory handling
   - `parallel.rs`: Parallel processing utilities and thread pool configuration

3. **Configuration** (`src/config/`)
   - `settings.rs`: Main Settings struct with all configuration options
   - `parser.rs`: TOML configuration file parsing
   - `file.rs`: Configuration file discovery (checks .walker.toml, ~/.walker.toml, XDG paths)
   - `cli.rs`: CLI configuration conversion to Settings
   - Hierarchy: CLI args override file config which overrides defaults

4. **Error Handling** (`src/error/`)
   - `types.rs`: WalkerError enum with variants for different error types
   - `context.rs`: Error context traits using ResultExt for adding user-friendly messages
   - All errors include severity levels (Warning, Error, Critical) and resolution suggestions

5. **Models** (`src/models/`)
   - `package.rs`: Core data structures (ModuleSupport, PackageDetails, DependencyCounts)
   - `analysis.rs`: AnalysisResults, AnalysisSummary, ErrorTracking, PerformanceMetrics
   - `config.rs`: Configuration-related models and OutputFormat enum

6. **Output** (`src/output/`)
   - `formatters.rs`: Trait-based formatters (TextFormatter, JsonFormatter, CsvFormatter)
   - `progress.rs`: Progress reporting using indicatif with spinner and progress bars
   - `writers.rs`: Output writers for file and stdout with automatic format detection

7. **Parsers** (`src/parsers/`)
   - `package_json.rs`: Comprehensive package.json parsing with serde_json
   - `exports.rs`: Module exports and conditional exports detection

### Key Design Patterns

- **Error Recovery**: `try_with_recovery` pattern for continuing after non-critical errors
- **Parallel Processing**: Default parallel analysis with work-stealing using rayon
- **Builder Pattern**: ConfigBuilder for incremental configuration construction
- **Strategy Pattern**: Formatter trait for pluggable output formats
- **Observer Pattern**: Progress callbacks for non-blocking UI updates
- **Resource Management**: RAII with Drop traits for cleanup

### Module System Detection Logic

The analyzer (`src/core/analyzer.rs`) determines module support through:

**ESM Detection**:
- `"type": "module"` in package.json
- Presence of `.mjs` files in package directory
- `"exports"` field with ESM-specific conditions
- `"module"` field pointing to ESM entry

**CommonJS Detection**:
- Default when no `"type": "module"` is present
- Presence of `.cjs` files
- `"main"` field without ESM indicators
- `require()` usage patterns in exports

**Dual Mode Detection**:
- Both ESM and CommonJS indicators present
- Conditional exports supporting both `"import"` and `"require"`

### Performance Considerations

- **Parallel by Default**: Uses available CPU cores via rayon
- **Smart Caching**: SHA-256 based cache invalidation in `~/.cache/walker`
- **Size Calculation**: Optional via `--no-size` flag (major performance boost)
- **Progress Reporting**: Separate thread to avoid blocking analysis
- **Memory Efficiency**: Streaming mode for directories with >10,000 packages

### Testing Structure

Tests are organized into:
- **Unit Tests**: In `tests/unit/` for individual components
- **Integration Tests**: In `tests/integration/` for end-to-end scenarios
- **Test Fixtures**: In `tests/fixtures/` with sample package structures
- Tests use `tempfile` for isolated file system operations

### Configuration Priority

Configuration is resolved in this order (highest priority first):
1. Command-line arguments
2. Configuration file specified via `--config`
3. `.walker.toml` in current directory
4. `~/.walker.toml` in home directory
5. `~/.config/walker/config.toml` (XDG config)
6. Built-in defaults from `src/config/default_config.toml`
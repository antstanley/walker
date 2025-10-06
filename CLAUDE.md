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

### Dependencies

Walker uses the following key dependencies:

**Core Parsing**:
- `oxc_parser` v0.93.0 - High-performance JavaScript/TypeScript parser
- `oxc_ast` v0.93.0 - Abstract syntax tree definitions
- `oxc_allocator` v0.93.0 - Arena allocator for AST nodes
- `oxc_span` v0.93.0 - Source code span tracking
- `oxc_syntax` v0.93.0 - Syntax definitions and utilities
- `oxc_diagnostics` v0.93.0 - Error diagnostics

**Concurrency & Performance**:
- `rayon` v1.11.0 - Data parallelism
- `dashmap` v6.1.0 - Concurrent hash map
- `parking_lot` v0.12.5 - Efficient synchronization primitives
- `num_cpus` v1.17.0 - CPU core detection

**Other**:
- `petgraph` v0.8.3 - Dependency graph algorithms
- `lru` v0.16.1 - LRU caching

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
   - `file_metadata.rs`: File-level AST analysis data (FileMetadata, ModuleSystem, ExportedSymbol, ImportedSymbol)
   - `ast.rs`: AST analysis results and dependency graph structures

6. **Output** (`src/output/`)
   - `formatters.rs`: Trait-based formatters (TextFormatter, JsonFormatter, CsvFormatter)
   - `progress.rs`: Progress reporting using indicatif with spinner and progress bars
   - `writers.rs`: Output writers for file and stdout with automatic format detection

7. **Parsers** (`src/parsers/`)
   - `package_json.rs`: Comprehensive package.json parsing with serde_json
   - `exports.rs`: Module exports and conditional exports detection
   - `ast_parser.rs`: OXC-based AST parser with allocator pooling for thread safety
   - `module_detector.rs`: AST visitor pattern for detecting module system usage
   - `dependency_graph_builder.rs`: Build dependency graphs from parsed AST

### Key Design Patterns

- **Error Recovery**: `try_with_recovery` pattern for continuing after non-critical errors
- **Parallel Processing**: Default parallel analysis with work-stealing using rayon
- **Builder Pattern**: ConfigBuilder for incremental configuration construction
- **Strategy Pattern**: Formatter trait for pluggable output formats
- **Observer Pattern**: Progress callbacks for non-blocking UI updates
- **Resource Management**: RAII with Drop traits for cleanup
- **Visitor Pattern**: AST traversal in `module_detector.rs` for detecting module patterns
- **Object Pool Pattern**: Allocator pooling in `ast_parser.rs` for memory efficiency

### Module System Detection Logic

The analyzer (`src/core/analyzer.rs`) determines module support through two layers:

**1. Package.json-based Detection** (fast, static analysis):
- `"type": "module"` in package.json indicates ESM
- Presence of `.mjs` files indicates ESM
- `"exports"` field with ESM-specific conditions
- `"module"` field pointing to ESM entry
- Presence of `.cjs` files indicates CommonJS
- `"main"` field without ESM indicators indicates CommonJS
- Conditional exports supporting both `"import"` and `"require"` indicates dual mode

**2. AST-based Detection** (deep, syntax-aware analysis using OXC):

Walker integrates the **OXC Parser** (oxc_parser, oxc_ast, oxc_allocator) for high-performance JavaScript/TypeScript AST analysis.

**OXC Integration Architecture**:
- `ast_parser.rs`: Main parser wrapper with thread-safe allocator pooling
  - Uses `oxc_allocator::Allocator` with pooling to avoid allocation overhead
  - Supports all JavaScript/TypeScript file types via `oxc_span::SourceType`
  - Processes AST immediately to avoid lifetime issues
  - Returns ownership-friendly `FileAnalysis` with extracted data

- `module_detector.rs`: AST visitor for module system detection
  - Implements visitor pattern to traverse `oxc_ast` nodes
  - Detects ESM syntax: `import`/`export` statements, dynamic `import()`
  - Detects CommonJS syntax: `require()` calls, `module.exports`, `exports.xxx`
  - Extracts imported/exported symbols with line numbers
  - Tracks circular dependencies

**AST-based ESM Detection**:
- `import` declarations (static imports)
- `export` named/default/all declarations
- Dynamic `import()` expressions
- Identifies imported/exported symbols and their types (function, class, variable)

**AST-based CommonJS Detection**:
- `require()` function calls with module paths
- `module.exports` assignments
- `exports.xxx` property assignments
- Detects mixed usage patterns

**Mixed/Dual Mode Detection**:
- Both ESM and CommonJS syntax present in same file
- Conditional exports supporting multiple formats
- Separate import/require entry points

**Performance Optimizations**:
- Allocator pooling reduces GC pressure during parallel parsing
- Parse results extracted immediately to avoid AST lifetime constraints
- Thread-safe via `parking_lot::RwLock` for concurrent access
- Graceful error handling for invalid syntax

### Performance Considerations

- **Parallel by Default**: Uses available CPU cores via rayon
- **Smart Caching**: SHA-256 based cache invalidation in `~/.cache/walker`
- **Size Calculation**: Optional via `--no-size` flag (major performance boost)
- **Progress Reporting**: Separate thread to avoid blocking analysis
- **Memory Efficiency**: Streaming mode for directories with >10,000 packages
- **AST Parsing Optimization**:
  - Allocator pooling (size = number of CPU cores) for reduced allocations
  - Immediate AST processing to avoid lifetime/borrowing overhead
  - Thread-safe concurrent parsing via parking_lot synchronization primitives
  - OXC parser is among the fastest JavaScript parsers available

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
# Design Document

## Overview

This design document outlines the architectural improvements for the Walker tool, transforming it from a monolithic single-file application into a robust, modular, and extensible CLI tool. The design focuses on separation of concerns, error resilience, performance optimization, and user experience enhancement.

The refactored Walker will maintain backward compatibility in terms of core functionality while adding significant new capabilities through a well-structured, maintainable codebase.

## Architecture

### High-Level Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Layer     │    │  Config Layer   │    │  Output Layer   │
│                 │    │                 │    │                 │
│ • Argument      │    │ • File Config   │    │ • Formatters    │
│   Parsing       │    │ • CLI Config    │    │ • Writers       │
│ • Help/Version  │    │ • Validation    │    │ • Progress      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Core Engine    │
                    │                 │
                    │ • Walker        │
                    │ • Analyzer      │
                    │ • Cache         │
                    └─────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Data Layer     │
                    │                 │
                    │ • Models        │
                    │ • Parsers       │
                    │ • Validators    │
                    └─────────────────┘
```

### Module Structure

```
src/
├── main.rs                 # Entry point and CLI orchestration
├── lib.rs                  # Library exports and common types
├── cli/
│   ├── mod.rs             # CLI module exports
│   ├── args.rs            # Command line argument parsing
│   └── commands.rs        # Command implementations
├── config/
│   ├── mod.rs             # Configuration module exports
│   ├── file.rs            # Configuration file handling
│   └── settings.rs        # Configuration data structures
├── core/
│   ├── mod.rs             # Core module exports
│   ├── walker.rs          # Directory walking logic
│   ├── analyzer.rs        # Package analysis logic
│   ├── cache.rs           # Caching implementation
│   └── parallel.rs        # Parallel processing utilities
├── models/
│   ├── mod.rs             # Model exports
│   ├── package.rs         # Package data structures
│   ├── analysis.rs        # Analysis result structures
│   └── config.rs          # Configuration structures
├── parsers/
│   ├── mod.rs             # Parser exports
│   ├── package_json.rs    # Package.json parsing
│   └── exports.rs         # Exports field parsing
├── output/
│   ├── mod.rs             # Output module exports
│   ├── formatters.rs      # Output format implementations
│   ├── writers.rs         # File writing utilities
│   └── progress.rs        # Progress indication
└── error/
    ├── mod.rs             # Error handling exports
    └── types.rs           # Error type definitions
```

## Components and Interfaces

### 1. CLI Layer (`src/cli/`)

**Purpose**: Handle command-line interface, argument parsing, and user interaction.

**Key Components**:

- `Args`: Command-line argument structure using `clap`
- `Commands`: Individual command implementations
- `Help`: Help text and usage information

**Interface**:

```rust
pub struct Args {
    pub path: Option<PathBuf>,
    pub exclude: Vec<String>,
    pub max_depth: Option<usize>,
    pub output_format: OutputFormat,
    pub output_file: Option<PathBuf>,
    pub quiet: bool,
    pub verbose: bool,
    pub no_size: bool,
    pub config: Option<PathBuf>,
}

pub enum Command {
    Analyze(Args),
    Version,
    Help,
}
```

### 2. Configuration Layer (`src/config/`)

**Purpose**: Manage configuration from files and command-line arguments.

**Key Components**:

- `Settings`: Merged configuration structure
- `FileConfig`: Configuration file parsing
- `ConfigBuilder`: Configuration merging logic

**Interface**:

```rust
pub struct Settings {
    pub scan_path: PathBuf,
    pub exclude_patterns: Vec<String>,
    pub max_depth: Option<usize>,
    pub output_format: OutputFormat,
    pub calculate_size: bool,
    pub parallel: bool,
    pub cache_enabled: bool,
}

pub trait ConfigSource {
    fn load(&self) -> Result<PartialSettings>;
}
```

### 3. Core Engine (`src/core/`)

**Purpose**: Implement the main business logic for directory walking and package analysis.

**Key Components**:

- `Walker`: Directory traversal with error handling
- `Analyzer`: Package analysis logic
- `Cache`: Result caching for performance
- `ParallelWalker`: Concurrent directory processing

**Interface**:

```rust
pub struct Walker {
    settings: Settings,
    cache: Option<Cache>,
}

impl Walker {
    pub fn new(settings: Settings) -> Self;
    pub fn analyze(&self) -> Result<AnalysisResults>;
    pub fn analyze_with_progress<F>(&self, progress_fn: F) -> Result<AnalysisResults>
    where F: Fn(ProgressUpdate);
}

pub struct Analyzer;

impl Analyzer {
    pub fn analyze_package(path: &Path) -> Result<PackageAnalysis>;
    pub fn parse_package_json(content: &str) -> Result<PackageDetails>;
}
```

### 4. Data Models (`src/models/`)

**Purpose**: Define data structures for packages, analysis results, and configuration.

**Key Structures**:

```rust
pub struct PackageAnalysis {
    pub path: PathBuf,
    pub details: PackageDetails,
    pub module_support: ModuleSupport,
    pub size: Option<u64>,
    pub dependencies: DependencyInfo,
    pub typescript_support: bool,
    pub browser_support: bool,
    pub node_version_requirement: Option<String>,
    pub license: Option<String>,
}

pub struct ModuleSupport {
    pub esm: EsmSupport,
    pub cjs: CjsSupport,
}

pub struct EsmSupport {
    pub type_module: bool,
    pub exports_import: bool,
    pub module_field: bool,
    pub main_mjs: bool,
    pub overall: bool,
}

pub struct AnalysisResults {
    pub packages: Vec<PackageAnalysis>,
    pub summary: AnalysisSummary,
    pub errors: Vec<AnalysisError>,
}
```

### 5. Output Layer (`src/output/`)

**Purpose**: Handle different output formats and progress reporting.

**Key Components**:

- `Formatter`: Trait for different output formats
- `TextFormatter`, `JsonFormatter`, `CsvFormatter`: Format implementations
- `ProgressReporter`: Progress indication during analysis

**Interface**:

```rust
pub trait Formatter {
    fn format(&self, results: &AnalysisResults) -> Result<String>;
}

pub struct TextFormatter {
    pub use_colors: bool,
    pub verbose: bool,
}

pub struct ProgressReporter {
    pub quiet: bool,
}

impl ProgressReporter {
    pub fn start(&self, total: usize);
    pub fn update(&self, current: usize, message: &str);
    pub fn finish(&self);
}
```

### 6. Error Handling (`src/error/`)

**Purpose**: Centralized error handling with context and recovery strategies.

**Key Components**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum WalkerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error in {file}: {source}")]
    JsonParse { file: PathBuf, source: serde_json::Error },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Permission denied accessing {path}")]
    PermissionDenied { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, WalkerError>;
```

## Data Models

### Core Data Structures

```rust
// Enhanced package details with comprehensive information
pub struct PackageDetails {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub main: Option<String>,
    pub module: Option<String>,
    pub types: Option<String>,
    pub browser: Option<serde_json::Value>,
    pub exports: Option<serde_json::Value>,
    pub package_type: Option<String>,
    pub engines: Option<serde_json::Value>,
    pub license: Option<String>,
    pub dependencies: Option<serde_json::Value>,
    pub dev_dependencies: Option<serde_json::Value>,
    pub peer_dependencies: Option<serde_json::Value>,
    pub optional_dependencies: Option<serde_json::Value>,
}

// Comprehensive dependency analysis
pub struct DependencyInfo {
    pub production_count: usize,
    pub development_count: usize,
    pub peer_count: usize,
    pub optional_count: usize,
    pub total_count: usize,
}

// Analysis summary for reporting
pub struct AnalysisSummary {
    pub total_packages: usize,
    pub esm_supported: usize,
    pub cjs_supported: usize,
    pub typescript_supported: usize,
    pub browser_supported: usize,
    pub total_size: u64,
    pub scan_duration: std::time::Duration,
    pub errors_encountered: usize,
}
```

## Error Handling

### Error Recovery Strategy

1. **Graceful Degradation**: Continue processing when encountering non-critical errors
2. **Context Preservation**: Maintain error context with file paths and operation details
3. **User-Friendly Messages**: Convert technical errors into actionable user messages
4. **Logging Levels**: Support different verbosity levels for error reporting

### Error Categories

```rust
pub enum ErrorSeverity {
    Warning,  // Log and continue (permission denied, corrupted file)
    Error,    // Log and skip current item (invalid JSON, missing file)
    Critical, // Stop execution (invalid configuration, system error)
}
```

## Testing Strategy

### Unit Testing Approach

1. **Parser Testing**: Test package.json parsing with various configurations
2. **Analysis Logic**: Test module support detection with edge cases
3. **Error Handling**: Test error conditions and recovery
4. **Configuration**: Test config file parsing and merging

### Integration Testing

1. **CLI Interface**: Test command-line argument parsing and execution
2. **File System**: Test directory walking with various structures
3. **Output Formats**: Test all output format implementations
4. **Performance**: Test with large directory structures

### Test Data Strategy

```
tests/
├── fixtures/
│   ├── packages/           # Sample package.json files
│   │   ├── esm-only/
│   │   ├── cjs-only/
│   │   ├── dual-mode/
│   │   └── complex-exports/
│   ├── configs/           # Sample configuration files
│   └── projects/          # Complete project structures
├── unit/
│   ├── parser_tests.rs
│   ├── analyzer_tests.rs
│   └── config_tests.rs
└── integration/
    ├── cli_tests.rs
    ├── walker_tests.rs
    └── output_tests.rs
```

## Performance Considerations

### Optimization Strategies

1. **Parallel Processing**: Use `rayon` for concurrent directory traversal
2. **Lazy Evaluation**: Only calculate expensive metrics when requested
3. **Caching**: Cache parsed package.json results for repeated analysis
4. **Memory Management**: Stream processing for large result sets

### Scalability Targets

- Handle projects with 10,000+ packages
- Complete analysis within 30 seconds for typical projects
- Memory usage under 100MB for large projects
- Graceful handling of deep directory structures (1000+ levels)

## Dependencies

### New Dependencies to Add

```toml
[dependencies]
# Existing
serde_json = "1.0"
ansi_term = "0.12"

# New additions
clap = { version = "4.0", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
thiserror = "1.0"
rayon = "1.7"
indicatif = "0.17"
walkdir = "2.3"
glob = "0.3"
csv = "1.2"
```

### Rationale for Dependencies

- `clap`: Modern CLI argument parsing with derive macros
- `serde`/`toml`: Configuration file parsing
- `thiserror`: Ergonomic error handling
- `rayon`: Data parallelism for performance
- `indicatif`: Progress bars and spinners
- `walkdir`: Robust directory traversal
- `glob`: Pattern matching for exclusions
- `csv`: CSV output format support

## Migration Strategy

### Phase 1: Core Refactoring

1. Extract existing logic into modules
2. Implement error handling with `Result` types
3. Add basic CLI argument parsing
4. Maintain existing functionality

### Phase 2: Enhanced Features

1. Add configuration file support
2. Implement multiple output formats
3. Add comprehensive package analysis
4. Implement caching

### Phase 3: Performance & UX

1. Add parallel processing
2. Implement progress reporting
3. Add comprehensive testing
4. Performance optimization

### Backward Compatibility

- Maintain existing default behavior when run without arguments
- Preserve existing output format as default
- Ensure existing scripts continue to work

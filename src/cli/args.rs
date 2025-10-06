//! Command-line argument parsing

use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Walker - Node.js package analyzer for module system support detection
#[derive(Parser, Debug)]
#[command(name = "walker")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Analyze Node.js packages for ESM/CommonJS module system support")]
#[command(long_about = "Walker is a tool for analyzing Node.js packages to detect their module system support (ESM/CommonJS). \
It scans directory structures to identify packages and provides detailed information about their configuration, \
including module type, TypeScript support, browser compatibility, and dependency analysis.")]
#[command(after_help = "EXAMPLES:

Basic Usage:
    # Scan the current directory
    walker

    # Scan a specific directory
    walker --path ./my-project

    # Exclude specific directories (can specify multiple patterns)
    walker --exclude node_modules --exclude .git

    # Limit directory traversal depth
    walker --max-depth 3

Output Options:
    # Output in JSON format
    walker --output json

    # Output in CSV format for spreadsheet analysis
    walker --output csv

    # Save results to a file
    walker --output-file results.json

    # Disable colored output
    walker --no-colors

Performance Options:
    # Skip package size calculation for faster scanning
    walker --no-size

    # Disable parallel processing
    walker --no-parallel

    # Disable result caching
    walker --no-cache

    # Disable progress bars
    walker --no-progress

Dependency Analysis:
    # Exclude development dependencies from analysis
    walker --no-dev-deps

    # Exclude peer dependencies from analysis
    walker --no-peer-deps

    # Exclude optional dependencies from analysis
    walker --no-optional-deps

Configuration:
    # Use a specific configuration file
    walker --config ./walker-config.toml

    # Create a default configuration file
    walker --init

Verbosity:
    # Quiet mode with minimal output
    walker --quiet

    # Verbose mode with detailed information
    walker --verbose

Common Workflows:
    # Quick scan of a project (fastest performance)
    walker --path ./my-project --no-size --exclude node_modules

    # Detailed analysis with all information
    walker --path ./my-project --verbose

    # Generate a CSV report for spreadsheet analysis
    walker --path ./my-project --output csv --output-file report.csv

    # Analyze only production dependencies
    walker --path ./my-project --no-dev-deps --no-peer-deps --no-optional-deps
")]
pub struct Args {
    /// Target directory to scan
    #[arg(short, long, value_name = "PATH", help = "Directory to scan for Node.js packages (defaults to current directory if not specified)")]
    pub path: Option<PathBuf>,

    /// Exclude directories matching these glob patterns
    #[arg(short, long, value_name = "PATTERN", help = "Glob patterns for directories to exclude (can be specified multiple times, e.g., --exclude node_modules --exclude .git)")]
    pub exclude: Vec<String>,

    /// Maximum depth for directory traversal
    #[arg(long, value_name = "DEPTH", help = "Maximum directory depth to traverse (e.g., 3 will scan up to 3 levels deep from the starting directory)")]
    pub max_depth: Option<usize>,

    /// Output format (text, json, csv)
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text, help = "Output format for results: 'text' for human-readable output, 'json' for machine processing, 'csv' for spreadsheet analysis")]
    pub output: OutputFormat,

    /// Output file path (stdout if not specified)
    #[arg(long, value_name = "FILE", help = "File to write output to (uses stdout if not specified, e.g., --output-file ./results.json)")]
    pub output_file: Option<PathBuf>,

    /// Suppress non-essential output
    #[arg(short, long, help = "Suppress non-essential output (only show results, no progress or summary information)")]
    pub quiet: bool,

    /// Show detailed progress and debug information
    #[arg(short, long, help = "Show detailed progress and debug information (includes package processing details and configuration information)")]
    pub verbose: bool,

    /// Skip size calculation for faster scanning
    #[arg(long, help = "Skip package size calculation for faster scanning (significantly improves performance for large codebases)")]
    pub no_size: bool,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE", help = "Path to configuration file (defaults to .walker.toml in current directory if not specified)")]
    pub config: Option<PathBuf>,

    /// Disable parallel processing
    #[arg(long, help = "Disable parallel processing (uses single-threaded mode, may be slower but uses less memory)")]
    pub no_parallel: bool,

    /// Disable result caching
    #[arg(long, help = "Disable result caching (forces re-analysis of all packages, even if previously analyzed)")]
    pub no_cache: bool,
    
    /// Follow symbolic links during directory traversal
    #[arg(long, help = "Follow symbolic links during directory traversal (may cause infinite loops or duplicate analysis if links form cycles)")]
    pub follow_links: bool,
    
    /// Exclude development dependencies from analysis
    #[arg(long, help = "Exclude development dependencies from analysis (only analyze production dependencies)")]
    pub no_dev_deps: bool,
    
    /// Exclude peer dependencies from analysis
    #[arg(long, help = "Exclude peer dependencies from analysis (only analyze direct and production dependencies)")]
    pub no_peer_deps: bool,
    
    /// Exclude optional dependencies from analysis
    #[arg(long, help = "Exclude optional dependencies from analysis (only analyze required dependencies)")]
    pub no_optional_deps: bool,
    
    /// Disable colored output
    #[arg(long, help = "Disable colored output (useful for terminals that don't support ANSI colors or for piping output)")]
    pub no_colors: bool,
    
    /// Custom cache directory path
    #[arg(long, value_name = "DIR", help = "Custom cache directory path (defaults to system temp directory if not specified)")]
    pub cache_dir: Option<PathBuf>,
    
    /// Disable progress bars
    #[arg(long, help = "Disable progress bars (useful for CI environments or when redirecting output)")]
    pub no_progress: bool,
    
    /// Enable streaming mode for large result sets
    #[arg(long, help = "Enable streaming mode for large result sets (reduces memory usage)")]
    pub stream_results: bool,
    
    /// Set batch size for streaming mode
    #[arg(long, value_name = "SIZE", help = "Set batch size for streaming mode (default: 100)")]
    pub batch_size: Option<usize>,
    
    /// Set memory limit in MB before switching to disk-based storage
    #[arg(long, value_name = "MB", help = "Set memory limit in MB before switching to disk-based storage (default: 512)")]
    pub memory_limit: Option<usize>,
    
    /// Initialize a default configuration file
    #[arg(long, help = "Create a default configuration file (.walker.toml) in the current directory")]
    pub init: bool,

    /// Enable AST-based analysis for dependency graphs and dead code detection
    #[arg(long, help = "Enable AST-based analysis to build dependency graphs and detect dead code (slower but more accurate)")]
    pub ast_analysis: bool,

    /// Follow dynamic imports in AST analysis
    #[arg(long, help = "Include dynamic imports when building dependency graphs (requires --ast-analysis)")]
    pub follow_dynamic_imports: bool,

    /// Include node_modules in AST analysis
    #[arg(long, help = "Include node_modules packages in dependency graph analysis (requires --ast-analysis, may be slow)")]
    pub include_node_modules_ast: bool,

    /// Output dependency graph in DOT format
    #[arg(long, value_name = "FILE", help = "Export dependency graph to DOT format file for visualization (requires --ast-analysis)")]
    pub dependency_graph_output: Option<PathBuf>,
}

/// Output format options
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for programmatic consumption
    Json,
    /// CSV output for spreadsheet analysis
    Csv,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
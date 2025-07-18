//! Configuration-related data structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration settings for Walker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Path to scan for packages
    pub scan_path: PathBuf,
    
    /// Patterns to exclude from scanning
    pub exclude_patterns: Vec<String>,
    
    /// Maximum directory depth to traverse
    pub max_depth: Option<usize>,
    
    /// Output format (text, json, csv)
    pub output_format: OutputFormat,
    
    /// Output file path (if not specified, output to stdout)
    pub output_file: Option<PathBuf>,
    
    /// Whether to calculate package sizes
    pub calculate_size: bool,
    
    /// Whether to use parallel processing
    pub parallel: bool,
    
    /// Whether to enable result caching
    pub cache_enabled: bool,
    
    /// Whether to suppress non-essential output
    pub quiet: bool,
    
    /// Whether to show detailed progress and debug information
    pub verbose: bool,
    
    /// Whether to follow symbolic links during directory traversal
    pub follow_links: bool,
    
    /// Whether to include dev dependencies in analysis
    pub include_dev_deps: bool,
    
    /// Whether to include peer dependencies in analysis
    pub include_peer_deps: bool,
    
    /// Whether to include optional dependencies in analysis
    pub include_optional_deps: bool,
    
    /// Whether to use colors in text output
    pub use_colors: bool,
    
    /// Cache directory path
    pub cache_dir: Option<PathBuf>,
    
    /// Whether to show progress bars
    pub show_progress: bool,
    
    /// Whether to use streaming for large result sets
    pub stream_results: bool,
    
    /// Batch size for streaming results
    pub batch_size: usize,
    
    /// Memory limit in MB before switching to streaming mode
    pub memory_limit_mb: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scan_path: PathBuf::from("."),
            exclude_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
            max_depth: None,
            output_format: OutputFormat::Text,
            output_file: None,
            calculate_size: true,
            parallel: true,
            cache_enabled: true,
            quiet: false,
            verbose: false,
            follow_links: false,
            include_dev_deps: true,
            include_peer_deps: true,
            include_optional_deps: true,
            use_colors: true,
            cache_dir: None,
            show_progress: true,
            stream_results: false,
            batch_size: 100,
            memory_limit_mb: 512,
        }
    }
}

/// Supported output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Human-readable text output
    Text,
    /// JSON output for programmatic consumption
    Json,
    /// CSV output for spreadsheet analysis
    Csv,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
        }
    }
}

/// Partial settings for configuration merging
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartialSettings {
    pub scan_path: Option<PathBuf>,
    pub exclude_patterns: Option<Vec<String>>,
    pub max_depth: Option<usize>,
    pub output_format: Option<OutputFormat>,
    pub output_file: Option<PathBuf>,
    pub calculate_size: Option<bool>,
    pub parallel: Option<bool>,
    pub cache_enabled: Option<bool>,
    pub quiet: Option<bool>,
    pub verbose: Option<bool>,
    pub follow_links: Option<bool>,
    pub include_dev_deps: Option<bool>,
    pub include_peer_deps: Option<bool>,
    pub include_optional_deps: Option<bool>,
    pub use_colors: Option<bool>,
    pub cache_dir: Option<PathBuf>,
    pub show_progress: Option<bool>,
    pub stream_results: Option<bool>,
    pub batch_size: Option<usize>,
    pub memory_limit_mb: Option<usize>,
}

impl PartialSettings {
    /// Merge another PartialSettings into this one
    /// Fields from `other` take precedence over existing fields
    pub fn merge_from(&mut self, other: PartialSettings) {
        if other.scan_path.is_some() {
            self.scan_path = other.scan_path;
        }
        if other.exclude_patterns.is_some() {
            self.exclude_patterns = other.exclude_patterns;
        }
        if other.max_depth.is_some() {
            self.max_depth = other.max_depth;
        }
        if other.output_format.is_some() {
            self.output_format = other.output_format;
        }
        if other.output_file.is_some() {
            self.output_file = other.output_file;
        }
        if other.calculate_size.is_some() {
            self.calculate_size = other.calculate_size;
        }
        if other.parallel.is_some() {
            self.parallel = other.parallel;
        }
        if other.cache_enabled.is_some() {
            self.cache_enabled = other.cache_enabled;
        }
        if other.quiet.is_some() {
            self.quiet = other.quiet;
        }
        if other.verbose.is_some() {
            self.verbose = other.verbose;
        }
        if other.follow_links.is_some() {
            self.follow_links = other.follow_links;
        }
        if other.include_dev_deps.is_some() {
            self.include_dev_deps = other.include_dev_deps;
        }
        if other.include_peer_deps.is_some() {
            self.include_peer_deps = other.include_peer_deps;
        }
        if other.include_optional_deps.is_some() {
            self.include_optional_deps = other.include_optional_deps;
        }
        if other.use_colors.is_some() {
            self.use_colors = other.use_colors;
        }
        if other.cache_dir.is_some() {
            self.cache_dir = other.cache_dir;
        }
        if other.show_progress.is_some() {
            self.show_progress = other.show_progress;
        }
        if other.stream_results.is_some() {
            self.stream_results = other.stream_results;
        }
        if other.batch_size.is_some() {
            self.batch_size = other.batch_size;
        }
        if other.memory_limit_mb.is_some() {
            self.memory_limit_mb = other.memory_limit_mb;
        }
    }

    /// Convert partial settings to full settings
    /// Uses defaults for any fields that are None
    pub fn to_settings(&self) -> Settings {
        let mut settings = Settings::default();
        
        if let Some(scan_path) = &self.scan_path {
            settings.scan_path = scan_path.clone();
        }
        if let Some(exclude_patterns) = &self.exclude_patterns {
            settings.exclude_patterns = exclude_patterns.clone();
        }
        if let Some(max_depth) = self.max_depth {
            settings.max_depth = Some(max_depth);
        }
        if let Some(output_format) = &self.output_format {
            settings.output_format = output_format.clone();
        }
        if let Some(output_file) = &self.output_file {
            settings.output_file = Some(output_file.clone());
        }
        if let Some(calculate_size) = self.calculate_size {
            settings.calculate_size = calculate_size;
        }
        if let Some(parallel) = self.parallel {
            settings.parallel = parallel;
        }
        if let Some(cache_enabled) = self.cache_enabled {
            settings.cache_enabled = cache_enabled;
        }
        if let Some(quiet) = self.quiet {
            settings.quiet = quiet;
        }
        if let Some(verbose) = self.verbose {
            settings.verbose = verbose;
        }
        if let Some(follow_links) = self.follow_links {
            settings.follow_links = follow_links;
        }
        if let Some(include_dev_deps) = self.include_dev_deps {
            settings.include_dev_deps = include_dev_deps;
        }
        if let Some(include_peer_deps) = self.include_peer_deps {
            settings.include_peer_deps = include_peer_deps;
        }
        if let Some(include_optional_deps) = self.include_optional_deps {
            settings.include_optional_deps = include_optional_deps;
        }
        if let Some(use_colors) = self.use_colors {
            settings.use_colors = use_colors;
        }
        if let Some(cache_dir) = &self.cache_dir {
            settings.cache_dir = Some(cache_dir.clone());
        }
        if let Some(show_progress) = self.show_progress {
            settings.show_progress = show_progress;
        }
        if let Some(stream_results) = self.stream_results {
            settings.stream_results = stream_results;
        }
        if let Some(batch_size) = self.batch_size {
            settings.batch_size = batch_size;
        }
        if let Some(memory_limit_mb) = self.memory_limit_mb {
            settings.memory_limit_mb = memory_limit_mb;
        }
        
        settings
    }
}
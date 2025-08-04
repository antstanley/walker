//! Error types and definitions for Walker
//!
//! This module provides a comprehensive error handling system for the Walker application,
//! including error types, result aliases, and error context utilities.

use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Error severity levels for different error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Warning level errors - operation can continue
    Warning,
    /// Error level - current operation fails but overall process can continue
    Error,
    /// Critical level - process should terminate
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Main error type for Walker operations
#[derive(Debug, Error)]
pub enum WalkerError {
    /// Standard IO errors
    #[error("IO error: {source}")]
    Io {
        #[source]
        source: std::io::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// JSON parsing errors with file context
    #[error("JSON parsing error in {file}: {source}")]
    JsonParse {
        file: PathBuf,
        #[source]
        source: serde_json::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Permission denied errors
    #[error("Permission denied accessing {path}")]
    PermissionDenied {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Invalid path errors
    #[error("Invalid path: {path}")]
    InvalidPath {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// TOML parsing errors
    #[error("TOML parsing error: {source}")]
    TomlParse {
        #[source]
        source: toml::de::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// CSV handling errors
    #[error("CSV error: {source}")]
    Csv {
        #[source]
        source: csv::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Glob pattern errors
    #[error("Glob pattern error: {source}")]
    GlobPattern {
        #[source]
        source: glob::PatternError,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Configuration file not found
    #[error("Configuration file not found at {path}")]
    ConfigNotFound {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Configuration file read errors
    #[error("Error reading configuration file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Configuration file parse errors
    #[error("Error parsing configuration file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Invalid output format
    #[error("Invalid output format: {format}")]
    InvalidOutputFormat {
        format: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Output file write errors
    #[error("Error writing to output file {path}: {source}")]
    OutputWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Stdout write errors
    #[error("Error writing to stdout: {source}")]
    StdoutWrite {
        #[source]
        source: std::io::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Package analysis errors
    #[error("Package analysis error: {message}")]
    PackageAnalysis {
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Directory traversal errors
    #[error("Directory traversal error for {path}: {message}")]
    DirectoryTraversal {
        path: PathBuf,
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Package.json not found
    #[error("package.json not found in {path}")]
    PackageJsonNotFound {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Invalid package.json structure
    #[error("Invalid package.json structure in {path}: {message}")]
    InvalidPackageJson {
        path: PathBuf,
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Maximum depth exceeded
    #[error("Maximum directory depth exceeded at {path}")]
    MaxDepthExceeded {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Parallel execution error
    #[error("Parallel execution error: {message}")]
    ParallelExecution {
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Cache error
    #[error("Cache error: {message}")]
    Cache {
        message: String,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Interrupted operation
    #[error("Operation interrupted")]
    Interrupted {
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// JSON serialization error
    #[error("JSON serialization error: {source}")]
    JsonSerialize {
        #[source]
        source: serde_json::Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// CSV serialization error
    #[error("CSV serialization error: {source}")]
    CsvSerialize {
        #[source]
        source: std::string::FromUtf8Error,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },

    /// Output directory not found
    #[error("Output directory not found: {path}")]
    OutputDirectoryNotFound {
        path: PathBuf,
        #[cfg(not(tarpaulin_include))]
        backtrace: std::backtrace::Backtrace,
    },
}

impl WalkerError {
    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Warning level errors - operation can continue
            WalkerError::PermissionDenied { .. } => ErrorSeverity::Warning,
            WalkerError::JsonParse { .. } => ErrorSeverity::Warning,
            WalkerError::PackageJsonNotFound { .. } => ErrorSeverity::Warning,
            WalkerError::InvalidPackageJson { .. } => ErrorSeverity::Warning,

            // Critical errors - process should terminate
            WalkerError::Config { .. } => ErrorSeverity::Critical,
            WalkerError::ConfigNotFound { .. } => ErrorSeverity::Critical,
            WalkerError::ConfigRead { .. } => ErrorSeverity::Critical,
            WalkerError::ConfigParse { .. } => ErrorSeverity::Critical,
            WalkerError::InvalidOutputFormat { .. } => ErrorSeverity::Critical,
            WalkerError::StdoutWrite { .. } => ErrorSeverity::Critical,
            WalkerError::OutputDirectoryNotFound { .. } => ErrorSeverity::Critical,

            // Regular errors - current operation fails but overall process can continue
            _ => ErrorSeverity::Error,
        }
    }

    /// Check if this is a critical error that should terminate the process
    pub fn is_critical(&self) -> bool {
        self.severity() == ErrorSeverity::Critical
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            WalkerError::PermissionDenied { path, .. } => {
                format!("Cannot access '{}' due to permission denied. Try running with elevated permissions or check file permissions.", path.display())
            }
            WalkerError::JsonParse { file, source, .. } => {
                format!("Invalid JSON in '{}': {}. Please check the file format.", file.display(), source)
            }
            WalkerError::Io { source, .. } => {
                format!("File system error: {}. Check disk space and permissions.", source)
            }
            WalkerError::InvalidPath { path, .. } => {
                format!("Invalid path: '{}'. Please provide a valid directory path.", path.display())
            }
            WalkerError::ConfigNotFound { path, .. } => {
                format!("Configuration file not found at '{}'. Create a config file or use command line options.", path.display())
            }
            WalkerError::MaxDepthExceeded { path, .. } => {
                format!("Maximum directory depth exceeded at '{}'. Use --max-depth option to increase the limit.", path.display())
            }
            WalkerError::PackageJsonNotFound { path, .. } => {
                format!("No package.json found in '{}'. Skipping directory.", path.display())
            }
            WalkerError::InvalidPackageJson { path, message, .. } => {
                format!("Invalid package.json in '{}': {}. Skipping package.", path.display(), message)
            }
            WalkerError::OutputDirectoryNotFound { path, .. } => {
                format!("Output directory '{}' does not exist. Please create the directory or specify a different output path.", path.display())
            }
            // For other errors, use the standard Display implementation
            _ => self.to_string(),
        }
    }

    /// Create an IO error with context
    pub fn io_error(source: std::io::Error) -> Self {
        WalkerError::Io {
            source,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    /// Create a JSON parse error with file context
    pub fn json_parse_error(file: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        WalkerError::JsonParse {
            file: file.into(),
            source,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    /// Create a configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        WalkerError::Config {
            message: message.into(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(path: impl Into<PathBuf>) -> Self {
        WalkerError::PermissionDenied {
            path: path.into(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    /// Create a package analysis error
    pub fn package_analysis_error(message: impl Into<String>) -> Self {
        WalkerError::PackageAnalysis {
            message: message.into(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    /// Create a directory traversal error
    pub fn directory_traversal_error(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        WalkerError::DirectoryTraversal {
            path: path.into(),
            message: message.into(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

// Implement From for common error types
impl From<std::io::Error> for WalkerError {
    fn from(err: std::io::Error) -> Self {
        WalkerError::io_error(err)
    }
}

impl From<toml::de::Error> for WalkerError {
    fn from(err: toml::de::Error) -> Self {
        WalkerError::TomlParse {
            source: err,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl From<csv::Error> for WalkerError {
    fn from(err: csv::Error) -> Self {
        WalkerError::Csv {
            source: err,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl From<glob::PatternError> for WalkerError {
    fn from(err: glob::PatternError) -> Self {
        WalkerError::GlobPattern {
            source: err,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl From<serde_json::Error> for WalkerError {
    fn from(err: serde_json::Error) -> Self {
        WalkerError::JsonSerialize {
            source: err,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

/// Result type alias for Walker operations
pub type Result<T> = std::result::Result<T, WalkerError>;

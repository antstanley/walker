//! Walker - A Node.js package analyzer for module system support detection
//! 
//! This library provides functionality to scan directory structures and analyze
//! JavaScript/Node.js packages for their ESM/CommonJS module system support.

#![feature(error_generic_member_access)]

pub mod cli;
pub mod config;
pub mod core;
pub mod error;
pub mod models;
pub mod output;
pub mod parsers;
pub mod utils;

// Re-export commonly used types
pub use error::{
    ErrorSeverity, OptionExt, Result, ResultExt, WalkerError, 
    handle_error, try_with_recovery
};
pub use models::{
    analysis::{AnalysisResults, AnalysisSummary, PackageAnalysis},
    config::Settings,
    package::{ModuleSupport, PackageDetails},
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
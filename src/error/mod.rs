//! Error handling for the Walker application
//!
//! This module provides a comprehensive error handling system for the Walker application,
//! including error types, result aliases, and error context utilities.

pub mod context;
pub mod tests;
pub mod types;

pub use context::{OptionExt, ResultExt, handle_error, try_with_recovery};
pub use types::{ErrorSeverity, Result, WalkerError};
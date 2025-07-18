//! Error context utilities for Walker
//!
//! This module provides utilities for adding context to errors and handling
//! errors in a consistent way throughout the application.

use std::path::Path;
use crate::error::{Result, WalkerError};

/// Extension trait for Result to add context to errors
pub trait ResultExt<T, E> {
    /// Add context to an error with a custom message
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: std::fmt::Display;

    /// Add file context to an error
    fn with_file_context<P: AsRef<Path>>(self, path: P) -> Result<T>;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: std::error::Error + 'static,
{
    fn with_context<C, F>(self, context: F) -> Result<T>
    where
        F: FnOnce() -> C,
        C: std::fmt::Display,
    {
        self.map_err(|err| {
            WalkerError::PackageAnalysis {
                message: format!("{}: {}", context(), err),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
        })
    }

    fn with_file_context<P: AsRef<Path>>(self, path: P) -> Result<T> {
        self.map_err(|err| {
            if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                if io_err.kind() == std::io::ErrorKind::PermissionDenied {
                    return WalkerError::PermissionDenied {
                        path: path.as_ref().to_path_buf(),
                        #[cfg(not(tarpaulin_include))]
                        backtrace: std::backtrace::Backtrace::capture(),
                    };
                }
            }

            WalkerError::DirectoryTraversal {
                path: path.as_ref().to_path_buf(),
                message: format!("{}", err),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
        })
    }
}

/// Handle an error based on its severity
///
/// - Warning: Log the error and return None
/// - Error: Log the error and return None
/// - Critical: Log the error and return Some(error)
pub fn handle_error(err: WalkerError) -> Option<WalkerError> {
    let severity = err.severity();
    let message = err.user_message();

    match severity {
        crate::error::types::ErrorSeverity::Warning => {
            eprintln!("Warning: {}", message);
            None
        }
        crate::error::types::ErrorSeverity::Error => {
            eprintln!("Error: {}", message);
            None
        }
        crate::error::types::ErrorSeverity::Critical => {
            eprintln!("Critical Error: {}", message);
            Some(err)
        }
    }
}

/// Try to run a function and handle any errors based on their severity
///
/// Returns Ok(T) if the function succeeds, or Err(WalkerError) if a critical error occurs.
/// Non-critical errors are logged but do not cause the function to fail.
pub fn try_with_recovery<T, F>(f: F) -> Result<Option<T>>
where
    F: FnOnce() -> Result<T>,
{
    match f() {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            if let Some(critical_err) = handle_error(err) {
                Err(critical_err)
            } else {
                Ok(None)
            }
        }
    }
}

/// Extension trait for Option to convert to Result with a custom error
pub trait OptionExt<T> {
    /// Convert Option to Result with a custom error message
    fn ok_or_error<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce() -> WalkerError;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_error<F>(self, err_fn: F) -> Result<T>
    where
        F: FnOnce() -> WalkerError,
    {
        self.ok_or_else(err_fn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_with_context() {
        let result: std::result::Result<(), io::Error> = Err(io::Error::new(
            io::ErrorKind::NotFound,
            "file not found",
        ));
        
        let with_context = result.with_context(|| "Failed to read config");
        assert!(with_context.is_err());
        
        if let Err(err) = with_context {
            if let WalkerError::PackageAnalysis { message, .. } = err {
                assert!(message.contains("Failed to read config"));
                assert!(message.contains("file not found"));
            } else {
                panic!("Expected PackageAnalysis error");
            }
        }
    }

    #[test]
    fn test_with_file_context() {
        let result: std::result::Result<(), io::Error> = Err(io::Error::new(
            io::ErrorKind::NotFound,
            "file not found",
        ));
        
        let with_context = result.with_file_context("test/path");
        assert!(with_context.is_err());
        
        if let Err(err) = with_context {
            if let WalkerError::DirectoryTraversal { path, .. } = err {
                assert_eq!(path.to_string_lossy(), "test/path");
            } else {
                panic!("Expected DirectoryTraversal error");
            }
        }
    }

    #[test]
    fn test_with_file_context_permission_denied() {
        let result: std::result::Result<(), io::Error> = Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied",
        ));
        
        let with_context = result.with_file_context("test/path");
        assert!(with_context.is_err());
        
        if let Err(err) = with_context {
            if let WalkerError::PermissionDenied { path, .. } = err {
                assert_eq!(path.to_string_lossy(), "test/path");
            } else {
                panic!("Expected PermissionDenied error");
            }
        }
    }

    #[test]
    fn test_option_ext() {
        let none: Option<i32> = None;
        let result = none.ok_or_error(|| WalkerError::config_error("Missing value"));
        
        assert!(result.is_err());
        if let Err(WalkerError::Config { message, .. }) = result {
            assert_eq!(message, "Missing value");
        } else {
            panic!("Expected Config error");
        }
        
        let some = Some(42);
        let result = some.ok_or_error(|| WalkerError::config_error("Missing value"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
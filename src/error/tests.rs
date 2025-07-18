//! Tests for error handling system

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn test_error_severity() {
        // Test warning level errors
        assert_eq!(
            WalkerError::PermissionDenied {
                path: PathBuf::from("test"),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
            .severity(),
            ErrorSeverity::Warning
        );

        // Test error level errors
        assert_eq!(
            WalkerError::Io {
                source: io::Error::new(io::ErrorKind::NotFound, "not found"),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
            .severity(),
            ErrorSeverity::Error
        );

        // Test critical level errors
        assert_eq!(
            WalkerError::Config {
                message: "Invalid config".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
            .severity(),
            ErrorSeverity::Critical
        );
    }

    #[test]
    fn test_is_critical() {
        assert!(
            WalkerError::Config {
                message: "Invalid config".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
            .is_critical()
        );

        assert!(
            !WalkerError::Io {
                source: io::Error::new(io::ErrorKind::NotFound, "not found"),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }
            .is_critical()
        );
    }

    #[test]
    fn test_user_message() {
        let err = WalkerError::PermissionDenied {
            path: PathBuf::from("/test/path"),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        let msg = err.user_message();
        assert!(msg.contains("/test/path"));
        assert!(msg.contains("permission denied"));

        let err = WalkerError::JsonParse {
            file: PathBuf::from("/test/package.json"),
            source: serde_json::Error::syntax(serde_json::error::ErrorCode::ExpectedSomeValue, 0, 0),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        let msg = err.user_message();
        assert!(msg.contains("/test/package.json"));
        assert!(msg.contains("Invalid JSON"));
    }

    #[test]
    fn test_error_factory_methods() {
        let io_err = WalkerError::io_error(io::Error::new(io::ErrorKind::NotFound, "not found"));
        if let WalkerError::Io { source, .. } = io_err {
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        } else {
            panic!("Expected Io error");
        }

        let json_err = WalkerError::json_parse_error(
            "/test/package.json",
            serde_json::Error::syntax(serde_json::error::ErrorCode::ExpectedSomeValue, 0, 0),
        );
        if let WalkerError::JsonParse { file, .. } = json_err {
            assert_eq!(file, PathBuf::from("/test/package.json"));
        } else {
            panic!("Expected JsonParse error");
        }

        let config_err = WalkerError::config_error("Invalid config");
        if let WalkerError::Config { message, .. } = config_err {
            assert_eq!(message, "Invalid config");
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_handle_error() {
        use super::super::context::handle_error;

        // Warning level error should return None
        let warning_err = WalkerError::PermissionDenied {
            path: PathBuf::from("/test/path"),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert!(handle_error(warning_err).is_none());

        // Error level error should return None
        let error_err = WalkerError::Io {
            source: io::Error::new(io::ErrorKind::NotFound, "not found"),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert!(handle_error(error_err).is_none());

        // Critical level error should return Some(err)
        let critical_err = WalkerError::Config {
            message: "Invalid config".to_string(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        };
        assert!(handle_error(critical_err).is_some());
    }

    #[test]
    fn test_try_with_recovery() {
        use super::super::context::try_with_recovery;

        // Successful operation should return Ok(Some(value))
        let result = try_with_recovery(|| Ok::<_, WalkerError>(42));
        assert_eq!(result, Ok(Some(42)));

        // Non-critical error should return Ok(None)
        let result = try_with_recovery(|| {
            Err::<i32, _>(WalkerError::Io {
                source: io::Error::new(io::ErrorKind::NotFound, "not found"),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            })
        });
        assert_eq!(result, Ok(None));

        // Critical error should return Err(err)
        let result = try_with_recovery(|| {
            Err::<i32, _>(WalkerError::Config {
                message: "Invalid config".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            })
        });
        assert!(result.is_err());
    }
}
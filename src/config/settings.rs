//! Configuration settings structures and validation

use std::path::Path;
use crate::models::config::{PartialSettings, Settings};
use crate::error::{Result, WalkerError, ResultExt};

/// Settings validator for ensuring configuration is valid
pub struct SettingsValidator;

impl SettingsValidator {
    /// Validate settings and return errors if invalid
    pub fn validate(settings: &Settings) -> Result<()> {
        // Validate scan path exists
        if !settings.scan_path.exists() {
            return Err(WalkerError::InvalidPath {
                path: settings.scan_path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Validate exclude patterns
        for pattern in &settings.exclude_patterns {
            glob::Pattern::new(pattern)
                .with_context(|| format!("Invalid exclude pattern: {}", pattern))?;
        }

        // Validate max depth is reasonable
        if let Some(depth) = settings.max_depth {
            if depth == 0 {
                return Err(WalkerError::Config {
                    message: "Max depth must be at least 1".to_string(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
        }

        // Validate output file path is writable if specified
        if let Some(path) = &settings.output_file {
            Self::validate_output_path(path)?;
        }

        Ok(())
    }

    /// Validate that an output path is writable
    fn validate_output_path(path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(WalkerError::InvalidPath {
                    path: parent.to_path_buf(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }

            // Try to check if the directory is writable
            // This is a best-effort check and may not be accurate in all cases
            match std::fs::metadata(parent) {
                Ok(metadata) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let mode = metadata.permissions().mode();
                        // Check if the directory is writable by the current user
                        if mode & 0o200 == 0 {
                            return Err(WalkerError::PermissionDenied {
                                path: parent.to_path_buf(),
                                #[cfg(not(tarpaulin_include))]
                                backtrace: std::backtrace::Backtrace::capture(),
                            });
                        }
                    }
                }
                Err(e) => {
                    return Err(WalkerError::io_error(e));
                }
            }
        }

        Ok(())
    }
}

/// Configuration builder for creating settings from various sources
pub struct ConfigBuilder {
    partial: PartialSettings,
}

impl ConfigBuilder {
    /// Create a new configuration builder
    pub fn new() -> Self {
        Self {
            partial: PartialSettings::default(),
        }
    }

    /// Merge with another partial settings
    pub fn merge(mut self, other: PartialSettings) -> Self {
        // Apply all fields from other that are Some
        self.partial.merge_from(other);
        self
    }

    /// Build final settings with validation
    pub fn build(self) -> Result<Settings> {
        // Convert partial settings to full settings
        let settings = self.partial.to_settings();
        
        // Validate settings
        SettingsValidator::validate(&settings)?;

        Ok(settings)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
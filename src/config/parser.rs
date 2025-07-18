//! Configuration file parsing utilities

use std::fs;
use std::path::{Path, PathBuf};
use toml;

use crate::error::{Result, WalkerError, ResultExt};
use crate::models::config::{PartialSettings, Settings};

/// Parse a TOML configuration file into PartialSettings
pub fn parse_config_file<P: AsRef<Path>>(path: P) -> Result<PartialSettings> {
    let path = path.as_ref();
    
    if !path.exists() {
        return Err(WalkerError::ConfigNotFound {
            path: path.to_path_buf(),
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        });
    }

    let content = fs::read_to_string(path)
        .map_err(|e| WalkerError::ConfigRead {
            path: path.to_path_buf(),
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

    parse_config_content(&content, path)
}

/// Parse TOML configuration content into PartialSettings
pub fn parse_config_content<P: AsRef<Path>>(content: &str, path: P) -> Result<PartialSettings> {
    let path = path.as_ref();
    
    // Parse the TOML content
    let settings: PartialSettings = toml::from_str(content)
        .map_err(|e| WalkerError::ConfigParse {
            path: path.to_path_buf(),
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
    
    // Validate the parsed settings
    validate_partial_settings(&settings, path)?;
    
    Ok(settings)
}

/// Validate partial settings for obvious errors
pub fn validate_partial_settings<P: AsRef<Path>>(settings: &PartialSettings, path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Validate scan path if specified
    if let Some(scan_path) = &settings.scan_path {
        if scan_path.as_os_str().is_empty() {
            return Err(WalkerError::Config {
                message: format!("Invalid empty scan_path in config file: {}", path.display()),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
    }
    
    // Validate exclude patterns if specified
    if let Some(patterns) = &settings.exclude_patterns {
        for pattern in patterns {
            if pattern.is_empty() {
                return Err(WalkerError::Config {
                    message: format!("Empty exclude pattern in config file: {}", path.display()),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            
            // Try to compile the pattern to check validity
            glob::Pattern::new(pattern)
                .map_err(|e| WalkerError::Config {
                    message: format!("Invalid exclude pattern '{}' in config file: {}: {}", 
                        pattern, path.display(), e),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?;
        }
    }
    
    // Validate max_depth if specified
    if let Some(depth) = settings.max_depth {
        if depth == 0 {
            return Err(WalkerError::Config {
                message: format!("Invalid max_depth 0 in config file: {}. Must be at least 1.", path.display()),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
    }
    
    // Validate output file if specified
    if let Some(output_file) = &settings.output_file {
        if output_file.as_os_str().is_empty() {
            return Err(WalkerError::Config {
                message: format!("Invalid empty output_file in config file: {}", path.display()),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
    }
    
    // Validate cache_dir if specified
    if let Some(cache_dir) = &settings.cache_dir {
        if cache_dir.as_os_str().is_empty() {
            return Err(WalkerError::Config {
                message: format!("Invalid empty cache_dir in config file: {}", path.display()),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
    }
    
    Ok(())
}

/// Find and load configuration from default locations
pub fn find_default_config() -> Result<Option<PartialSettings>> {
    // Check current directory first
    let current_dir_config = PathBuf::from(".walker.toml");
    if current_dir_config.exists() {
        return Ok(Some(parse_config_file(current_dir_config)?));
    }
    
    // Check user home directory next
    if let Some(home_dir) = dirs::home_dir() {
        let home_config = home_dir.join(".walker.toml");
        if home_config.exists() {
            return Ok(Some(parse_config_file(home_config)?));
        }
    }
    
    // Check XDG config directory if available
    if let Some(config_dir) = dirs::config_dir() {
        let xdg_config = config_dir.join("walker").join("config.toml");
        if xdg_config.exists() {
            return Ok(Some(parse_config_file(xdg_config)?));
        }
    }
    
    // No config file found
    Ok(None)
}

/// Create a default configuration file at the specified path
pub fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| WalkerError::io_error(e))?;
        }
    }
    
    // Use the embedded default configuration template
    let default_config = include_str!("default_config.toml");
    
    // Write to file
    fs::write(path, default_config)
        .map_err(|e| WalkerError::io_error(e))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_parse_config_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config_content = r#"
            scan_path = "/test/path"
            exclude_patterns = ["node_modules", "dist"]
            max_depth = 5
            output_format = "json"
            calculate_size = false
            parallel = false
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let settings = parse_config_file(&config_path).unwrap();
        
        assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
        assert_eq!(settings.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
        assert_eq!(settings.max_depth, Some(5));
    }
    
    #[test]
    fn test_parse_config_content() {
        let config_content = r#"
            scan_path = "/test/path"
            exclude_patterns = ["node_modules", "dist"]
            max_depth = 5
            output_format = "json"
            calculate_size = false
            parallel = false
        "#;
        
        let settings = parse_config_content(config_content, "virtual_path.toml").unwrap();
        
        assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
        assert_eq!(settings.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
        assert_eq!(settings.max_depth, Some(5));
    }
    
    #[test]
    fn test_validate_partial_settings() {
        // Valid settings
        let valid_settings = PartialSettings {
            scan_path: Some(PathBuf::from("/test/path")),
            exclude_patterns: Some(vec!["node_modules".to_string()]),
            max_depth: Some(5),
            ..Default::default()
        };
        
        assert!(validate_partial_settings(&valid_settings, "test.toml").is_ok());
        
        // Invalid max_depth
        let invalid_depth = PartialSettings {
            max_depth: Some(0),
            ..Default::default()
        };
        
        assert!(validate_partial_settings(&invalid_depth, "test.toml").is_err());
        
        // Invalid exclude pattern
        let invalid_pattern = PartialSettings {
            exclude_patterns: Some(vec!["".to_string()]),
            ..Default::default()
        };
        
        assert!(validate_partial_settings(&invalid_pattern, "test.toml").is_err());
    }
    
    #[test]
    fn test_create_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("default_config.toml");
        
        assert!(!config_path.exists());
        
        create_default_config(&config_path).unwrap();
        
        assert!(config_path.exists());
        
        // Parse the created file to ensure it's valid
        let settings = parse_config_file(&config_path).unwrap();
        
        // Check that it contains default values
        assert!(settings.scan_path.is_none()); // Default values aren't included in PartialSettings
    }
    
    #[test]
    fn test_find_default_config() {
        let temp_dir = tempdir().unwrap();
        let current_dir = std::env::current_dir().unwrap();
        
        // Temporarily change to the temp directory
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create a config file in the current directory
        let config_path = PathBuf::from(".walker.toml");
        let config_content = r#"
            scan_path = "/test/path"
            max_depth = 5
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Find the config
        let settings = find_default_config().unwrap();
        
        // Change back to the original directory
        std::env::set_current_dir(current_dir).unwrap();
        
        assert!(settings.is_some());
        let settings = settings.unwrap();
        assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
        assert_eq!(settings.max_depth, Some(5));
    }
}
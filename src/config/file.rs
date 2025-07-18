//! Configuration file handling

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::config::PartialSettings;
use super::{ConfigSource, parser};

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = ".walker.toml";

/// Configuration file source
pub struct FileConfig {
    path: PathBuf,
    name: String,
    priority: u8,
}

impl FileConfig {
    /// Create a new file configuration source with the default path
    pub fn new() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_CONFIG_FILE),
            name: "default config file".to_string(),
            priority: 20, // Higher priority than environment variables but lower than CLI
        }
    }

    /// Create a new file configuration source with a custom path
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            name: format!("config file ({})", path.as_ref().display()),
            priority: 20,
        }
    }

    /// Set the priority for this configuration source
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set a custom name for this configuration source
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    /// Get the path of this configuration file
    pub fn path(&self) -> &Path {
        &self.path
    }
    
    /// Create a default configuration file at this location
    pub fn create_default(&self) -> Result<()> {
        parser::create_default_config(&self.path)
    }
}

impl ConfigSource for FileConfig {
    fn load(&self) -> Result<PartialSettings> {
        if !self.is_available() {
            return Err(crate::error::WalkerError::ConfigNotFound {
                path: self.path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        parser::parse_config_file(&self.path)
    }
    
    fn is_available(&self) -> bool {
        self.path.exists() && self.path.is_file()
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn priority(&self) -> u8 {
        self.priority
    }
}

impl Default for FileConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment variable configuration source
pub struct EnvConfig {
    prefix: String,
    name: String,
    priority: u8,
}

impl EnvConfig {
    /// Create a new environment variable configuration source
    pub fn new(prefix: impl Into<String>) -> Self {
        let prefix = prefix.into();
        Self {
            name: format!("{} environment variables", &prefix),
            prefix,
            priority: 10, // Lower priority than file config
        }
    }
    
    /// Set the priority for this configuration source
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl ConfigSource for EnvConfig {
    fn load(&self) -> Result<PartialSettings> {
        let mut settings = PartialSettings::default();
        
        // Example of how to load from environment variables
        // This would be expanded to handle all configuration options
        if let Ok(path) = std::env::var(format!("{}_SCAN_PATH", self.prefix)) {
            settings.scan_path = Some(PathBuf::from(path));
        }
        
        if let Ok(exclude) = std::env::var(format!("{}_EXCLUDE", self.prefix)) {
            settings.exclude_patterns = Some(
                exclude.split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            );
        }
        
        if let Ok(max_depth) = std::env::var(format!("{}_MAX_DEPTH", self.prefix)) {
            if let Ok(depth) = max_depth.parse() {
                settings.max_depth = Some(depth);
            }
        }
        
        if let Ok(format) = std::env::var(format!("{}_OUTPUT_FORMAT", self.prefix)) {
            if let Ok(output_format) = format.parse() {
                settings.output_format = Some(output_format);
            }
        }
        
        // Add more environment variables as needed
        
        Ok(settings)
    }
    
    fn is_available(&self) -> bool {
        // Check if any relevant environment variables exist
        std::env::var(format!("{}_SCAN_PATH", self.prefix)).is_ok() ||
        std::env::var(format!("{}_EXCLUDE", self.prefix)).is_ok() ||
        std::env::var(format!("{}_MAX_DEPTH", self.prefix)).is_ok() ||
        std::env::var(format!("{}_OUTPUT_FORMAT", self.prefix)).is_ok()
        // Add more checks as needed
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn priority(&self) -> u8 {
        self.priority
    }
}
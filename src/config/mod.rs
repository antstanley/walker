//! Configuration management

pub mod cli;
pub mod file;
pub mod parser;
pub mod settings;
#[cfg(test)]
pub mod tests;

use crate::error::Result;
use crate::models::config::{PartialSettings, Settings};

pub use file::{FileConfig, EnvConfig};
pub use cli::{CliConfig, CliArgs};
pub use settings::SettingsValidator;
pub use parser::{parse_config_file, parse_config_content, find_default_config, create_default_config};

/// Trait for configuration sources
pub trait ConfigSource {
    /// Load configuration from this source
    fn load(&self) -> Result<PartialSettings>;
    
    /// Check if this configuration source is available
    fn is_available(&self) -> bool;
    
    /// Get the name of this configuration source for logging
    fn name(&self) -> &str;
    
    /// Get the priority of this source (higher numbers take precedence)
    fn priority(&self) -> u8 {
        10 // Default priority
    }
}

/// Configuration builder for merging multiple sources
pub struct ConfigBuilder {
    partial: PartialSettings,
}

impl ConfigBuilder {
    /// Create a new configuration builder with defaults
    pub fn new() -> Self {
        Self {
            partial: PartialSettings::default(),
        }
    }

    /// Merge settings from a partial configuration
    pub fn merge(mut self, partial: PartialSettings) -> Self {
        self.partial.merge_from(partial);
        self
    }
    
    /// Load and merge settings from a configuration source
    pub fn load_from<S: ConfigSource>(self, source: &S) -> Result<Self> {
        if source.is_available() {
            match source.load() {
                Ok(partial) => Ok(self.merge(partial)),
                Err(e) => Err(e),
            }
        } else {
            Ok(self)
        }
    }
    
    /// Try to load from a source, ignoring if not available
    pub fn try_load_from<S: ConfigSource>(self, source: &S) -> Self {
        if source.is_available() {
            match source.load() {
                Ok(partial) => self.merge(partial),
                Err(_) => self,
            }
        } else {
            self
        }
    }

    /// Add configuration from a file
    pub fn add_config_file(self, path: &std::path::Path) -> Result<Self> {
        let file_config = FileConfig::with_path(path.to_path_buf());
        self.load_from(&file_config)
    }
    
    /// Try to add configuration from the default config file
    pub fn try_add_default_config_file(self) -> Self {
        if let Ok(Some(default_config)) = parser::find_default_config() {
            self.merge(default_config)
        } else {
            self
        }
    }
    
    /// Build the final settings with validation
    pub fn build(self) -> Result<Settings> {
        // Convert partial settings to full settings
        let settings = self.partial.to_settings();
        
        // Validate settings
        settings::SettingsValidator::validate(&settings)?;
        
        Ok(settings)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Load configuration from multiple sources with proper precedence handling
pub fn load_config(cli_args: CliArgs) -> Result<Settings> {
    // Create CLI config source (highest priority)
    let cli_config = CliConfig::new(cli_args.clone());
    
    // Start with an empty builder
    let mut builder = ConfigBuilder::new();
    
    // Try to load from default locations if no config file specified
    if cli_args.config.is_none() {
        if let Ok(Some(default_config)) = parser::find_default_config() {
            builder = builder.merge(default_config);
        }
    } else {
        // Load from specified config file
        let file_config = FileConfig::with_path(cli_args.config.unwrap());
        builder = builder.load_from(&file_config)?;
    }
    
    // Try to load from environment variables
    let env_config = EnvConfig::new("WALKER");
    builder = builder.try_load_from(&env_config);
    
    // Load from CLI args (highest priority)
    builder = builder.load_from(&cli_config)?;
    
    // Build and validate the final settings
    builder.build()
}

/// Load configuration with a custom environment variable prefix
pub fn load_config_with_env_prefix(cli_args: CliArgs, env_prefix: &str) -> Result<Settings> {
    // Create CLI config source (highest priority)
    let cli_config = CliConfig::new(cli_args.clone());
    
    // Start with an empty builder
    let mut builder = ConfigBuilder::new();
    
    // Try to load from default locations if no config file specified
    if cli_args.config.is_none() {
        if let Ok(Some(default_config)) = parser::find_default_config() {
            builder = builder.merge(default_config);
        }
    } else {
        // Load from specified config file
        let file_config = FileConfig::with_path(cli_args.config.unwrap());
        builder = builder.load_from(&file_config)?;
    }
    
    // Try to load from environment variables
    let env_config = EnvConfig::new(env_prefix);
    builder = builder.try_load_from(&env_config);
    
    // Load from CLI args (highest priority)
    builder = builder.load_from(&cli_config)?;
    
    // Build and validate the final settings
    builder.build()
}
//! Tests for configuration system

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::models::config::{PartialSettings, Settings, OutputFormat};
    use std::path::PathBuf;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_partial_settings_merge() {
        let mut base = PartialSettings::default();
        base.scan_path = Some(PathBuf::from("/base/path"));
        base.exclude_patterns = Some(vec!["base_exclude".to_string()]);
        
        let override_settings = PartialSettings {
            scan_path: Some(PathBuf::from("/override/path")),
            max_depth: Some(5),
            ..Default::default()
        };
        
        base.merge_from(override_settings);
        
        assert_eq!(base.scan_path, Some(PathBuf::from("/override/path")));
        assert_eq!(base.exclude_patterns, Some(vec!["base_exclude".to_string()]));
        assert_eq!(base.max_depth, Some(5));
    }
    
    #[test]
    fn test_partial_settings_to_settings() {
        let partial = PartialSettings {
            scan_path: Some(PathBuf::from("/custom/path")),
            exclude_patterns: Some(vec!["custom_exclude".to_string()]),
            max_depth: Some(3),
            output_format: Some(OutputFormat::Json),
            calculate_size: Some(false),
            ..Default::default()
        };
        
        let settings = partial.to_settings();
        
        // Check that specified values are used
        assert_eq!(settings.scan_path, PathBuf::from("/custom/path"));
        assert_eq!(settings.exclude_patterns, vec!["custom_exclude".to_string()]);
        assert_eq!(settings.max_depth, Some(3));
        assert!(matches!(settings.output_format, OutputFormat::Json));
        assert_eq!(settings.calculate_size, false);
        
        // Check that default values are used for unspecified fields
        assert_eq!(settings.parallel, true); // Default value
    }
    
    #[test]
    fn test_config_builder() {
        let builder = ConfigBuilder::new();
        
        let partial1 = PartialSettings {
            scan_path: Some(PathBuf::from("/path1")),
            exclude_patterns: Some(vec!["exclude1".to_string()]),
            ..Default::default()
        };
        
        let partial2 = PartialSettings {
            scan_path: Some(PathBuf::from("/path2")),
            max_depth: Some(5),
            ..Default::default()
        };
        
        let settings = builder
            .merge(partial1)
            .merge(partial2)
            .build()
            .unwrap();
        
        // Last merge wins for scan_path
        assert_eq!(settings.scan_path, PathBuf::from("/path2"));
        // First merge is preserved for exclude_patterns
        assert_eq!(settings.exclude_patterns, vec!["exclude1".to_string()]);
        // Second merge is applied for max_depth
        assert_eq!(settings.max_depth, Some(5));
    }
    
    #[test]
    fn test_file_config_source() {
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
        
        let file_config = file::FileConfig::with_path(&config_path);
        assert!(file_config.is_available());
        assert_eq!(file_config.priority(), 20);
        
        let partial = file_config.load().unwrap();
        
        assert_eq!(partial.scan_path, Some(PathBuf::from("/test/path")));
        assert_eq!(partial.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
        assert_eq!(partial.max_depth, Some(5));
        assert!(matches!(partial.output_format, Some(OutputFormat::Json)));
        assert_eq!(partial.calculate_size, Some(false));
        assert_eq!(partial.parallel, Some(false));
    }
    
    #[test]
    fn test_file_config_not_found() {
        let file_config = file::FileConfig::with_path("/nonexistent/path/config.toml");
        assert!(!file_config.is_available());
        assert!(file_config.load().is_err());
    }
    
    #[test]
    fn test_env_config_source() {
        // Set some environment variables for testing
        std::env::set_var("TEST_SCAN_PATH", "/env/path");
        std::env::set_var("TEST_EXCLUDE", "node_modules,dist,build");
        std::env::set_var("TEST_MAX_DEPTH", "10");
        std::env::set_var("TEST_OUTPUT_FORMAT", "csv");
        
        let env_config = file::EnvConfig::new("TEST");
        assert!(env_config.is_available());
        assert_eq!(env_config.priority(), 10);
        
        let partial = env_config.load().unwrap();
        
        assert_eq!(partial.scan_path, Some(PathBuf::from("/env/path")));
        assert_eq!(
            partial.exclude_patterns, 
            Some(vec!["node_modules".to_string(), "dist".to_string(), "build".to_string()])
        );
        assert_eq!(partial.max_depth, Some(10));
        assert!(matches!(partial.output_format, Some(OutputFormat::Csv)));
        
        // Clean up environment variables
        std::env::remove_var("TEST_SCAN_PATH");
        std::env::remove_var("TEST_EXCLUDE");
        std::env::remove_var("TEST_MAX_DEPTH");
        std::env::remove_var("TEST_OUTPUT_FORMAT");
    }
    
    #[test]
    fn test_config_builder_load_from() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config_content = r#"
            scan_path = "/file/path"
            exclude_patterns = ["file_exclude"]
            max_depth = 5
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let file_config = file::FileConfig::with_path(&config_path);
        
        // Set environment variables with lower priority
        std::env::set_var("TEST_SCAN_PATH", "/env/path");
        std::env::set_var("TEST_OUTPUT_FORMAT", "json");
        
        let env_config = file::EnvConfig::new("TEST");
        
        // Create a builder and load from both sources
        let builder = ConfigBuilder::new();
        let settings = builder
            .load_from(&env_config).unwrap()  // Lower priority
            .load_from(&file_config).unwrap() // Higher priority
            .build()
            .unwrap();
        
        // File config should override env config for scan_path
        assert_eq!(settings.scan_path, PathBuf::from("/file/path"));
        // File config's exclude_patterns should be used
        assert_eq!(settings.exclude_patterns, vec!["file_exclude".to_string()]);
        // File config's max_depth should be used
        assert_eq!(settings.max_depth, Some(5));
        // Env config's output_format should be preserved since file doesn't specify it
        assert!(matches!(settings.output_format, OutputFormat::Json));
        
        // Clean up environment variables
        std::env::remove_var("TEST_SCAN_PATH");
        std::env::remove_var("TEST_OUTPUT_FORMAT");
    }
    
    #[test]
    fn test_settings_validator() {
        // Test valid settings
        let valid_settings = Settings {
            scan_path: std::env::current_dir().unwrap(),
            ..Settings::default()
        };
        
        assert!(settings::SettingsValidator::validate(&valid_settings).is_ok());
        
        // Test invalid path
        let invalid_path_settings = Settings {
            scan_path: PathBuf::from("/nonexistent/path"),
            ..Settings::default()
        };
        
        assert!(settings::SettingsValidator::validate(&invalid_path_settings).is_err());
        
        // Test invalid max_depth
        let invalid_depth_settings = Settings {
            scan_path: std::env::current_dir().unwrap(),
            max_depth: Some(0),
            ..Settings::default()
        };
        
        assert!(settings::SettingsValidator::validate(&invalid_depth_settings).is_err());
    }
    
    #[test]
    fn test_load_config() {
        // Create a temporary config file
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config_content = r#"
            scan_path = "/file/path"
            exclude_patterns = ["file_exclude"]
            max_depth = 5
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Set environment variables
        std::env::set_var("WALKER_OUTPUT_FORMAT", "json");
        std::env::set_var("WALKER_CALCULATE_SIZE", "false");
        
        // Create CLI args with highest priority
        let cli_args = CliArgs {
            path: Some(PathBuf::from("/cli/path")),
            config: Some(config_path.clone()),
            ..Default::default()
        };
        
        // Load config with all sources
        let settings = load_config(cli_args).unwrap();
        
        // CLI args should override file config for scan_path
        assert_eq!(settings.scan_path, PathBuf::from("/cli/path"));
        // File config's exclude_patterns should be used
        assert_eq!(settings.exclude_patterns, vec!["file_exclude".to_string()]);
        // File config's max_depth should be used
        assert_eq!(settings.max_depth, Some(5));
        // Env config's output_format should be used
        assert!(matches!(settings.output_format, OutputFormat::Json));
        // Env config's calculate_size should be used
        assert_eq!(settings.calculate_size, false);
        
        // Clean up environment variables
        std::env::remove_var("WALKER_OUTPUT_FORMAT");
        std::env::remove_var("WALKER_CALCULATE_SIZE");
    }
    
    #[test]
    fn test_load_config_with_env_prefix() {
        // Create a temporary config file
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let config_content = r#"
            scan_path = "/file/path"
            exclude_patterns = ["file_exclude"]
            max_depth = 5
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Set environment variables with custom prefix
        std::env::set_var("CUSTOM_OUTPUT_FORMAT", "json");
        std::env::set_var("CUSTOM_CALCULATE_SIZE", "false");
        
        // Create CLI args with highest priority
        let cli_args = CliArgs {
            path: Some(PathBuf::from("/cli/path")),
            config: Some(config_path.clone()),
            ..Default::default()
        };
        
        // Load config with all sources and custom env prefix
        let settings = load_config_with_env_prefix(cli_args, "CUSTOM").unwrap();
        
        // CLI args should override file config for scan_path
        assert_eq!(settings.scan_path, PathBuf::from("/cli/path"));
        // File config's exclude_patterns should be used
        assert_eq!(settings.exclude_patterns, vec!["file_exclude".to_string()]);
        // File config's max_depth should be used
        assert_eq!(settings.max_depth, Some(5));
        // Env config's output_format should be used
        assert!(matches!(settings.output_format, OutputFormat::Json));
        // Env config's calculate_size should be used
        assert_eq!(settings.calculate_size, false);
        
        // Clean up environment variables
        std::env::remove_var("CUSTOM_OUTPUT_FORMAT");
        std::env::remove_var("CUSTOM_CALCULATE_SIZE");
    }
    
    #[test]
    fn test_load_config_default_locations() {
        // Save current directory
        let current_dir = std::env::current_dir().unwrap();
        
        // Create a temporary directory and change to it
        let temp_dir = tempdir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // Create a default config file in the current directory
        let config_content = r#"
            scan_path = "/default/path"
            exclude_patterns = ["default_exclude"]
            max_depth = 3
        "#;
        
        fs::write(".walker.toml", config_content).unwrap();
        
        // Create CLI args without specifying a config file
        let cli_args = CliArgs {
            output_format: Some("csv".to_string()),
            ..Default::default()
        };
        
        // Load config
        let settings = load_config(cli_args).unwrap();
        
        // Default config's scan_path should be used
        assert_eq!(settings.scan_path, PathBuf::from("/default/path"));
        // Default config's exclude_patterns should be used
        assert_eq!(settings.exclude_patterns, vec!["default_exclude".to_string()]);
        // Default config's max_depth should be used
        assert_eq!(settings.max_depth, Some(3));
        // CLI args' output_format should override default
        assert!(matches!(settings.output_format, OutputFormat::Csv));
        
        // Change back to original directory and clean up
        std::env::set_current_dir(current_dir).unwrap();
    }
    
    #[test]
    fn test_parse_config_file_validation() {
        let temp_dir = tempdir().unwrap();
        
        // Test invalid max_depth
        let invalid_depth_path = temp_dir.path().join("invalid_depth.toml");
        let invalid_depth_content = r#"
            scan_path = "/test/path"
            max_depth = 0  # Invalid - must be at least 1
        "#;
        
        fs::write(&invalid_depth_path, invalid_depth_content).unwrap();
        let result = parser::parse_config_file(&invalid_depth_path);
        assert!(result.is_err());
        
        // Test invalid exclude pattern
        let invalid_pattern_path = temp_dir.path().join("invalid_pattern.toml");
        let invalid_pattern_content = r#"
            scan_path = "/test/path"
            exclude_patterns = [""]  # Empty pattern is invalid
        "#;
        
        fs::write(&invalid_pattern_path, invalid_pattern_content).unwrap();
        let result = parser::parse_config_file(&invalid_pattern_path);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_create_default_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("default_config.toml");
        
        // Create default config
        parser::create_default_config(&config_path).unwrap();
        
        // Verify file exists
        assert!(config_path.exists());
        
        // Parse the file to ensure it's valid
        let result = parser::parse_config_file(&config_path);
        assert!(result.is_ok());
    }
}
use std::path::PathBuf;
use tempfile::tempdir;
use std::fs;
use walker::{
    config::parser::{parse_config_content, parse_config_file, validate_partial_settings, create_default_config},
    error::Result,
    models::config::{OutputFormat, PartialSettings},
};

#[test]
fn test_parse_config_content() -> Result<()> {
    let config_content = r#"
        scan_path = "/test/path"
        exclude_patterns = ["node_modules", "dist"]
        max_depth = 5
        output_format = "json"
        calculate_size = false
        parallel = false
    "#;

    let settings = parse_config_content(config_content, "virtual_path.toml")?;

    assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
    assert_eq!(settings.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
    assert_eq!(settings.max_depth, Some(5));
    assert_eq!(settings.output_format, Some(OutputFormat::Json));
    assert_eq!(settings.calculate_size, Some(false));
    assert_eq!(settings.parallel, Some(false));

    Ok(())
}

#[test]
fn test_parse_config_file() -> Result<()> {
    let temp_dir = tempdir()?;
    let config_path = temp_dir.path().join("test_config.toml");

    let config_content = r#"
        scan_path = "/test/path"
        exclude_patterns = ["node_modules", "dist"]
        max_depth = 5
        output_format = "json"
        calculate_size = false
        parallel = false
    "#;

    fs::write(&config_path, config_content)?;

    let settings = parse_config_file(&config_path)?;

    assert_eq!(settings.scan_path, Some(PathBuf::from("/test/path")));
    assert_eq!(settings.exclude_patterns, Some(vec!["node_modules".to_string(), "dist".to_string()]));
    assert_eq!(settings.max_depth, Some(5));
    assert_eq!(settings.output_format, Some(OutputFormat::Json));
    assert_eq!(settings.calculate_size, Some(false));
    assert_eq!(settings.parallel, Some(false));

    Ok(())
}

#[test]
fn test_validate_partial_settings() -> Result<()> {
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

    Ok(())
}

#[test]
fn test_create_default_config() -> Result<()> {
    let temp_dir = tempdir()?;
    let config_path = temp_dir.path().join("default_config.toml");

    assert!(!config_path.exists());

    create_default_config(&config_path)?;

    assert!(config_path.exists());

    // Parse the created file to ensure it's valid
    let settings = parse_config_file(&config_path)?;

    // Check that it contains default values
    assert!(settings.scan_path.is_none()); // Default values aren't included in PartialSettings

    Ok(())
}

#[test]
fn test_invalid_config_content() {
    // Invalid TOML syntax
    let invalid_toml = r#"
        scan_path = "/test/path"
        exclude_patterns = ["node_modules", "dist"
        max_depth = 5
    "#;

    let result = parse_config_content(invalid_toml, "virtual_path.toml");
    assert!(result.is_err());

    // Invalid output format
    let invalid_format = r#"
        scan_path = "/test/path"
        output_format = "invalid"
    "#;

    let result = parse_config_content(invalid_format, "virtual_path.toml");
    assert!(result.is_err());
}

#[test]
fn test_config_file_not_found() {
    let result = parse_config_file("non_existent_config.toml");
    assert!(result.is_err());
}
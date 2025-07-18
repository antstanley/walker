use std::path::PathBuf;
use walker::{
    error::Result,
    models::config::{OutputFormat, PartialSettings, Settings},
};

#[test]
fn test_partial_settings_merge() -> Result<()> {
    // Create base settings
    let mut base = PartialSettings {
        scan_path: Some(PathBuf::from("/base/path")),
        exclude_patterns: Some(vec!["node_modules".to_string()]),
        max_depth: Some(5),
        output_format: Some(OutputFormat::Text),
        ..Default::default()
    };

    // Create override settings
    let override_settings = PartialSettings {
        scan_path: Some(PathBuf::from("/override/path")),
        exclude_patterns: Some(vec!["dist".to_string(), "build".to_string()]),
        output_format: Some(OutputFormat::Json),
        calculate_size: Some(false),
        ..Default::default()
    };

    // Merge settings
    base.merge_from(override_settings);

    // Check merged results
    assert_eq!(base.scan_path, Some(PathBuf::from("/override/path")));
    assert_eq!(base.exclude_patterns, Some(vec!["dist".to_string(), "build".to_string()]));
    assert_eq!(base.max_depth, Some(5)); // Unchanged
    assert_eq!(base.output_format, Some(OutputFormat::Json));
    assert_eq!(base.calculate_size, Some(false));

    Ok(())
}

#[test]
fn test_partial_settings_to_settings() -> Result<()> {
    // Create partial settings
    let partial = PartialSettings {
        scan_path: Some(PathBuf::from("/custom/path")),
        exclude_patterns: Some(vec!["node_modules".to_string(), "dist".to_string()]),
        max_depth: Some(10),
        output_format: Some(OutputFormat::Json),
        calculate_size: Some(false),
        parallel: Some(false),
        ..Default::default()
    };

    // Convert to full settings
    let settings = partial.to_settings();

    // Check converted settings
    assert_eq!(settings.scan_path, PathBuf::from("/custom/path"));
    assert_eq!(settings.exclude_patterns, vec!["node_modules".to_string(), "dist".to_string()]);
    assert_eq!(settings.max_depth, Some(10));
    assert_eq!(settings.output_format, OutputFormat::Json);
    assert_eq!(settings.calculate_size, false);
    assert_eq!(settings.parallel, false);

    // Check that default values are used for unspecified fields
    assert!(settings.cache_enabled); // Default is true
    assert!(!settings.quiet); // Default is false
    assert!(!settings.verbose); // Default is false

    Ok(())
}

#[test]
fn test_settings_default() -> Result<()> {
    let settings = Settings::default();

    // Check default values
    assert_eq!(settings.scan_path, PathBuf::from("."));
    assert!(settings.exclude_patterns.contains(&"node_modules".to_string()));
    assert_eq!(settings.max_depth, None);
    assert!(matches!(settings.output_format, OutputFormat::Text));
    assert!(settings.calculate_size);
    assert!(settings.parallel);
    assert!(settings.cache_enabled);
    assert!(!settings.quiet);
    assert!(!settings.verbose);
    assert!(settings.use_colors);

    Ok(())
}

#[test]
fn test_output_format_from_str() {
    assert!(matches!("text".parse::<OutputFormat>(), Ok(OutputFormat::Text)));
    assert!(matches!("json".parse::<OutputFormat>(), Ok(OutputFormat::Json)));
    assert!(matches!("csv".parse::<OutputFormat>(), Ok(OutputFormat::Csv)));
    assert!(matches!("TEXT".parse::<OutputFormat>(), Ok(OutputFormat::Text)));
    assert!(matches!("JSON".parse::<OutputFormat>(), Ok(OutputFormat::Json)));
    assert!(matches!("CSV".parse::<OutputFormat>(), Ok(OutputFormat::Csv)));
    assert!("invalid".parse::<OutputFormat>().is_err());
}

#[test]
fn test_output_format_display() {
    assert_eq!(format!("{}", OutputFormat::Text), "text");
    assert_eq!(format!("{}", OutputFormat::Json), "json");
    assert_eq!(format!("{}", OutputFormat::Csv), "csv");
}
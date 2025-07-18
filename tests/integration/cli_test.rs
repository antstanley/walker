use std::path::PathBuf;
use walker::{
    cli::args::Args,
    models::config::OutputFormat,
};

#[test]
fn test_cli_args_parsing() {
    // Test default values
    let args = Args::parse_from(&["walker"]);
    assert_eq!(args.path, None);
    assert!(args.exclude.is_empty());
    assert_eq!(args.max_depth, None);
    assert!(matches!(args.output_format, OutputFormat::Text));
    assert_eq!(args.output_file, None);
    assert!(!args.quiet);
    assert!(!args.verbose);
    assert!(args.calculate_size);
    
    // Test with arguments
    let args = Args::parse_from(&[
        "walker",
        "--path", "/test/path",
        "--exclude", "node_modules",
        "--exclude", "dist",
        "--max-depth", "5",
        "--output", "json",
        "--output-file", "results.json",
        "--quiet",
        "--no-size",
    ]);
    
    assert_eq!(args.path, Some(PathBuf::from("/test/path")));
    assert_eq!(args.exclude, vec!["node_modules".to_string(), "dist".to_string()]);
    assert_eq!(args.max_depth, Some(5));
    assert!(matches!(args.output_format, OutputFormat::Json));
    assert_eq!(args.output_file, Some(PathBuf::from("results.json")));
    assert!(args.quiet);
    assert!(!args.verbose);
    assert!(!args.calculate_size);
}

#[test]
fn test_cli_version_command() {
    let args = Args::parse_from(&["walker", "--version"]);
    assert!(args.version);
}

#[test]
fn test_cli_help_command() {
    let args = Args::parse_from(&["walker", "--help"]);
    assert!(args.help);
}

#[test]
fn test_cli_config_option() {
    let args = Args::parse_from(&["walker", "--config", "custom-config.toml"]);
    assert_eq!(args.config, Some(PathBuf::from("custom-config.toml")));
}

#[test]
fn test_cli_output_formats() {
    // Test text format
    let args = Args::parse_from(&["walker", "--output", "text"]);
    assert!(matches!(args.output_format, OutputFormat::Text));
    
    // Test JSON format
    let args = Args::parse_from(&["walker", "--output", "json"]);
    assert!(matches!(args.output_format, OutputFormat::Json));
    
    // Test CSV format
    let args = Args::parse_from(&["walker", "--output", "csv"]);
    assert!(matches!(args.output_format, OutputFormat::Csv));
}

#[test]
#[should_panic]
fn test_cli_invalid_output_format() {
    // This should panic because "invalid" is not a valid output format
    Args::parse_from(&["walker", "--output", "invalid"]);
}

#[test]
fn test_cli_verbose_quiet_conflict() {
    // When both --verbose and --quiet are specified, the last one should win
    let args = Args::parse_from(&["walker", "--verbose", "--quiet"]);
    assert!(args.quiet);
    assert!(!args.verbose);
    
    let args = Args::parse_from(&["walker", "--quiet", "--verbose"]);
    assert!(!args.quiet);
    assert!(args.verbose);
}

#[test]
fn test_cli_multiple_excludes() {
    let args = Args::parse_from(&[
        "walker",
        "--exclude", "node_modules",
        "--exclude", "dist",
        "--exclude", "build",
        "--exclude", ".git",
    ]);
    
    assert_eq!(args.exclude, vec![
        "node_modules".to_string(),
        "dist".to_string(),
        "build".to_string(),
        ".git".to_string(),
    ]);
}
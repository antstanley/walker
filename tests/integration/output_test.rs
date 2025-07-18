use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walker::{
    core::analyzer::Analyzer,
    error::Result,
    models::{
        analysis::{AnalysisResults, AnalysisSummary, PackageAnalysis},
        config::OutputFormat,
        package::PackageDetails,
    },
    output::{
        formatters::{CsvFormatter, Formatter, JsonFormatter, TextFormatter},
        writers::FileWriter,
    },
};

fn create_test_analysis_results() -> AnalysisResults {
    // Create a simple package analysis
    let pkg1 = PackageAnalysis::new(
        PathBuf::from("/test/pkg1"),
        PackageDetails {
            name: "pkg1".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some("module".to_string()),
            ..Default::default()
        },
    );
    
    let pkg2 = PackageAnalysis::new(
        PathBuf::from("/test/pkg2"),
        PackageDetails {
            name: "pkg2".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some("commonjs".to_string()),
            ..Default::default()
        },
    );
    
    // Create analysis results
    AnalysisResults {
        packages: vec![pkg1, pkg2],
        summary: AnalysisSummary {
            total_packages: 2,
            esm_supported: 1,
            cjs_supported: 1,
            typescript_supported: 0,
            browser_supported: 0,
            total_size: 0,
            scan_duration: std::time::Duration::from_secs(1),
            errors_encountered: 0,
        },
        errors: vec![],
    }
}

#[test]
fn test_text_formatter() -> Result<()> {
    let results = create_test_analysis_results();
    
    // Test normal text formatter
    let formatter = TextFormatter::new(true, false, false);
    let output = formatter.format(&results)?;
    
    // Basic checks
    assert!(output.contains("pkg1"));
    assert!(output.contains("pkg2"));
    assert!(output.contains("ESM"));
    assert!(output.contains("CJS"));
    assert!(output.contains("Total packages: 2"));
    
    // Test verbose formatter
    let formatter = TextFormatter::new(true, true, false);
    let output = formatter.format(&results)?;
    
    // Verbose output should have more details
    assert!(output.contains("Path:"));
    assert!(output.contains("/test/pkg1"));
    assert!(output.contains("/test/pkg2"));
    
    // Test quiet formatter
    let formatter = TextFormatter::new(true, false, true);
    let output = formatter.format(&results)?;
    
    // Quiet output should be more concise
    assert!(output.contains("pkg1"));
    assert!(output.contains("pkg2"));
    assert!(!output.contains("Scan completed in"));
    
    Ok(())
}

#[test]
fn test_json_formatter() -> Result<()> {
    let results = create_test_analysis_results();
    
    // Test JSON formatter
    let formatter = JsonFormatter::new(false);
    let output = formatter.format(&results)?;
    
    // Parse the JSON output
    let json: serde_json::Value = serde_json::from_str(&output)?;
    
    // Check structure
    assert!(json.is_object());
    assert!(json.get("packages").is_some());
    assert!(json.get("summary").is_some());
    assert!(json.get("errors").is_some());
    
    // Check packages
    let packages = json.get("packages").unwrap().as_array().unwrap();
    assert_eq!(packages.len(), 2);
    
    // Check first package
    let pkg1 = &packages[0];
    assert_eq!(pkg1.get("name").unwrap().as_str().unwrap(), "pkg1");
    assert_eq!(pkg1.get("version").unwrap().as_str().unwrap(), "1.0.0");
    
    // Check summary
    let summary = json.get("summary").unwrap();
    assert_eq!(summary.get("total_packages").unwrap().as_u64().unwrap(), 2);
    assert_eq!(summary.get("esm_supported").unwrap().as_u64().unwrap(), 1);
    assert_eq!(summary.get("cjs_supported").unwrap().as_u64().unwrap(), 1);
    
    // Test pretty JSON formatter
    let formatter = JsonFormatter::new(true);
    let output = formatter.format(&results)?;
    
    // Pretty JSON should have newlines
    assert!(output.contains("\n"));
    
    Ok(())
}

#[test]
fn test_csv_formatter() -> Result<()> {
    let results = create_test_analysis_results();
    
    // Test CSV formatter
    let formatter = CsvFormatter::new();
    let output = formatter.format(&results)?;
    
    // CSV should have header and two data rows
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() >= 3); // Header + 2 data rows
    
    // Check header
    assert!(lines[0].contains("name"));
    assert!(lines[0].contains("version"));
    assert!(lines[0].contains("esm_support"));
    assert!(lines[0].contains("cjs_support"));
    
    // Check data rows
    assert!(lines[1].contains("pkg1"));
    assert!(lines[2].contains("pkg2"));
    
    Ok(())
}

#[test]
fn test_file_writer() -> Result<()> {
    let temp_dir = tempdir()?;
    let file_path = temp_dir.path().join("output.txt");
    
    // Create a file writer
    let writer = FileWriter::new(&file_path);
    
    // Write some content
    let content = "Test content";
    writer.write(content)?;
    
    // Read the file back
    let read_content = fs::read_to_string(&file_path)?;
    assert_eq!(read_content, content);
    
    Ok(())
}

#[test]
fn test_output_format_integration() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create test fixtures
    let pkg_dir = temp_dir.path().join("pkg");
    fs::create_dir_all(&pkg_dir)?;
    fs::write(pkg_dir.join("package.json"), r#"{
        "name": "test-pkg",
        "version": "1.0.0",
        "type": "module"
    }"#)?;
    
    // Create analyzer
    let analyzer = Analyzer::new(false, true);
    
    // Analyze the package
    let analysis = analyzer.analyze_package_with_options(&pkg_dir)?;
    
    // Create results
    let results = AnalysisResults {
        packages: vec![analysis],
        summary: AnalysisSummary {
            total_packages: 1,
            esm_supported: 1,
            cjs_supported: 0,
            typescript_supported: 0,
            browser_supported: 0,
            total_size: 0,
            scan_duration: std::time::Duration::from_secs(0),
            errors_encountered: 0,
        },
        errors: vec![],
    };
    
    // Test each output format
    let formats = vec![
        (OutputFormat::Text, "text_output.txt"),
        (OutputFormat::Json, "json_output.json"),
        (OutputFormat::Csv, "csv_output.csv"),
    ];
    
    for (format, filename) in formats {
        let file_path = temp_dir.path().join(filename);
        
        // Format the results
        let output = match format {
            OutputFormat::Text => TextFormatter::new(true, false, false).format(&results)?,
            OutputFormat::Json => JsonFormatter::new(true).format(&results)?,
            OutputFormat::Csv => CsvFormatter::new().format(&results)?,
        };
        
        // Write to file
        let writer = FileWriter::new(&file_path);
        writer.write(&output)?;
        
        // Check that the file exists and has content
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path)?;
        assert!(!content.is_empty());
        
        // Basic format-specific checks
        match format {
            OutputFormat::Text => assert!(content.contains("test-pkg")),
            OutputFormat::Json => assert!(content.contains("\"name\": \"test-pkg\"")),
            OutputFormat::Csv => assert!(content.contains("test-pkg")),
        }
    }
    
    Ok(())
}
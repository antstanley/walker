//! Tests for output formatting

#[cfg(test)]
mod tests {
    use crate::models::analysis::{AnalysisResults, PackageAnalysis};
    use crate::models::package::{PackageDetails, ModuleSupport};
    use crate::output::{TextFormatter, JsonFormatter, CsvFormatter, Formatter, FileWriter, OutputWriter};
    use std::path::PathBuf;
    use std::time::Duration;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;

    // Helper function to create test results
    fn create_test_results() -> AnalysisResults {
        let mut results = AnalysisResults::new();

        // Add a package
        let mut package = PackageAnalysis {
            path: PathBuf::from("/test/package"),
            details: PackageDetails {
                name: "test-package".to_string(),
                version: "1.0.0".to_string(),
                ..Default::default()
            },
            module_support: ModuleSupport::default(),
            size: Some(1024 * 1024), // 1MB
            dependencies: Default::default(),
            typescript_support: true,
            browser_support: false,
            node_version_requirement: Some(">=12".to_string()),
            license: Some("MIT".to_string()),
            has_bin: false,
            is_private: false,
            has_scripts: Default::default(),
            analysis_date: chrono::Utc::now(),
        };

        // Set module support
        package.module_support.esm.overall = true;
        package.module_support.cjs.overall = true;

        results.add_package(package);

        // Set summary values
        results.summary.scan_duration = Duration::from_secs(5);

        results
    }

    #[test]
    fn test_text_formatter() {
        let results = create_test_results();

        // Create formatters with different settings
        let normal_formatter = TextFormatter::new(false, false, false);
        let verbose_formatter = TextFormatter::new(false, true, false);
        let quiet_formatter = TextFormatter::new(false, false, true);

        // Test normal formatter
        let normal_output = normal_formatter.format(&results).unwrap();
        assert!(normal_output.contains("Package Analysis Summary"));
        assert!(normal_output.contains("Total packages: 1"));
        assert!(normal_output.contains("ESM support: 1"));
        assert!(normal_output.contains("CJS support: 1"));

        // Test verbose formatter
        let verbose_output = verbose_formatter.format(&results).unwrap();
        assert!(verbose_output.contains("Package Details:"));
        assert!(verbose_output.contains("Module Support Details:"));

        // Test quiet formatter
        let quiet_output = quiet_formatter.format(&results).unwrap();
        assert!(quiet_output.contains("Total: 1, ESM: 1"));
        assert!(!quiet_output.contains("Package Analysis Summary"));
    }

    #[test]
    fn test_json_formatter() {
        let results = create_test_results();

        // Create JSON formatter
        let json_formatter = JsonFormatter::new(true);

        // Test JSON formatter
        let json_output = json_formatter.format(&results).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_output).unwrap();

        // Check basic structure
        assert!(parsed.is_object());
        assert!(parsed.get("packages").is_some());
        assert!(parsed.get("summary").is_some());
        assert!(parsed.get("errors").is_some());

        // Check package data
        let packages = parsed["packages"].as_array().unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0]["details"]["name"], "test-package");
        assert_eq!(packages[0]["details"]["version"], "1.0.0");
        assert_eq!(packages[0]["module_support"]["esm"]["overall"], true);
        assert_eq!(packages[0]["module_support"]["cjs"]["overall"], true);
        assert_eq!(packages[0]["typescript_support"], true);
        assert_eq!(packages[0]["browser_support"], false);
    }

    #[test]
    fn test_csv_formatter() {
        let results = create_test_results();

        // Create CSV formatter
        let csv_formatter = CsvFormatter::new();

        // Test CSV formatter
        let csv_output = csv_formatter.format(&results).unwrap();

        // Check CSV structure
        let lines: Vec<&str> = csv_output.lines().collect();
        assert!(lines.len() >= 3); // Header, data row, summary row

        // Check header
        assert!(lines[0].contains("Package Name"));
        assert!(lines[0].contains("Version"));
        assert!(lines[0].contains("ESM Support"));

        // Check data row
        assert!(lines[1].contains("test-package"));
        assert!(lines[1].contains("1.0.0"));
        assert!(lines[1].contains("true")); // ESM support

        // Check summary row
        assert!(lines[2].contains("SUMMARY"));
    }

    #[test]
    fn test_file_writer() {
        // Create a temporary directory for the test
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("output.txt");

        // Create a file writer
        let writer = FileWriter::new(&file_path);

        // Write some content
        let content = "Test content";
        writer.write(content).unwrap();

        // Read the file back
        let mut file = fs::File::open(&file_path).unwrap();
        let mut read_content = String::new();
        file.read_to_string(&mut read_content).unwrap();

        // Verify the content
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_writer_creation() {
        // Test with file path
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("output.txt");

        // Create a file writer directly
        let writer = FileWriter::new(&file_path);

        // Write some content
        let content = "Test content";
        writer.write(content).unwrap();

        // Read the file back
        let mut file = fs::File::open(&file_path).unwrap();
        let mut read_content = String::new();
        file.read_to_string(&mut read_content).unwrap();

        // Verify the content
        assert_eq!(read_content, content);
    }
}

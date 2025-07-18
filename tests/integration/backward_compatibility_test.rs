//! Tests for backward compatibility with existing scripts and usage patterns
//!
//! These tests ensure that the Walker tool maintains backward compatibility
//! with existing usage patterns, especially when run without command-line arguments.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walker::{
    core::{Walker, ParallelWalker},
    models::config::Settings,
    cli::args::Args,
    cli::commands::Command,
    error::Result,
};

/// Create a test project structure with various package types
fn create_test_project_structure(base_dir: &Path) -> Result<()> {
    // Create a simple project structure with multiple packages
    
    // Root package
    fs::create_dir_all(base_dir.join("src"))?;
    fs::write(base_dir.join("package.json"), r#"{
        "name": "root-package",
        "version": "1.0.0",
        "type": "module"
    }"#)?;
    
    // Nested package 1 (ESM)
    let pkg1_dir = base_dir.join("packages").join("pkg1");
    fs::create_dir_all(&pkg1_dir)?;
    fs::write(pkg1_dir.join("package.json"), r#"{
        "name": "pkg1",
        "version": "1.0.0",
        "type": "module"
    }"#)?;
    
    // Nested package 2 (CommonJS)
    let pkg2_dir = base_dir.join("packages").join("pkg2");
    fs::create_dir_all(&pkg2_dir)?;
    fs::write(pkg2_dir.join("package.json"), r#"{
        "name": "pkg2",
        "version": "1.0.0",
        "type": "commonjs"
    }"#)?;
    
    // Dual mode package
    let dual_pkg_dir = base_dir.join("packages").join("dual");
    fs::create_dir_all(&dual_pkg_dir)?;
    fs::write(dual_pkg_dir.join("package.json"), r#"{
        "name": "dual-pkg",
        "version": "1.0.0",
        "main": "index.js",
        "module": "index.mjs",
        "exports": {
            "import": "./index.mjs",
            "require": "./index.js"
        }
    }"#)?;
    
    // TypeScript package
    let ts_pkg_dir = base_dir.join("packages").join("typescript");
    fs::create_dir_all(&ts_pkg_dir)?;
    fs::write(ts_pkg_dir.join("package.json"), r#"{
        "name": "ts-pkg",
        "version": "1.0.0",
        "main": "dist/index.js",
        "types": "dist/index.d.ts"
    }"#)?;
    
    // Create a node_modules directory with packages
    let node_modules_dir = base_dir.join("node_modules");
    fs::create_dir_all(&node_modules_dir)?;
    
    // Package in node_modules
    let nm_pkg_dir = node_modules_dir.join("some-dep");
    fs::create_dir_all(&nm_pkg_dir)?;
    fs::write(nm_pkg_dir.join("package.json"), r#"{
        "name": "some-dep",
        "version": "1.0.0"
    }"#)?;
    
    Ok(())
}

#[test]
/// Test that the Walker works with default settings (no command-line arguments)
fn test_default_behavior() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings with defaults (simulating no command-line args)
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 5 packages (root, pkg1, pkg2, dual, typescript)
    // but not the one in node_modules due to default exclude patterns
    assert_eq!(results.packages.len(), 5);
    
    // Check that we found the expected packages
    let package_names: Vec<String> = results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert!(package_names.contains(&"root-package".to_string()));
    assert!(package_names.contains(&"pkg1".to_string()));
    assert!(package_names.contains(&"pkg2".to_string()));
    assert!(package_names.contains(&"dual-pkg".to_string()));
    assert!(package_names.contains(&"ts-pkg".to_string()));
    
    // Make sure we didn't find the package in node_modules
    assert!(!package_names.contains(&"some-dep".to_string()));
    
    Ok(())
}

#[test]
/// Test that the ParallelWalker produces the same results as the regular Walker
fn test_parallel_walker_compatibility() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings with defaults
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create regular walker
    let walker = Walker::new(settings.clone());
    
    // Run analysis with regular walker
    let regular_results = walker.analyze()?;
    
    // Create parallel walker with same settings
    let parallel_walker = ParallelWalker::new(settings);
    
    // Run analysis with parallel walker
    let parallel_results = parallel_walker.analyze()?;
    
    // Both should find the same number of packages
    assert_eq!(regular_results.packages.len(), parallel_results.packages.len());
    
    // Both should find the same packages (by name)
    let regular_names: Vec<String> = regular_results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    let parallel_names: Vec<String> = parallel_results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert_eq!(regular_names.len(), parallel_names.len());
    
    for name in &regular_names {
        assert!(parallel_names.contains(name));
    }
    
    Ok(())
}

#[test]
/// Test that the CLI argument parsing maintains backward compatibility
fn test_cli_backward_compatibility() {
    // Test with no arguments (default behavior)
    let args = Args::parse_from(&["walker"]);
    assert_eq!(args.path, None);
    assert!(args.exclude.is_empty());
    assert_eq!(args.max_depth, None);
    assert!(matches!(args.output_format, walker::models::config::OutputFormat::Text));
    assert!(!args.quiet);
    assert!(!args.verbose);
    assert!(args.calculate_size);
    
    // Convert to command
    let command = Command::from_args(args);
    
    // Should be Analyze command with default settings
    match command {
        Command::Analyze(_) => {
            // This is expected
        },
        _ => {
            panic!("Expected Analyze command with default settings");
        }
    }
}

#[test]
/// Test that the Walker handles real-world project structures correctly
fn test_real_world_project_structure() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create a more complex project structure mimicking a real-world monorepo
    
    // Root package
    fs::create_dir_all(temp_dir.path().join("src"))?;
    fs::write(temp_dir.path().join("package.json"), r#"{
        "name": "monorepo-root",
        "version": "1.0.0",
        "private": true,
        "workspaces": ["packages/*"]
    }"#)?;
    
    // Create multiple packages with different configurations
    let packages_dir = temp_dir.path().join("packages");
    fs::create_dir_all(&packages_dir)?;
    
    // Package 1: ESM with TypeScript
    let pkg1_dir = packages_dir.join("pkg1");
    fs::create_dir_all(&pkg1_dir)?;
    fs::write(pkg1_dir.join("package.json"), r#"{
        "name": "@monorepo/pkg1",
        "version": "1.0.0",
        "type": "module",
        "main": "dist/index.js",
        "module": "dist/index.mjs",
        "types": "dist/index.d.ts",
        "exports": {
            ".": {
                "import": "./dist/index.mjs",
                "require": "./dist/index.cjs",
                "types": "./dist/index.d.ts"
            }
        },
        "dependencies": {
            "lodash": "^4.17.21"
        },
        "devDependencies": {
            "typescript": "^4.9.5"
        }
    }"#)?;
    
    // Package 2: CommonJS only
    let pkg2_dir = packages_dir.join("pkg2");
    fs::create_dir_all(&pkg2_dir)?;
    fs::write(pkg2_dir.join("package.json"), r#"{
        "name": "@monorepo/pkg2",
        "version": "1.0.0",
        "main": "lib/index.js",
        "dependencies": {
            "@monorepo/pkg1": "1.0.0"
        }
    }"#)?;
    
    // Package 3: Browser package
    let pkg3_dir = packages_dir.join("pkg3");
    fs::create_dir_all(&pkg3_dir)?;
    fs::write(pkg3_dir.join("package.json"), r#"{
        "name": "@monorepo/pkg3",
        "version": "1.0.0",
        "main": "dist/index.js",
        "browser": "dist/index.browser.js",
        "dependencies": {
            "@monorepo/pkg1": "1.0.0",
            "@monorepo/pkg2": "1.0.0"
        }
    }"#)?;
    
    // Create node_modules with nested dependencies
    let node_modules_dir = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules_dir)?;
    
    // Create settings with defaults
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 4 packages (root + 3 workspace packages)
    assert_eq!(results.packages.len(), 4);
    
    // Check that we found the expected packages
    let package_names: Vec<String> = results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert!(package_names.contains(&"monorepo-root".to_string()));
    assert!(package_names.contains(&"@monorepo/pkg1".to_string()));
    assert!(package_names.contains(&"@monorepo/pkg2".to_string()));
    assert!(package_names.contains(&"@monorepo/pkg3".to_string()));
    
    // Check that module support is correctly detected
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "@monorepo/pkg1" => {
                    // Should be detected as ESM and TypeScript
                    assert!(package.module_support.esm.overall);
                    assert!(package.typescript_support);
                },
                "@monorepo/pkg2" => {
                    // Should be detected as CommonJS only
                    assert!(!package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "@monorepo/pkg3" => {
                    // Should be detected as having browser support
                    assert!(package.browser_support);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker works with existing scripts (simulated by direct API calls)
fn test_existing_script_compatibility() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Simulate an existing script that uses the Walker API directly
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.exclude_patterns = vec!["node_modules".to_string()];
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Check that the analysis completed successfully
    assert!(results.packages.len() > 0);
    assert_eq!(results.errors.len(), 0);
    
    // Check that the summary statistics are calculated correctly
    assert_eq!(results.summary.total_packages, results.packages.len());
    
    // Check ESM vs CJS counts
    let esm_count = results.packages.iter()
        .filter(|p| p.module_support.esm.overall)
        .count();
    
    let cjs_count = results.packages.iter()
        .filter(|p| p.module_support.cjs.overall)
        .count();
    
    assert_eq!(results.summary.esm_supported, esm_count);
    assert_eq!(results.summary.cjs_supported, cjs_count);
    
    Ok(())
}

#[test]
/// Test that the Walker handles edge cases correctly
fn test_edge_cases() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create an empty directory (no packages)
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir_all(&empty_dir)?;
    
    // Create settings pointing to the empty directory
    let mut settings = Settings::default();
    settings.scan_path = empty_dir.clone();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 0 packages
    assert_eq!(results.packages.len(), 0);
    assert_eq!(results.errors.len(), 0);
    
    // Create a directory with an invalid package.json
    let invalid_dir = temp_dir.path().join("invalid");
    fs::create_dir_all(&invalid_dir)?;
    fs::write(invalid_dir.join("package.json"), "{ this is not valid JSON }")?;
    
    // Create settings pointing to the invalid directory
    let mut settings = Settings::default();
    settings.scan_path = invalid_dir.clone();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 0 packages and have an error
    assert_eq!(results.packages.len(), 0);
    assert_eq!(results.errors.len(), 1);
    
    Ok(())
}

#[test]
/// Test that the Walker handles permission errors gracefully
fn test_permission_handling() -> Result<()> {
    // Skip this test on Windows as permission handling is different
    if cfg!(windows) {
        return Ok(());
    }
    
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create a directory with restricted permissions
    // Note: This test may not work on all systems depending on user permissions
    let restricted_dir = temp_dir.path().join("restricted");
    fs::create_dir_all(&restricted_dir)?;
    
    // Try to make the directory non-readable (may fail on some systems)
    // This is a best-effort test
    let _ = std::process::Command::new("chmod")
        .args(&["000", restricted_dir.to_str().unwrap()])
        .output();
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // The analysis should complete even with permission errors
    assert!(results.packages.len() > 0);
    
    // Restore permissions for cleanup
    let _ = std::process::Command::new("chmod")
        .args(&["755", restricted_dir.to_str().unwrap()])
        .output();
    
    Ok(())
}

#[test]
/// Test that the Walker handles symlinks correctly
fn test_symlink_handling() -> Result<()> {
    // Skip this test on platforms where symlinks might be problematic
    if cfg!(windows) && !has_windows_symlink_privilege() {
        return Ok(());
    }
    
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create a directory with a package
    let target_dir = temp_dir.path().join("target");
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("package.json"), r#"{
        "name": "target-pkg",
        "version": "1.0.0"
    }"#)?;
    
    // Create a symlink to the target directory
    let symlink_dir = temp_dir.path().join("symlink");
    std::os::unix::fs::symlink(&target_dir, &symlink_dir)?;
    
    // Create settings with symlink following enabled
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.follow_links = true;
    
    // Create walker
    let walker = Walker::new(settings.clone());
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find the package through the symlink
    let package_paths: Vec<PathBuf> = results.packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    // Check if either the target or symlink path is found
    let found_target = package_paths.contains(&target_dir);
    let found_symlink = package_paths.contains(&symlink_dir);
    
    // At least one of them should be found
    assert!(found_target || found_symlink);
    
    // Now test with symlink following disabled
    settings.follow_links = false;
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should only find the target package, not through the symlink
    let package_paths: Vec<PathBuf> = results.packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    assert!(package_paths.contains(&target_dir));
    assert!(!package_paths.contains(&symlink_dir));
    
    Ok(())
}

/// Helper function to check if the current Windows user has symlink creation privileges
#[cfg(windows)]
fn has_windows_symlink_privilege() -> bool {
    use std::process::Command;
    
    // Try to create a symlink and check if it succeeds
    let temp_dir = match tempdir() {
        Ok(dir) => dir,
        Err(_) => return false,
    };
    
    let target = temp_dir.path().join("target");
    let symlink = temp_dir.path().join("symlink");
    
    if let Err(_) = std::fs::write(&target, "test") {
        return false;
    }
    
    let output = Command::new("cmd")
        .args(&["/C", "mklink", 
                symlink.to_str().unwrap(), 
                target.to_str().unwrap()])
        .output();
    
    match output {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(windows))]
fn has_windows_symlink_privilege() -> bool {
    false // Not relevant on non-Windows platforms
}
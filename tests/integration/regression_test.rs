//! Regression tests for the Walker tool
//!
//! These tests ensure that the Walker maintains backward compatibility
//! with existing scripts and usage patterns, and that bugs don't reappear.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walker::{
    core::{Walker, ParallelWalker},
    models::config::Settings,
    error::Result,
};

/// Create a test project structure with edge cases
fn create_edge_case_project(base_dir: &Path) -> Result<()> {
    // Root package
    fs::create_dir_all(base_dir.join("src"))?;
    fs::write(base_dir.join("package.json"), r#"{
        "name": "edge-case-project",
        "version": "1.0.0"
    }"#)?;
    
    // Package with empty package.json
    let empty_dir = base_dir.join("empty");
    fs::create_dir_all(&empty_dir)?;
    fs::write(empty_dir.join("package.json"), "{}")?;
    
    // Package with minimal package.json
    let minimal_dir = base_dir.join("minimal");
    fs::create_dir_all(&minimal_dir)?;
    fs::write(minimal_dir.join("package.json"), r#"{"name": "minimal"}"#)?;
    
    // Package with invalid JSON (should be handled gracefully)
    let invalid_dir = base_dir.join("invalid");
    fs::create_dir_all(&invalid_dir)?;
    fs::write(invalid_dir.join("package.json"), "{ this is not valid JSON }")?;
    
    // Package with unusual fields
    let unusual_dir = base_dir.join("unusual");
    fs::create_dir_all(&unusual_dir)?;
    fs::write(unusual_dir.join("package.json"), r#"{
        "name": "unusual",
        "version": "1.0.0",
        "type": "module",
        "main": "./dist/index.js",
        "module": "./dist/index.mjs",
        "types": "./dist/index.d.ts",
        "exports": {
            ".": {
                "types": "./dist/index.d.ts",
                "node": {
                    "import": "./dist/index.mjs",
                    "require": "./dist/index.js"
                },
                "default": "./dist/index.js"
            },
            "./package.json": "./package.json"
        },
        "customField": "value",
        "anotherCustomField": {
            "nested": "value"
        }
    }"#)?;
    
    // Package with very large package.json
    let large_dir = base_dir.join("large");
    fs::create_dir_all(&large_dir)?;
    
    // Generate a large package.json with many dependencies
    let mut large_pkg_json = r#"{
        "name": "large-package",
        "version": "1.0.0",
        "dependencies": {
    "#.to_string();
    
    for i in 0..100 {
        large_pkg_json.push_str(&format!(r#"        "dep-{}": "^1.0.0"{}"#, 
            i, 
            if i < 99 { ",\n" } else { "\n" }
        ));
    }
    
    large_pkg_json.push_str(r#"    },
        "devDependencies": {
    "#);
    
    for i in 0..100 {
        large_pkg_json.push_str(&format!(r#"        "dev-dep-{}": "^1.0.0"{}"#, 
            i, 
            if i < 99 { ",\n" } else { "\n" }
        ));
    }
    
    large_pkg_json.push_str(r#"    }
    }"#);
    
    fs::write(large_dir.join("package.json"), large_pkg_json)?;
    
    // Create a directory with a package.json that has unusual whitespace
    let whitespace_dir = base_dir.join("whitespace");
    fs::create_dir_all(&whitespace_dir)?;
    fs::write(whitespace_dir.join("package.json"), r#"
    
    
    {
        "name": "whitespace",
        "version": "1.0.0",
        "main":    "index.js",
        "type":     "module"
    }
    
    
    "#)?;
    
    Ok(())
}

#[test]
/// Test that the Walker handles edge cases correctly
fn test_edge_cases() -> Result<()> {
    let temp_dir = tempdir()?;
    create_edge_case_project(temp_dir.path())?;
    
    // Create settings with defaults
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 6 packages (root, empty, minimal, unusual, large, whitespace)
    // but not the invalid one (which should be in errors)
    assert_eq!(results.packages.len(), 6);
    
    // Check that we found the expected packages
    let package_names: Vec<String> = results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert!(package_names.contains(&"edge-case-project".to_string()));
    assert!(package_names.contains(&"minimal".to_string()));
    assert!(package_names.contains(&"unusual".to_string()));
    assert!(package_names.contains(&"large-package".to_string()));
    assert!(package_names.contains(&"whitespace".to_string()));
    
    // The empty package.json might not have a name, so check by path
    let package_paths: Vec<PathBuf> = results.packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    assert!(package_paths.contains(&temp_dir.path().join("empty")));
    
    // Check that the invalid package.json is in errors
    assert_eq!(results.errors.len(), 1);
    assert_eq!(results.errors[0].path, temp_dir.path().join("invalid"));
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual module configurations correctly
fn test_unusual_module_configs() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual module configurations
    
    // 1. Package with both type:module and main ending in .cjs
    let mixed1_dir = temp_dir.path().join("mixed1");
    fs::create_dir_all(&mixed1_dir)?;
    fs::write(mixed1_dir.join("package.json"), r#"{
        "name": "mixed1",
        "version": "1.0.0",
        "type": "module",
        "main": "index.cjs"
    }"#)?;
    
    // 2. Package with main:index.mjs but type:commonjs
    let mixed2_dir = temp_dir.path().join("mixed2");
    fs::create_dir_all(&mixed2_dir)?;
    fs::write(mixed2_dir.join("package.json"), r#"{
        "name": "mixed2",
        "version": "1.0.0",
        "type": "commonjs",
        "main": "index.mjs"
    }"#)?;
    
    // 3. Package with exports field but no main or module
    let exports_only_dir = temp_dir.path().join("exports-only");
    fs::create_dir_all(&exports_only_dir)?;
    fs::write(exports_only_dir.join("package.json"), r#"{
        "name": "exports-only",
        "version": "1.0.0",
        "exports": {
            ".": "./index.js"
        }
    }"#)?;
    
    // 4. Package with module field but no main
    let module_only_dir = temp_dir.path().join("module-only");
    fs::create_dir_all(&module_only_dir)?;
    fs::write(module_only_dir.join("package.json"), r#"{
        "name": "module-only",
        "version": "1.0.0",
        "module": "index.mjs"
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 4 packages
    assert_eq!(results.packages.len(), 4);
    
    // Check module system detection
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "mixed1" => {
                    // Should be detected as both ESM and CommonJS
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "mixed2" => {
                    // Should be detected as both ESM and CommonJS
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "exports-only" => {
                    // Should be detected as CommonJS by default
                    assert!(package.module_support.cjs.overall);
                },
                "module-only" => {
                    // Should be detected as ESM
                    assert!(package.module_support.esm.overall);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual dependency configurations correctly
fn test_unusual_dependency_configs() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual dependency configurations
    
    // 1. Package with empty dependencies
    let empty_deps_dir = temp_dir.path().join("empty-deps");
    fs::create_dir_all(&empty_deps_dir)?;
    fs::write(empty_deps_dir.join("package.json"), r#"{
        "name": "empty-deps",
        "version": "1.0.0",
        "dependencies": {}
    }"#)?;
    
    // 2. Package with null dependencies
    let null_deps_dir = temp_dir.path().join("null-deps");
    fs::create_dir_all(&null_deps_dir)?;
    fs::write(null_deps_dir.join("package.json"), r#"{
        "name": "null-deps",
        "version": "1.0.0",
        "dependencies": null
    }"#)?;
    
    // 3. Package with non-string dependency versions
    let non_string_deps_dir = temp_dir.path().join("non-string-deps");
    fs::create_dir_all(&non_string_deps_dir)?;
    fs::write(non_string_deps_dir.join("package.json"), r#"{
        "name": "non-string-deps",
        "version": "1.0.0",
        "dependencies": {
            "dep1": 1,
            "dep2": true,
            "dep3": null,
            "dep4": { "version": "1.0.0" }
        }
    }"#)?;
    
    // 4. Package with unusual dependency types
    let unusual_deps_dir = temp_dir.path().join("unusual-deps");
    fs::create_dir_all(&unusual_deps_dir)?;
    fs::write(unusual_deps_dir.join("package.json"), r#"{
        "name": "unusual-deps",
        "version": "1.0.0",
        "dependencies": {
            "dep1": "1.0.0"
        },
        "devDependencies": {
            "dev1": "1.0.0"
        },
        "peerDependencies": {
            "peer1": "1.0.0"
        },
        "optionalDependencies": {
            "opt1": "1.0.0"
        },
        "bundledDependencies": ["dep1"],
        "customDependencies": {
            "custom1": "1.0.0"
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 4 packages
    assert_eq!(results.packages.len(), 4);
    
    // Check dependency analysis
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "empty-deps" => {
                    // Should have 0 dependencies
                    assert_eq!(package.dependencies.total_count, 0);
                },
                "null-deps" => {
                    // Should have 0 dependencies
                    assert_eq!(package.dependencies.total_count, 0);
                },
                "non-string-deps" => {
                    // Should have some dependencies, but the exact count depends on implementation
                    // Just check that it doesn't crash
                },
                "unusual-deps" => {
                    // Should have 4 dependencies (1 regular, 1 dev, 1 peer, 1 optional)
                    assert_eq!(package.dependencies.total_count, 4);
                    assert_eq!(package.dependencies.production_count, 1);
                    assert_eq!(package.dependencies.development_count, 1);
                    assert_eq!(package.dependencies.peer_count, 1);
                    assert_eq!(package.dependencies.optional_count, 1);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual version fields correctly
fn test_unusual_version_fields() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual version fields
    
    // 1. Package with no version
    let no_version_dir = temp_dir.path().join("no-version");
    fs::create_dir_all(&no_version_dir)?;
    fs::write(no_version_dir.join("package.json"), r#"{
        "name": "no-version"
    }"#)?;
    
    // 2. Package with non-string version
    let non_string_version_dir = temp_dir.path().join("non-string-version");
    fs::create_dir_all(&non_string_version_dir)?;
    fs::write(non_string_version_dir.join("package.json"), r#"{
        "name": "non-string-version",
        "version": 1
    }"#)?;
    
    // 3. Package with unusual version format
    let unusual_version_dir = temp_dir.path().join("unusual-version");
    fs::create_dir_all(&unusual_version_dir)?;
    fs::write(unusual_version_dir.join("package.json"), r#"{
        "name": "unusual-version",
        "version": "not.a.semver"
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 3 packages
    assert_eq!(results.packages.len(), 3);
    
    // Check version handling
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "no-version" => {
                    // Should have no version or a default version
                    assert!(package.details.version.is_empty() || package.details.version == "0.0.0");
                },
                "non-string-version" => {
                    // Should convert to string or use default
                    assert!(!package.details.version.is_empty());
                },
                "unusual-version" => {
                    // Should preserve the unusual version
                    assert_eq!(package.details.version, "not.a.semver");
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual name fields correctly
fn test_unusual_name_fields() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual name fields
    
    // 1. Package with no name
    let no_name_dir = temp_dir.path().join("no-name");
    fs::create_dir_all(&no_name_dir)?;
    fs::write(no_name_dir.join("package.json"), r#"{
        "version": "1.0.0"
    }"#)?;
    
    // 2. Package with non-string name
    let non_string_name_dir = temp_dir.path().join("non-string-name");
    fs::create_dir_all(&non_string_name_dir)?;
    fs::write(non_string_name_dir.join("package.json"), r#"{
        "name": 123,
        "version": "1.0.0"
    }"#)?;
    
    // 3. Package with unusual name format
    let unusual_name_dir = temp_dir.path().join("unusual-name");
    fs::create_dir_all(&unusual_name_dir)?;
    fs::write(unusual_name_dir.join("package.json"), r#"{
        "name": "This is not a valid package name!",
        "version": "1.0.0"
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 3 packages
    assert_eq!(results.packages.len(), 3);
    
    // Check name handling
    for package in &results.packages {
        match package.path.file_name().unwrap().to_string_lossy().as_ref() {
            "no-name" => {
                // Should have no name or use directory name
                assert!(package.details.name.is_none() || 
                       package.details.name.as_ref().unwrap() == "no-name");
            },
            "non-string-name" => {
                // Should convert to string or use directory name
                assert!(package.details.name.is_some());
                if let Some(name) = &package.details.name {
                    assert!(name == "123" || name == "non-string-name");
                }
            },
            "unusual-name" => {
                // Should preserve the unusual name
                assert_eq!(package.details.name.as_ref().unwrap(), "This is not a valid package name!");
            },
            _ => {}
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual engines fields correctly
fn test_unusual_engines_fields() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual engines fields
    
    // 1. Package with string engines field
    let string_engines_dir = temp_dir.path().join("string-engines");
    fs::create_dir_all(&string_engines_dir)?;
    fs::write(string_engines_dir.join("package.json"), r#"{
        "name": "string-engines",
        "version": "1.0.0",
        "engines": "node >= 14"
    }"#)?;
    
    // 2. Package with object engines field
    let object_engines_dir = temp_dir.path().join("object-engines");
    fs::create_dir_all(&object_engines_dir)?;
    fs::write(object_engines_dir.join("package.json"), r#"{
        "name": "object-engines",
        "version": "1.0.0",
        "engines": {
            "node": ">=14",
            "npm": ">=6"
        }
    }"#)?;
    
    // 3. Package with unusual engines field
    let unusual_engines_dir = temp_dir.path().join("unusual-engines");
    fs::create_dir_all(&unusual_engines_dir)?;
    fs::write(unusual_engines_dir.join("package.json"), r#"{
        "name": "unusual-engines",
        "version": "1.0.0",
        "engines": {
            "node": 14,
            "npm": true,
            "custom": "value"
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 3 packages
    assert_eq!(results.packages.len(), 3);
    
    // Check engines handling
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "string-engines" => {
                    // Should handle string engines field
                    assert!(package.node_version_requirement.is_some());
                },
                "object-engines" => {
                    // Should extract node version requirement
                    assert_eq!(package.node_version_requirement.as_ref().unwrap(), ">=14");
                },
                "unusual-engines" => {
                    // Should handle unusual engines field
                    assert!(package.node_version_requirement.is_some());
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual license fields correctly
fn test_unusual_license_fields() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual license fields
    
    // 1. Package with string license field
    let string_license_dir = temp_dir.path().join("string-license");
    fs::create_dir_all(&string_license_dir)?;
    fs::write(string_license_dir.join("package.json"), r#"{
        "name": "string-license",
        "version": "1.0.0",
        "license": "MIT"
    }"#)?;
    
    // 2. Package with object license field
    let object_license_dir = temp_dir.path().join("object-license");
    fs::create_dir_all(&object_license_dir)?;
    fs::write(object_license_dir.join("package.json"), r#"{
        "name": "object-license",
        "version": "1.0.0",
        "license": {
            "type": "MIT",
            "url": "https://opensource.org/licenses/MIT"
        }
    }"#)?;
    
    // 3. Package with licenses array (deprecated but still used)
    let licenses_array_dir = temp_dir.path().join("licenses-array");
    fs::create_dir_all(&licenses_array_dir)?;
    fs::write(licenses_array_dir.join("package.json"), r#"{
        "name": "licenses-array",
        "version": "1.0.0",
        "licenses": [
            {
                "type": "MIT",
                "url": "https://opensource.org/licenses/MIT"
            },
            {
                "type": "Apache-2.0",
                "url": "https://opensource.org/licenses/Apache-2.0"
            }
        ]
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 3 packages
    assert_eq!(results.packages.len(), 3);
    
    // Check license handling
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "string-license" => {
                    // Should extract license
                    assert_eq!(package.license.as_ref().unwrap(), "MIT");
                },
                "object-license" => {
                    // Should extract license from object
                    assert!(package.license.is_some());
                },
                "licenses-array" => {
                    // Should handle licenses array
                    assert!(package.license.is_some());
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual file structures correctly
fn test_unusual_file_structures() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual file structures
    
    // 1. Package with package.json in subdirectory
    let nested_dir = temp_dir.path().join("nested");
    fs::create_dir_all(&nested_dir.join("subdir"))?;
    fs::write(nested_dir.join("subdir").join("package.json"), r#"{
        "name": "nested-package",
        "version": "1.0.0"
    }"#)?;
    
    // 2. Package with multiple package.json files
    let multiple_dir = temp_dir.path().join("multiple");
    fs::create_dir_all(&multiple_dir.join("subdir"))?;
    fs::write(multiple_dir.join("package.json"), r#"{
        "name": "multiple-root",
        "version": "1.0.0"
    }"#)?;
    fs::write(multiple_dir.join("subdir").join("package.json"), r#"{
        "name": "multiple-sub",
        "version": "1.0.0"
    }"#)?;
    
    // 3. Package with very deep nesting
    let deep_dir = temp_dir.path().join("deep");
    let mut current_dir = deep_dir.clone();
    for i in 1..6 {
        current_dir = current_dir.join(format!("level{}", i));
        fs::create_dir_all(&current_dir)?;
    }
    fs::write(current_dir.join("package.json"), r#"{
        "name": "deep-package",
        "version": "1.0.0"
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 4 packages
    assert_eq!(results.packages.len(), 4);
    
    // Check that we found all packages
    let package_names: Vec<String> = results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert!(package_names.contains(&"nested-package".to_string()));
    assert!(package_names.contains(&"multiple-root".to_string()));
    assert!(package_names.contains(&"multiple-sub".to_string()));
    assert!(package_names.contains(&"deep-package".to_string()));
    
    Ok(())
}

#[test]
/// Test that the Walker handles packages with unusual exports fields correctly
fn test_unusual_exports_fields() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with unusual exports fields
    
    // 1. Package with string exports field
    let string_exports_dir = temp_dir.path().join("string-exports");
    fs::create_dir_all(&string_exports_dir)?;
    fs::write(string_exports_dir.join("package.json"), r#"{
        "name": "string-exports",
        "version": "1.0.0",
        "exports": "./index.js"
    }"#)?;
    
    // 2. Package with array exports field
    let array_exports_dir = temp_dir.path().join("array-exports");
    fs::create_dir_all(&array_exports_dir)?;
    fs::write(array_exports_dir.join("package.json"), r#"{
        "name": "array-exports",
        "version": "1.0.0",
        "exports": ["./a.js", "./b.js"]
    }"#)?;
    
    // 3. Package with nested conditions
    let nested_conditions_dir = temp_dir.path().join("nested-conditions");
    fs::create_dir_all(&nested_conditions_dir)?;
    fs::write(nested_conditions_dir.join("package.json"), r#"{
        "name": "nested-conditions",
        "version": "1.0.0",
        "exports": {
            ".": {
                "node": {
                    "import": {
                        "types": "./types/index.d.mts",
                        "default": "./esm/index.js"
                    },
                    "require": {
                        "types": "./types/index.d.cts",
                        "default": "./cjs/index.js"
                    }
                },
                "browser": {
                    "development": {
                        "import": "./browser/dev.mjs",
                        "require": "./browser/dev.cjs"
                    },
                    "production": {
                        "import": "./browser/prod.mjs",
                        "require": "./browser/prod.cjs"
                    },
                    "default": {
                        "import": "./browser/index.mjs",
                        "require": "./browser/index.cjs"
                    }
                },
                "default": "./index.js"
            }
        }
    }"#)?;
    
    // 4. Package with unusual conditions
    let unusual_conditions_dir = temp_dir.path().join("unusual-conditions");
    fs::create_dir_all(&unusual_conditions_dir)?;
    fs::write(unusual_conditions_dir.join("package.json"), r#"{
        "name": "unusual-conditions",
        "version": "1.0.0",
        "exports": {
            ".": {
                "custom-condition": "./custom.js",
                "electron": "./electron.js",
                "react-native": "./react-native.js",
                "worker": "./worker.js",
                "default": "./index.js"
            }
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 4 packages
    assert_eq!(results.packages.len(), 4);
    
    // Check exports handling
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "string-exports" => {
                    // Should be detected as CommonJS by default
                    assert!(package.module_support.cjs.overall);
                },
                "array-exports" => {
                    // Should be detected as CommonJS by default
                    assert!(package.module_support.cjs.overall);
                },
                "nested-conditions" => {
                    // Should be detected as both ESM and CommonJS
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                    // Should be detected as TypeScript and browser
                    assert!(package.typescript_support);
                    assert!(package.browser_support);
                },
                "unusual-conditions" => {
                    // Should be detected as CommonJS by default
                    assert!(package.module_support.cjs.overall);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}
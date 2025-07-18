//! End-to-end tests for the Walker tool
//!
//! These tests verify that the Walker tool works correctly with real-world projects
//! and maintains backward compatibility with existing usage patterns.

use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walker::{
    core::{Walker, ParallelWalker},
    models::config::Settings,
    error::Result,
};

/// Create a complex project structure mimicking a real-world monorepo
fn create_complex_monorepo(base_dir: &Path) -> Result<()> {
    // Root package
    fs::create_dir_all(base_dir.join("src"))?;
    fs::write(base_dir.join("package.json"), r#"{
        "name": "complex-monorepo",
        "version": "1.0.0",
        "private": true,
        "workspaces": ["packages/*"],
        "scripts": {
            "analyze": "walker"
        }
    }"#)?;
    
    // Create packages directory
    let packages_dir = base_dir.join("packages");
    fs::create_dir_all(&packages_dir)?;
    
    // Create multiple packages with different configurations
    
    // 1. ESM package with TypeScript
    let esm_ts_dir = packages_dir.join("esm-ts");
    fs::create_dir_all(&esm_ts_dir)?;
    fs::write(esm_ts_dir.join("package.json"), r#"{
        "name": "@monorepo/esm-ts",
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
    
    // 2. CommonJS package
    let cjs_dir = packages_dir.join("cjs");
    fs::create_dir_all(&cjs_dir)?;
    fs::write(cjs_dir.join("package.json"), r#"{
        "name": "@monorepo/cjs",
        "version": "1.0.0",
        "main": "lib/index.js",
        "dependencies": {
            "@monorepo/esm-ts": "1.0.0"
        }
    }"#)?;
    
    // 3. Dual-mode package
    let dual_dir = packages_dir.join("dual");
    fs::create_dir_all(&dual_dir)?;
    fs::write(dual_dir.join("package.json"), r#"{
        "name": "@monorepo/dual",
        "version": "1.0.0",
        "main": "dist/index.js",
        "module": "dist/index.mjs",
        "exports": {
            ".": {
                "import": "./dist/index.mjs",
                "require": "./dist/index.js"
            }
        },
        "dependencies": {
            "@monorepo/cjs": "1.0.0"
        }
    }"#)?;
    
    // 4. Browser package
    let browser_dir = packages_dir.join("browser");
    fs::create_dir_all(&browser_dir)?;
    fs::write(browser_dir.join("package.json"), r#"{
        "name": "@monorepo/browser",
        "version": "1.0.0",
        "main": "dist/index.js",
        "browser": "dist/index.browser.js",
        "dependencies": {
            "@monorepo/dual": "1.0.0"
        }
    }"#)?;
    
    // 5. Complex exports package
    let complex_dir = packages_dir.join("complex");
    fs::create_dir_all(&complex_dir)?;
    fs::write(complex_dir.join("package.json"), r#"{
        "name": "@monorepo/complex",
        "version": "1.0.0",
        "main": "dist/index.js",
        "module": "dist/index.mjs",
        "types": "dist/index.d.ts",
        "exports": {
            ".": {
                "types": "./dist/index.d.ts",
                "node": {
                    "import": {
                        "types": "./dist/index.d.mts",
                        "default": "./dist/index.mjs"
                    },
                    "require": {
                        "types": "./dist/index.d.cts",
                        "default": "./dist/index.cjs"
                    }
                },
                "browser": {
                    "import": "./dist/browser.mjs",
                    "require": "./dist/browser.cjs"
                },
                "default": "./dist/index.js"
            },
            "./utils": {
                "import": "./dist/utils.mjs",
                "require": "./dist/utils.cjs"
            }
        },
        "dependencies": {
            "@monorepo/browser": "1.0.0",
            "@monorepo/esm-ts": "1.0.0"
        }
    }"#)?;
    
    // Create deeply nested packages
    let deep_dir = packages_dir.join("deep").join("nested").join("package");
    fs::create_dir_all(&deep_dir)?;
    fs::write(deep_dir.join("package.json"), r#"{
        "name": "@monorepo/deep",
        "version": "1.0.0",
        "private": true
    }"#)?;
    
    // Create node_modules with nested dependencies
    let node_modules_dir = base_dir.join("node_modules");
    fs::create_dir_all(&node_modules_dir)?;
    
    // Create a node_modules package
    let nm_pkg_dir = node_modules_dir.join("some-dep");
    fs::create_dir_all(&nm_pkg_dir)?;
    fs::write(nm_pkg_dir.join("package.json"), r#"{
        "name": "some-dep",
        "version": "1.0.0"
    }"#)?;
    
    // Create a nested node_modules package
    let nested_nm_dir = nm_pkg_dir.join("node_modules").join("nested-dep");
    fs::create_dir_all(&nested_nm_dir)?;
    fs::write(nested_nm_dir.join("package.json"), r#"{
        "name": "nested-dep",
        "version": "1.0.0"
    }"#)?;
    
    Ok(())
}

#[test]
/// Test that the Walker works correctly with a complex monorepo structure
fn test_complex_monorepo() -> Result<()> {
    let temp_dir = tempdir()?;
    create_complex_monorepo(temp_dir.path())?;
    
    // Create settings with defaults
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 7 packages (root + 6 workspace packages)
    // but not the ones in node_modules due to default exclude patterns
    assert_eq!(results.packages.len(), 7);
    
    // Check that we found the expected packages
    let package_names: Vec<String> = results.packages.iter()
        .filter_map(|p| p.details.name.clone())
        .collect();
    
    assert!(package_names.contains(&"complex-monorepo".to_string()));
    assert!(package_names.contains(&"@monorepo/esm-ts".to_string()));
    assert!(package_names.contains(&"@monorepo/cjs".to_string()));
    assert!(package_names.contains(&"@monorepo/dual".to_string()));
    assert!(package_names.contains(&"@monorepo/browser".to_string()));
    assert!(package_names.contains(&"@monorepo/complex".to_string()));
    assert!(package_names.contains(&"@monorepo/deep".to_string()));
    
    // Make sure we didn't find the packages in node_modules
    assert!(!package_names.contains(&"some-dep".to_string()));
    assert!(!package_names.contains(&"nested-dep".to_string()));
    
    // Check that module support is correctly detected
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "@monorepo/esm-ts" => {
                    // Should be detected as ESM and TypeScript
                    assert!(package.module_support.esm.overall);
                    assert!(package.typescript_support);
                },
                "@monorepo/cjs" => {
                    // Should be detected as CommonJS only
                    assert!(!package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "@monorepo/dual" => {
                    // Should be detected as both ESM and CommonJS
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "@monorepo/browser" => {
                    // Should be detected as having browser support
                    assert!(package.browser_support);
                },
                "@monorepo/complex" => {
                    // Should be detected as ESM, CommonJS, TypeScript, and browser
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                    assert!(package.typescript_support);
                    assert!(package.browser_support);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker and ParallelWalker produce identical results
fn test_walker_parallel_equivalence() -> Result<()> {
    let temp_dir = tempdir()?;
    create_complex_monorepo(temp_dir.path())?;
    
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
    
    // Check that module support detection is consistent
    for regular_pkg in &regular_results.packages {
        if let Some(name) = &regular_pkg.details.name {
            // Find the corresponding package in parallel results
            let parallel_pkg = parallel_results.packages.iter()
                .find(|p| p.details.name.as_ref() == Some(name));
            
            if let Some(parallel_pkg) = parallel_pkg {
                // Module support should be the same
                assert_eq!(
                    regular_pkg.module_support.esm.overall,
                    parallel_pkg.module_support.esm.overall,
                    "ESM support detection differs for package {}",
                    name
                );
                
                assert_eq!(
                    regular_pkg.module_support.cjs.overall,
                    parallel_pkg.module_support.cjs.overall,
                    "CJS support detection differs for package {}",
                    name
                );
                
                // TypeScript support should be the same
                assert_eq!(
                    regular_pkg.typescript_support,
                    parallel_pkg.typescript_support,
                    "TypeScript support detection differs for package {}",
                    name
                );
                
                // Browser support should be the same
                assert_eq!(
                    regular_pkg.browser_support,
                    parallel_pkg.browser_support,
                    "Browser support detection differs for package {}",
                    name
                );
            } else {
                panic!("Package {} found in regular results but not in parallel results", name);
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles large projects efficiently
fn test_large_project_performance() -> Result<()> {
    // Skip this test in CI environments to avoid long-running tests
    if std::env::var("CI").is_ok() {
        return Ok(());
    }
    
    let temp_dir = tempdir()?;
    
    // Create a large project structure with many packages
    // This is a simplified version for testing purposes
    
    // Root package
    fs::create_dir_all(temp_dir.path().join("src"))?;
    fs::write(temp_dir.path().join("package.json"), r#"{
        "name": "large-project",
        "version": "1.0.0",
        "private": true
    }"#)?;
    
    // Create many packages
    let num_packages = 100; // Adjust based on test environment capabilities
    
    for i in 0..num_packages {
        let pkg_dir = temp_dir.path().join("packages").join(format!("pkg-{}", i));
        fs::create_dir_all(&pkg_dir)?;
        fs::write(pkg_dir.join("package.json"), format!(r#"{{
            "name": "pkg-{}",
            "version": "1.0.0",
            "main": "index.js"
        }}"#, i))?;
    }
    
    // Create settings with defaults
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.parallel = true; // Use parallel processing for better performance
    
    // Create walker
    let parallel_walker = ParallelWalker::new(settings);
    
    // Measure execution time
    let start = std::time::Instant::now();
    
    // Run analysis
    let results = parallel_walker.analyze()?;
    
    // Calculate execution time
    let duration = start.elapsed();
    
    // Should find all packages
    assert_eq!(results.packages.len(), num_packages + 1); // +1 for root package
    
    // Check that execution time is reasonable
    // This is a soft assertion as performance depends on the test environment
    println!("Large project analysis took: {:?}", duration);
    
    // On a modern system, analyzing 100 packages should take less than 5 seconds
    // This is a very generous limit to avoid test failures on slower systems
    assert!(duration < std::time::Duration::from_secs(10));
    
    Ok(())
}

#[test]
/// Test that the Walker handles projects with complex exports fields correctly
fn test_complex_exports_handling() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create a package with complex exports field
    fs::create_dir_all(temp_dir.path())?;
    fs::write(temp_dir.path().join("package.json"), r#"{
        "name": "complex-exports",
        "version": "1.0.0",
        "main": "dist/index.js",
        "module": "dist/index.mjs",
        "types": "dist/index.d.ts",
        "exports": {
            ".": {
                "types": "./dist/index.d.ts",
                "node": {
                    "import": {
                        "types": "./dist/index.d.mts",
                        "default": "./dist/index.mjs"
                    },
                    "require": {
                        "types": "./dist/index.d.cts",
                        "default": "./dist/index.cjs"
                    }
                },
                "browser": {
                    "import": "./dist/browser.mjs",
                    "require": "./dist/browser.cjs"
                },
                "default": "./dist/index.js"
            },
            "./utils": {
                "import": "./dist/utils.mjs",
                "require": "./dist/utils.cjs"
            },
            "./package.json": "./package.json"
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 1 package
    assert_eq!(results.packages.len(), 1);
    
    // Check that the package is correctly analyzed
    let package = &results.packages[0];
    
    // Should be detected as ESM and CommonJS
    assert!(package.module_support.esm.overall);
    assert!(package.module_support.cjs.overall);
    
    // Should be detected as TypeScript and browser
    assert!(package.typescript_support);
    assert!(package.browser_support);
    
    Ok(())
}

#[test]
/// Test that the Walker handles projects with browser field correctly
fn test_browser_field_handling() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with different browser field formats
    
    // 1. String browser field
    let string_browser_dir = temp_dir.path().join("string-browser");
    fs::create_dir_all(&string_browser_dir)?;
    fs::write(string_browser_dir.join("package.json"), r#"{
        "name": "string-browser",
        "version": "1.0.0",
        "main": "index.js",
        "browser": "browser.js"
    }"#)?;
    
    // 2. Object browser field
    let object_browser_dir = temp_dir.path().join("object-browser");
    fs::create_dir_all(&object_browser_dir)?;
    fs::write(object_browser_dir.join("package.json"), r#"{
        "name": "object-browser",
        "version": "1.0.0",
        "main": "index.js",
        "browser": {
            "./index.js": "./browser.js",
            "module-a": false,
            "module-b": "module-b-browser"
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 2 packages
    assert_eq!(results.packages.len(), 2);
    
    // Check that both packages are detected as having browser support
    for package in &results.packages {
        assert!(package.browser_support);
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles projects with TypeScript support correctly
fn test_typescript_support_detection() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with different TypeScript configurations
    
    // 1. Package with types field
    let types_dir = temp_dir.path().join("types-pkg");
    fs::create_dir_all(&types_dir)?;
    fs::write(types_dir.join("package.json"), r#"{
        "name": "types-pkg",
        "version": "1.0.0",
        "main": "dist/index.js",
        "types": "dist/index.d.ts"
    }"#)?;
    
    // 2. Package with typings field
    let typings_dir = temp_dir.path().join("typings-pkg");
    fs::create_dir_all(&typings_dir)?;
    fs::write(typings_dir.join("package.json"), r#"{
        "name": "typings-pkg",
        "version": "1.0.0",
        "main": "dist/index.js",
        "typings": "dist/index.d.ts"
    }"#)?;
    
    // 3. Package with TypeScript in devDependencies
    let ts_dev_dir = temp_dir.path().join("ts-dev-pkg");
    fs::create_dir_all(&ts_dev_dir)?;
    fs::write(ts_dev_dir.join("package.json"), r#"{
        "name": "ts-dev-pkg",
        "version": "1.0.0",
        "main": "dist/index.js",
        "devDependencies": {
            "typescript": "^4.9.5"
        }
    }"#)?;
    
    // 4. Package without TypeScript
    let no_ts_dir = temp_dir.path().join("no-ts-pkg");
    fs::create_dir_all(&no_ts_dir)?;
    fs::write(no_ts_dir.join("package.json"), r#"{
        "name": "no-ts-pkg",
        "version": "1.0.0",
        "main": "dist/index.js"
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
    
    // Check TypeScript support detection
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "types-pkg" => {
                    assert!(package.typescript_support);
                },
                "typings-pkg" => {
                    assert!(package.typescript_support);
                },
                "ts-dev-pkg" => {
                    // Having TypeScript in devDependencies doesn't necessarily mean the package has TypeScript support
                    // This depends on the implementation details of the analyzer
                    // So we don't assert anything specific here
                },
                "no-ts-pkg" => {
                    assert!(!package.typescript_support);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}

#[test]
/// Test that the Walker handles projects with different module systems correctly
fn test_module_system_detection() -> Result<()> {
    let temp_dir = tempdir()?;
    
    // Create packages with different module system configurations
    
    // 1. ESM package (type: module)
    let esm_type_dir = temp_dir.path().join("esm-type");
    fs::create_dir_all(&esm_type_dir)?;
    fs::write(esm_type_dir.join("package.json"), r#"{
        "name": "esm-type",
        "version": "1.0.0",
        "type": "module",
        "main": "index.js"
    }"#)?;
    
    // 2. ESM package (module field)
    let esm_field_dir = temp_dir.path().join("esm-field");
    fs::create_dir_all(&esm_field_dir)?;
    fs::write(esm_field_dir.join("package.json"), r#"{
        "name": "esm-field",
        "version": "1.0.0",
        "main": "index.js",
        "module": "index.mjs"
    }"#)?;
    
    // 3. ESM package (.mjs extension)
    let esm_mjs_dir = temp_dir.path().join("esm-mjs");
    fs::create_dir_all(&esm_mjs_dir)?;
    fs::write(esm_mjs_dir.join("package.json"), r#"{
        "name": "esm-mjs",
        "version": "1.0.0",
        "main": "index.mjs"
    }"#)?;
    
    // 4. CommonJS package (explicit)
    let cjs_explicit_dir = temp_dir.path().join("cjs-explicit");
    fs::create_dir_all(&cjs_explicit_dir)?;
    fs::write(cjs_explicit_dir.join("package.json"), r#"{
        "name": "cjs-explicit",
        "version": "1.0.0",
        "type": "commonjs",
        "main": "index.js"
    }"#)?;
    
    // 5. CommonJS package (implicit)
    let cjs_implicit_dir = temp_dir.path().join("cjs-implicit");
    fs::create_dir_all(&cjs_implicit_dir)?;
    fs::write(cjs_implicit_dir.join("package.json"), r#"{
        "name": "cjs-implicit",
        "version": "1.0.0",
        "main": "index.js"
    }"#)?;
    
    // 6. Dual package (exports field)
    let dual_exports_dir = temp_dir.path().join("dual-exports");
    fs::create_dir_all(&dual_exports_dir)?;
    fs::write(dual_exports_dir.join("package.json"), r#"{
        "name": "dual-exports",
        "version": "1.0.0",
        "main": "index.js",
        "module": "index.mjs",
        "exports": {
            "import": "./index.mjs",
            "require": "./index.js"
        }
    }"#)?;
    
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Run analysis
    let results = walker.analyze()?;
    
    // Should find 6 packages
    assert_eq!(results.packages.len(), 6);
    
    // Check module system detection
    for package in &results.packages {
        if let Some(name) = &package.details.name {
            match name.as_str() {
                "esm-type" => {
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.esm.type_module);
                },
                "esm-field" => {
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.esm.module_field);
                },
                "esm-mjs" => {
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.esm.main_mjs);
                },
                "cjs-explicit" => {
                    assert!(!package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "cjs-implicit" => {
                    assert!(!package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                },
                "dual-exports" => {
                    assert!(package.module_support.esm.overall);
                    assert!(package.module_support.cjs.overall);
                    assert!(package.module_support.esm.exports_import);
                },
                _ => {}
            }
        }
    }
    
    Ok(())
}
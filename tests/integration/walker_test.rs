use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use walker::{
    core::walker::Walker,
    error::Result,
    models::config::Settings,
};

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
    
    // Deeply nested package
    let deep_pkg_dir = base_dir.join("packages").join("deep").join("nested").join("pkg");
    fs::create_dir_all(&deep_pkg_dir)?;
    fs::write(deep_pkg_dir.join("package.json"), r#"{
        "name": "deep-pkg",
        "version": "1.0.0"
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
fn test_walker_find_packages() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings with default exclude patterns
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Find packages
    let packages = walker.find_packages()?;
    
    // Should find 4 packages (root, pkg1, pkg2, deep-pkg)
    // but not the one in node_modules due to default exclude patterns
    assert_eq!(packages.len(), 4);
    
    // Check that we found the expected packages
    let package_names: Vec<String> = packages.iter()
        .map(|p| p.path.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    
    assert!(package_names.contains(&"root-package".to_string()) || 
            package_names.contains(&temp_dir.path().file_name().unwrap().to_string_lossy().to_string()));
    assert!(package_names.contains(&"pkg1".to_string()));
    assert!(package_names.contains(&"pkg2".to_string()));
    assert!(package_names.contains(&"pkg".to_string()));
    
    // Make sure we didn't find the package in node_modules
    assert!(!package_names.contains(&"some-dep".to_string()));
    
    Ok(())
}

#[test]
fn test_walker_with_max_depth() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings with max depth of 2
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.max_depth = Some(2);
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Find packages
    let packages = walker.find_packages()?;
    
    // Should find 3 packages (root, pkg1, pkg2)
    // but not the deeply nested one or the one in node_modules
    assert_eq!(packages.len(), 3);
    
    // Check that we found the expected packages
    let package_paths: Vec<PathBuf> = packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    // The deep package should not be found due to max_depth
    let deep_pkg_path = temp_dir.path().join("packages").join("deep").join("nested").join("pkg");
    assert!(!package_paths.contains(&deep_pkg_path));
    
    Ok(())
}

#[test]
fn test_walker_with_custom_exclude() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings with custom exclude patterns
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.exclude_patterns = vec![
        "node_modules".to_string(),
        "packages/pkg1".to_string(),  // Exclude pkg1
    ];
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Find packages
    let packages = walker.find_packages()?;
    
    // Check that pkg1 is not in the results
    let package_paths: Vec<PathBuf> = packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    let pkg1_path = temp_dir.path().join("packages").join("pkg1");
    assert!(!package_paths.contains(&pkg1_path));
    
    Ok(())
}

#[test]
fn test_walker_with_include_node_modules() -> Result<()> {
    let temp_dir = tempdir()?;
    create_test_project_structure(temp_dir.path())?;
    
    // Create settings without excluding node_modules
    let mut settings = Settings::default();
    settings.scan_path = temp_dir.path().to_path_buf();
    settings.exclude_patterns = vec![]; // No exclude patterns
    
    // Create walker
    let walker = Walker::new(settings);
    
    // Find packages
    let packages = walker.find_packages()?;
    
    // Should find all 5 packages including the one in node_modules
    assert_eq!(packages.len(), 5);
    
    // Check that we found the package in node_modules
    let package_paths: Vec<PathBuf> = packages.iter()
        .map(|p| p.path.clone())
        .collect();
    
    let nm_pkg_path = temp_dir.path().join("node_modules").join("some-dep");
    assert!(package_paths.contains(&nm_pkg_path));
    
    Ok(())
}
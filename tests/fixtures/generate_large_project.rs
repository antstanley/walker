use std::fs;
use std::path::{Path, PathBuf};
use std::error::Error;

/// Generate a large project structure for performance testing
///
/// This function creates a directory structure with many packages
/// to test the performance of the walker on large codebases.
///
/// # Arguments
///
/// * `base_dir` - The base directory where the project will be created
/// * `width` - The number of packages at each level
/// * `depth` - The maximum depth of the directory structure
///
/// # Returns
///
/// The number of packages created
pub fn generate_large_project(
    base_dir: &Path,
    width: usize,
    depth: usize,
) -> Result<usize, Box<dyn Error>> {
    // Create the base directory if it doesn't exist
    fs::create_dir_all(base_dir)?;
    
    // Create the root package.json
    fs::write(
        base_dir.join("package.json"),
        r#"{
  "name": "large-test-project",
  "version": "1.0.0",
  "private": true,
  "workspaces": [
    "packages/*"
  ]
}"#,
    )?;
    
    // Create packages directory
    let packages_dir = base_dir.join("packages");
    fs::create_dir_all(&packages_dir)?;
    
    // Generate packages recursively
    let count = generate_packages(&packages_dir, width, depth, 0)?;
    
    Ok(count)
}

/// Recursively generate packages
fn generate_packages(
    dir: &Path,
    width: usize,
    max_depth: usize,
    current_depth: usize,
) -> Result<usize, Box<dyn Error>> {
    if current_depth > max_depth {
        return Ok(0);
    }
    
    let mut count = 0;
    
    // Create packages at this level
    for i in 0..width {
        let pkg_name = format!("pkg-{}-{}", current_depth, i);
        let pkg_dir = dir.join(&pkg_name);
        fs::create_dir_all(&pkg_dir)?;
        
        // Create package.json
        let pkg_type = if i % 2 == 0 { "module" } else { "commonjs" };
        fs::write(
            pkg_dir.join("package.json"),
            format!(r#"{{
  "name": "{}",
  "version": "1.0.0",
  "type": "{}",
  "dependencies": {{
    "lodash": "^4.17.21"
  }}
}}"#, pkg_name, pkg_type),
        )?;
        
        count += 1;
        
        // Create nested packages if not at max depth
        if current_depth < max_depth {
            let nested_dir = pkg_dir.join("packages");
            fs::create_dir_all(&nested_dir)?;
            count += generate_packages(&nested_dir, width / 2, max_depth, current_depth + 1)?;
        }
    }
    
    Ok(count)
}

/// Create a test project with the specified structure
///
/// # Arguments
///
/// * `base_dir` - The base directory where the project will be created
///
/// # Returns
///
/// A vector of paths to the created packages
pub fn create_test_project(base_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    // Create the base directory if it doesn't exist
    fs::create_dir_all(base_dir)?;
    
    let mut package_paths = Vec::new();
    
    // Create root package
    fs::write(
        base_dir.join("package.json"),
        r#"{
  "name": "test-project",
  "version": "1.0.0",
  "private": true,
  "workspaces": [
    "packages/*"
  ]
}"#,
    )?;
    package_paths.push(base_dir.to_path_buf());
    
    // Create packages directory
    let packages_dir = base_dir.join("packages");
    fs::create_dir_all(&packages_dir)?;
    
    // Create ESM package
    let esm_dir = packages_dir.join("esm-package");
    fs::create_dir_all(&esm_dir)?;
    fs::write(
        esm_dir.join("package.json"),
        r#"{
  "name": "esm-package",
  "version": "1.0.0",
  "type": "module",
  "main": "index.js"
}"#,
    )?;
    package_paths.push(esm_dir);
    
    // Create CJS package
    let cjs_dir = packages_dir.join("cjs-package");
    fs::create_dir_all(&cjs_dir)?;
    fs::write(
        cjs_dir.join("package.json"),
        r#"{
  "name": "cjs-package",
  "version": "1.0.0",
  "type": "commonjs",
  "main": "index.js"
}"#,
    )?;
    package_paths.push(cjs_dir);
    
    // Create TypeScript package
    let ts_dir = packages_dir.join("ts-package");
    fs::create_dir_all(&ts_dir)?;
    fs::write(
        ts_dir.join("package.json"),
        r#"{
  "name": "ts-package",
  "version": "1.0.0",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "devDependencies": {
    "typescript": "^4.9.5"
  }
}"#,
    )?;
    package_paths.push(ts_dir);
    
    // Create dual-mode package
    let dual_dir = packages_dir.join("dual-package");
    fs::create_dir_all(&dual_dir)?;
    fs::write(
        dual_dir.join("package.json"),
        r#"{
  "name": "dual-package",
  "version": "1.0.0",
  "main": "index.cjs",
  "module": "index.mjs",
  "exports": {
    ".": {
      "import": "./index.mjs",
      "require": "./index.cjs"
    }
  }
}"#,
    )?;
    package_paths.push(dual_dir);
    
    // Create nested packages
    let nested_dir = packages_dir.join("nested");
    fs::create_dir_all(&nested_dir)?;
    
    for i in 1..=3 {
        let pkg_dir = nested_dir.join(format!("pkg{}", i));
        fs::create_dir_all(&pkg_dir)?;
        fs::write(
            pkg_dir.join("package.json"),
            format!(r#"{{
  "name": "nested-pkg{}",
  "version": "1.0.0"
}}"#, i),
        )?;
        package_paths.push(pkg_dir);
    }
    
    // Create node_modules with some packages
    let node_modules_dir = base_dir.join("node_modules");
    fs::create_dir_all(&node_modules_dir)?;
    
    for i in 1..=3 {
        let dep_dir = node_modules_dir.join(format!("dep{}", i));
        fs::create_dir_all(&dep_dir)?;
        fs::write(
            dep_dir.join("package.json"),
            format!(r#"{{
  "name": "dep{}",
  "version": "1.0.0"
}}"#, i),
        )?;
        // Don't add node_modules packages to the return list
        // as they should be excluded by default
    }
    
    Ok(package_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_create_test_project() {
        let temp_dir = tempdir().unwrap();
        let package_paths = create_test_project(temp_dir.path()).unwrap();
        
        // Should create 6 packages (root + 5 in packages dir)
        assert_eq!(package_paths.len(), 6);
        
        // Check that all package.json files exist
        for path in &package_paths {
            assert!(path.join("package.json").exists());
        }
    }
    
    #[test]
    fn test_generate_large_project() {
        let temp_dir = tempdir().unwrap();
        
        // Generate a small project for testing (3 packages at each level, 2 levels deep)
        let count = generate_large_project(temp_dir.path(), 3, 2).unwrap();
        
        // Calculate expected count: 1 (root) + 3 (level 1) + 3*1 (level 2, width/2=1)
        let expected = 1 + 3 + 3*1;
        assert_eq!(count, expected);
        
        // Check that the root package.json exists
        assert!(temp_dir.path().join("package.json").exists());
        
        // Check that the packages directory exists
        assert!(temp_dir.path().join("packages").exists());
    }
}
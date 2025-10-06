//! AST-based package analysis walker

use crate::error::{Result, WalkerError};
use crate::models::ast::{ASTAnalysisResults, AnalysisError, AnalysisErrorType};
use crate::models::package::PackageDetails;
use crate::parsers::dependency_graph_builder::{DependencyGraphBuilder, GraphBuilderConfig};
use crate::parsers::package_json::PackageJsonParser;
use std::path::{Path, PathBuf};

/// Configuration for AST analysis
#[derive(Debug, Clone)]
pub struct ASTWalkerConfig {
    pub follow_dynamic_imports: bool,
    pub include_node_modules: bool,
    pub max_depth: usize,
    pub ignore_patterns: Vec<String>,
}

impl Default for ASTWalkerConfig {
    fn default() -> Self {
        Self {
            follow_dynamic_imports: false,
            include_node_modules: false,
            max_depth: 100,
            ignore_patterns: vec![
                "**/*.test.js".to_string(),
                "**/*.spec.ts".to_string(),
                "**/test/**".to_string(),
                "**/__tests__/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/coverage/**".to_string(),
            ],
        }
    }
}

/// AST walker for package analysis
pub struct ASTWalker {
    config: ASTWalkerConfig,
}

impl ASTWalker {
    /// Create a new AST walker
    pub fn new(config: ASTWalkerConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(ASTWalkerConfig::default())
    }

    /// Analyze a package using AST parsing
    pub fn analyze_package(&self, package_path: &Path) -> Result<ASTAnalysisResults> {
        // Find entry points from package.json
        let entry_points = self.find_entry_points(package_path)?;

        if entry_points.is_empty() {
            return Err(WalkerError::InvalidPath {
                path: package_path.to_path_buf(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Build dependency graph configuration
        let graph_config = GraphBuilderConfig {
            follow_dynamic_imports: self.config.follow_dynamic_imports,
            max_depth: self.config.max_depth,
            include_node_modules: self.config.include_node_modules,
            ignore_patterns: self.config.ignore_patterns.clone(),
        };

        // Build dependency graph starting from entry points
        let builder = DependencyGraphBuilder::new(package_path, graph_config);
        let dependency_graph = builder.build(entry_points.clone())?;

        // Create analysis results
        let mut results = ASTAnalysisResults::new(package_path.to_path_buf());

        // Add files from dependency graph
        for (path, node) in &dependency_graph.nodes {
            let mut metadata = node.metadata.clone();

            // Update reference information
            metadata.is_referenced = dependency_graph.reachable_files.contains(path);
            metadata.reference_count = node.dependents.len();

            results.add_file(metadata);
        }

        // Collect errors before setting the dependency graph
        let mut errors_to_add = Vec::new();

        // Collect unresolved import errors
        for (path, unresolved_imports) in &dependency_graph.unresolved_imports {
            for import in unresolved_imports {
                errors_to_add.push(AnalysisError {
                    file_path: path.clone(),
                    error_type: AnalysisErrorType::UnresolvedImport,
                    message: format!("Cannot resolve import: {}", import),
                    import_chain: vec![path.clone()],
                    suggested_fix: Some(format!(
                        "Check if '{}' is installed or if the path is correct",
                        import
                    )),
                });
            }
        }

        // Collect circular dependency errors
        for (from, to) in &dependency_graph.circular_dependencies {
            errors_to_add.push(AnalysisError {
                file_path: from.clone(),
                error_type: AnalysisErrorType::CircularDependency,
                message: format!("Circular dependency detected: {} -> {}",
                    from.display(), to.display()),
                import_chain: vec![from.clone(), to.clone()],
                suggested_fix: Some("Consider refactoring to break the circular dependency".to_string()),
            });
        }

        // Set the dependency graph
        results.dependency_graph = dependency_graph;

        // Add collected errors
        for error in errors_to_add {
            results.add_error(error);
        }

        // Finalize the results
        results.finalize();

        Ok(results)
    }

    /// Find entry points from package.json
    fn find_entry_points(&self, package_path: &Path) -> Result<Vec<PathBuf>> {
        let package_json_path = package_path.join("package.json");

        if !package_json_path.exists() {
            return Err(WalkerError::PackageJsonNotFound {
                path: package_path.to_path_buf(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        let details = PackageJsonParser::parse_file(&package_json_path)?;
        let mut entry_points = Vec::new();

        // Add main entry
        if let Some(main) = &details.main {
            let main_path = self.resolve_entry_point(package_path, main);
            if let Some(path) = main_path {
                entry_points.push(path);
            }
        }

        // Add module entry
        if let Some(module) = &details.module {
            let module_path = self.resolve_entry_point(package_path, module);
            if let Some(path) = module_path {
                entry_points.push(path);
            }
        }

        // Add browser entry
        if let Some(serde_json::Value::String(browser)) = &details.browser {
            let browser_path = self.resolve_entry_point(package_path, browser);
            if let Some(path) = browser_path {
                entry_points.push(path);
            }
        }

        // Parse exports field for additional entry points
        if let Some(exports) = &details.exports {
            self.parse_exports_for_entry_points(exports, package_path, &mut entry_points);
        }

        // Add bin entries
        if let Some(bin) = &details.bin {
            self.parse_bin_for_entry_points(bin, package_path, &mut entry_points);
        }

        // Deduplicate entry points
        entry_points.sort();
        entry_points.dedup();

        // If no entry points found, try default locations
        if entry_points.is_empty() {
            let defaults = ["index.js", "index.ts", "src/index.js", "src/index.ts"];
            for default in &defaults {
                let path = package_path.join(default);
                if path.exists() {
                    entry_points.push(path);
                    break;
                }
            }
        }

        Ok(entry_points)
    }

    /// Resolve an entry point path
    fn resolve_entry_point(&self, package_path: &Path, entry: &str) -> Option<PathBuf> {
        let path = package_path.join(entry);

        // Try exact path
        if path.exists() {
            return Some(path);
        }

        // Try with common extensions
        let extensions = [".js", ".ts", ".mjs", ".cjs", ".jsx", ".tsx"];
        for ext in &extensions {
            let with_ext = package_path.join(format!("{}{}", entry, ext));
            if with_ext.exists() {
                return Some(with_ext);
            }
        }

        // Try as directory with index file
        let index_files = ["index.js", "index.ts", "index.mjs", "index.cjs"];
        for index in &index_files {
            let index_path = path.join(index);
            if index_path.exists() {
                return Some(index_path);
            }
        }

        None
    }

    /// Parse exports field for entry points
    fn parse_exports_for_entry_points(
        &self,
        exports: &serde_json::Value,
        package_path: &Path,
        entry_points: &mut Vec<PathBuf>,
    ) {
        match exports {
            serde_json::Value::String(path) => {
                if let Some(resolved) = self.resolve_entry_point(package_path, path) {
                    entry_points.push(resolved);
                }
            }
            serde_json::Value::Object(map) => {
                // Check for main export (.)
                if let Some(main_export) = map.get(".") {
                    self.parse_exports_for_entry_points(main_export, package_path, entry_points);
                }

                // Check conditional exports
                for key in ["import", "require", "default", "node", "browser"] {
                    if let Some(value) = map.get(key) {
                        if let serde_json::Value::String(path) = value {
                            if let Some(resolved) = self.resolve_entry_point(package_path, path) {
                                entry_points.push(resolved);
                            }
                        }
                    }
                }

                // Check subpath exports
                for (key, value) in map {
                    if key.starts_with("./") && key != "." {
                        self.parse_exports_for_entry_points(value, package_path, entry_points);
                    }
                }
            }
            _ => {}
        }
    }

    /// Parse bin field for entry points
    fn parse_bin_for_entry_points(
        &self,
        bin: &serde_json::Value,
        package_path: &Path,
        entry_points: &mut Vec<PathBuf>,
    ) {
        match bin {
            serde_json::Value::String(path) => {
                if let Some(resolved) = self.resolve_entry_point(package_path, path) {
                    entry_points.push(resolved);
                }
            }
            serde_json::Value::Object(map) => {
                for (_name, path) in map {
                    if let serde_json::Value::String(path_str) = path {
                        if let Some(resolved) = self.resolve_entry_point(package_path, path_str) {
                            entry_points.push(resolved);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_entry_points() {
        let dir = TempDir::new().unwrap();
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "main": "index.js",
            "module": "index.mjs"
        }"#;

        fs::write(dir.path().join("package.json"), package_json).unwrap();
        fs::write(dir.path().join("index.js"), "// main entry").unwrap();
        fs::write(dir.path().join("index.mjs"), "// module entry").unwrap();

        let walker = ASTWalker::with_defaults();
        let entry_points = walker.find_entry_points(dir.path()).unwrap();

        assert_eq!(entry_points.len(), 2);
        assert!(entry_points.contains(&dir.path().join("index.js")));
        assert!(entry_points.contains(&dir.path().join("index.mjs")));
    }

    #[test]
    fn test_analyze_package() {
        let dir = TempDir::new().unwrap();
        let package_json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "main": "index.js"
        }"#;

        fs::write(dir.path().join("package.json"), package_json).unwrap();
        fs::write(
            dir.path().join("index.js"),
            "export const test = 'hello';",
        ).unwrap();

        let walker = ASTWalker::with_defaults();
        let results = walker.analyze_package(dir.path()).unwrap();

        assert_eq!(results.package_path, dir.path());
        assert!(!results.files.is_empty());
    }
}
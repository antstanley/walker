//! Dependency graph construction from AST analysis

use crate::error::{Result, WalkerError};
use crate::models::dependency_graph::{ DependencyGraph, ImportType};
use crate::models::file_metadata::{FileLocation, FileMetadata, FileType, ModuleSystem};
use crate::parsers::ast_parser::ASTParser;
use crate::utils::path_resolver::PathResolver;
use dashmap::DashMap;
use std::collections::{HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Configuration for dependency graph building
#[derive(Debug, Clone)]
pub struct GraphBuilderConfig {
    pub follow_dynamic_imports: bool,
    pub max_depth: usize,
    pub include_node_modules: bool,
    pub ignore_patterns: Vec<String>,
}

impl Default for GraphBuilderConfig {
    fn default() -> Self {
        Self {
            follow_dynamic_imports: false,
            max_depth: 100,
            include_node_modules: false,
            ignore_patterns: vec![
                "**/*.test.js".to_string(),
                "**/*.spec.ts".to_string(),
                "**/test/**".to_string(),
                "**/__tests__/**".to_string(),
            ],
        }
    }
}

/// Builds dependency graphs from package entry points
pub struct DependencyGraphBuilder {
    package_root: PathBuf,
    resolver: Arc<PathResolver>,
    files: Arc<DashMap<PathBuf, FileMetadata>>,
    visited_paths: Arc<DashMap<PathBuf, ()>>,
    circular_deps: Arc<Mutex<HashSet<(PathBuf, PathBuf)>>>,
    parser: Arc<ASTParser>,
    config: GraphBuilderConfig,
}

impl DependencyGraphBuilder {
    /// Create a new dependency graph builder
    pub fn new(package_root: &Path, config: GraphBuilderConfig) -> Self {
        Self {
            package_root: package_root.to_path_buf(),
            resolver: Arc::new(PathResolver::new(package_root)),
            files: Arc::new(DashMap::new()),
            visited_paths: Arc::new(DashMap::new()),
            circular_deps: Arc::new(Mutex::new(HashSet::new())),
            parser: Arc::new(ASTParser::new()),
            config,
        }
    }

    /// Build dependency graph from entry points
    pub fn build(&self, entry_points: Vec<PathBuf>) -> Result<DependencyGraph> {
        // Process entry points sequentially due to allocator constraints
        for entry in &entry_points {
            self.traverse_file(entry, Vec::new())?;
        }

        // Build the graph from collected data
        let mut graph = self.create_graph(entry_points)?;

        // Calculate reachability
        graph.calculate_reachability();

        Ok(graph)
    }

    /// Traverse a file and its dependencies
    fn traverse_file(&self, file_path: &Path, import_chain: Vec<PathBuf>) -> Result<()> {
        // Check for circular dependency
        if import_chain.contains(&file_path.to_path_buf()) {
            if let Some(parent) = import_chain.last() {
                let mut circular_deps = self.circular_deps.lock().unwrap();
                circular_deps.insert((parent.clone(), file_path.to_path_buf()));
            }
            return Ok(());
        }

        // Avoid re-processing files
        if self.visited_paths.contains_key(file_path) {
            return Ok(());
        }
        self.visited_paths.insert(file_path.to_path_buf(), ());

        // Check if file should be ignored
        if self.should_ignore(file_path) {
            return Ok(());
        }

        // Parse and analyze the file
        let analysis = self.parser.parse_and_analyze(file_path)?;

        // Create file metadata
        let metadata = self.create_file_metadata(file_path, &analysis)?;

        // Store metadata
        self.files.insert(file_path.to_path_buf(), metadata);

        // Update import chain
        let mut new_chain = import_chain;
        new_chain.push(file_path.to_path_buf());

        // Check depth limit
        if new_chain.len() >= self.config.max_depth {
            return Ok(());
        }

        // Process imports
        let imports_to_resolve: Vec<_> = analysis
            .imports
            .iter()
            .filter(|imp| !imp.is_dynamic || self.config.follow_dynamic_imports)
            .map(|imp| imp.source.clone())
            .collect();

        // Resolve and traverse imports
        for source in imports_to_resolve {
            if let Ok(Some(resolved_path)) = self.resolver.resolve(&source, file_path) {
                // Skip node_modules unless configured to include
                if !self.config.include_node_modules && self.is_node_module(&resolved_path) {
                    continue;
                }

                // Recursively traverse
                self.traverse_file(&resolved_path, new_chain.clone())?;
            }
        }

        Ok(())
    }

    /// Create file metadata from analysis
    fn create_file_metadata(
        &self,
        path: &Path,
        analysis: &crate::parsers::ast_parser::FileAnalysis,
    ) -> Result<FileMetadata> {
        let file_meta = fs::metadata(path).map_err(|e| WalkerError::Io {
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        let relative_path = path
            .strip_prefix(&self.package_root)
            .unwrap_or(path)
            .to_path_buf();

        // Determine if file is in node_modules
        let file_location = if self.is_node_module(path) {
            let package_name = self.extract_package_name_from_path(path);
            FileLocation::Dependency(package_name)
        } else {
            FileLocation::CorePackage
        };

        Ok(FileMetadata {
            relative_path,
            absolute_path: path.to_path_buf(),
            file_name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_type: FileType::from_path(path),
            size_bytes: file_meta.len(),
            created: file_meta.created().ok(),
            modified: file_meta.modified().ok(),
            module_system: analysis.module_system.clone(),
            file_location,
            exports: analysis.exports.clone(),
            imports: analysis.imports.clone(),
            is_referenced: false, // Will be updated later
            reference_count: 0,   // Will be updated later
        })
    }

    /// Create the dependency graph from collected data
    fn create_graph(&self, entry_points: Vec<PathBuf>) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();

        // Add entry points
        for entry in entry_points {
            graph.add_entry_point(entry);
        }

        // Add all nodes
        for entry in self.files.iter() {
            let path = entry.key().clone();
            let metadata = entry.value().clone();
            graph.add_node(path, metadata);
        }

        // Add edges based on imports
        for entry in self.files.iter() {
            let from = entry.key();
            let metadata = entry.value();

            for import in &metadata.imports {
                if let Ok(Some(to)) = self.resolver.resolve(&import.source, from) {
                    if self.files.contains_key(&to) {
                        let import_type = if import.is_dynamic {
                            ImportType::DynamicImport
                        } else {
                            match metadata.module_system {
                                ModuleSystem::CommonJS => ImportType::Require,
                                _ => ImportType::StaticImport,
                            }
                        };

                        graph.add_edge(from.clone(), to, import_type);
                    }
                }
            }
        }

        // Add circular dependencies
        let circular_deps = self.circular_deps.lock().unwrap();
        for (from, to) in circular_deps.iter() {
            graph.add_circular_dependency(from.clone(), to.clone());
        }

        // Add unresolved imports
        for entry in self.files.iter() {
            let path = entry.key();
            let metadata = entry.value();

            for import in &metadata.imports {
                if self.resolver.resolve(&import.source, path).ok().flatten().is_none() {
                    graph.add_unresolved_import(path.clone(), import.source.clone());
                }
            }
        }

        Ok(graph)
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.ignore_patterns {
            if glob::Pattern::new(pattern)
                .map(|p| p.matches(&path_str))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    /// Check if a path is in node_modules
    fn is_node_module(&self, path: &Path) -> bool {
        path.components()
            .any(|c| c.as_os_str() == "node_modules")
    }

    /// Extract package name from a node_modules path
    fn extract_package_name_from_path(&self, path: &Path) -> String {
        let components: Vec<_> = path.components().collect();

        // Find node_modules index
        let nm_index = components
            .iter()
            .position(|c| c.as_os_str() == "node_modules");

        if let Some(idx) = nm_index {
            if idx + 1 < components.len() {
                let name = components[idx + 1].as_os_str().to_string_lossy();

                // Handle scoped packages
                if name.starts_with('@') && idx + 2 < components.len() {
                    let scope = name;
                    let package = components[idx + 2].as_os_str().to_string_lossy();
                    return format!("{}/{}", scope, package);
                }

                return name.to_string();
            }
        }

        "unknown".to_string()
    }

    /// Find all JavaScript/TypeScript files in a directory
    pub fn find_all_js_files(&self, dir: &Path) -> Result<HashSet<PathBuf>> {
        let mut files = HashSet::new();
        self.find_js_files_recursive(dir, &mut files)?;
        Ok(files)
    }

    fn find_js_files_recursive(&self, dir: &Path, files: &mut HashSet<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir).map_err(|e| WalkerError::Io {
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })? {
            let entry = entry.map_err(|e| WalkerError::Io {
                source: e,
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            })?;

            let path = entry.path();

            if path.is_dir() {
                // Skip node_modules unless configured
                if !self.config.include_node_modules && path.file_name() == Some("node_modules".as_ref()) {
                    continue;
                }
                self.find_js_files_recursive(&path, files)?;
            } else {
                let file_type = FileType::from_path(&path);
                if file_type.is_javascript() || file_type.is_typescript() {
                    if !self.should_ignore(&path) {
                        files.insert(path);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_dependency_graph_builder() {
        let dir = TempDir::new().unwrap();
        let config = GraphBuilderConfig::default();
        let builder = DependencyGraphBuilder::new(dir.path(), config);

        // Create test files
        let index_path = dir.path().join("index.js");
        fs::write(&index_path, "import './utils.js';").unwrap();

        let utils_path = dir.path().join("utils.js");
        fs::write(&utils_path, "export const helper = () => {};").unwrap();

        // Build graph
        let graph = builder.build(vec![index_path]).unwrap();

        assert_eq!(graph.nodes.len(), 2);
        assert!(!graph.edges.is_empty());
    }
}

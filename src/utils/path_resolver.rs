//! Module path resolution utilities

use crate::error::{Result, WalkerError};
use crate::models::package::PackageDetails;
use crate::parsers::package_json::PackageJsonParser;
use dashmap::DashMap;
use lru::LruCache;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;

/// TypeScript configuration for path resolution
#[derive(Debug, Clone)]
pub struct TsConfig {
    pub base_url: Option<PathBuf>,
    pub paths: HashMap<String, Vec<String>>,
    pub module_resolution: ModuleResolution,
}

/// Module resolution strategy
#[derive(Debug, Clone)]
pub enum ModuleResolution {
    Node,
    Bundler,
    Classic,
}

/// Path resolver with caching and TypeScript support
pub struct PathResolver {
    package_root: PathBuf,
    node_modules_cache: Arc<DashMap<String, PathBuf>>,
    tsconfig: Option<TsConfig>,
    file_exists_cache: Arc<DashMap<PathBuf, bool>>,
    package_json_cache: Arc<Mutex<LruCache<PathBuf, PackageDetails>>>,
}

impl PathResolver {
    /// Create a new path resolver
    pub fn new(package_root: &Path) -> Self {
        let tsconfig = Self::load_tsconfig(package_root);

        Self {
            package_root: package_root.to_path_buf(),
            node_modules_cache: Arc::new(DashMap::new()),
            tsconfig,
            file_exists_cache: Arc::new(DashMap::new()),
            package_json_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap()))),
        }
    }

    /// Load TypeScript configuration if available
    fn load_tsconfig(package_root: &Path) -> Option<TsConfig> {
        let tsconfig_path = package_root.join("tsconfig.json");
        if !tsconfig_path.exists() {
            return None;
        }

        let content = fs::read_to_string(&tsconfig_path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;

        let compiler_options = json.get("compilerOptions")?;

        let base_url = compiler_options
            .get("baseUrl")
            .and_then(|v| v.as_str())
            .map(|s| package_root.join(s));

        let paths = compiler_options
            .get("paths")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(key, value)| {
                        let paths = value.as_array()?
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                        Some((key.clone(), paths))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let module_resolution = compiler_options
            .get("moduleResolution")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "bundler" => ModuleResolution::Bundler,
                "classic" => ModuleResolution::Classic,
                _ => ModuleResolution::Node,
            })
            .unwrap_or(ModuleResolution::Node);

        Some(TsConfig {
            base_url,
            paths,
            module_resolution,
        })
    }

    /// Resolve a module specifier to an absolute path
    pub fn resolve(&self, specifier: &str, from: &Path) -> Result<Option<PathBuf>> {
        // Handle TypeScript path mappings first
        if let Some(tsconfig) = &self.tsconfig {
            if let Some(resolved) = self.resolve_typescript_paths(specifier, tsconfig)? {
                return Ok(Some(resolved));
            }
        }

        // Handle different import types
        match specifier.chars().next() {
            Some('.') => self.resolve_relative(specifier, from),
            Some('#') => self.resolve_subpath_import(specifier, from),
            Some('@') | Some(_) => self.resolve_package(specifier, from),
            None => Ok(None),
        }
    }

    /// Resolve relative imports
    fn resolve_relative(&self, specifier: &str, from: &Path) -> Result<Option<PathBuf>> {
        let base = from.parent().unwrap_or(from);
        let candidate = base.join(specifier);

        // Check exact path first
        if self.check_file_exists(&candidate) {
            return Ok(Some(candidate));
        }

        // Try adding extensions
        let extensions = [
            "ts", "tsx", "d.ts", // TypeScript files first
            "js", "jsx",         // JavaScript files
            "mjs", "cjs",        // ES/CommonJS modules
            "json",              // JSON imports
        ];

        let candidate_str = candidate.to_string_lossy();
        if !extensions.iter().any(|ext| candidate_str.ends_with(ext)) {
            for ext in &extensions {
                let with_ext = PathBuf::from(format!("{}.{}", candidate_str, ext));
                if self.check_file_exists(&with_ext) {
                    return Ok(Some(with_ext));
                }
            }
        }

        // Try as directory with index files
        if candidate.is_dir() {
            let index_files = [
                "index.ts", "index.tsx", "index.d.ts",
                "index.js", "index.jsx",
                "index.mjs", "index.cjs",
            ];

            for index_file in &index_files {
                let index_path = candidate.join(index_file);
                if self.check_file_exists(&index_path) {
                    return Ok(Some(index_path));
                }
            }
        }

        Ok(None)
    }

    /// Resolve package imports
    fn resolve_package(&self, specifier: &str, from: &Path) -> Result<Option<PathBuf>> {
        let (package_name, subpath) = self.split_package_specifier(specifier);

        // Check cache first
        if let Some(cached) = self.node_modules_cache.get(&package_name) {
            let package_dir = cached.clone();
            return self.resolve_package_subpath(&package_dir, subpath.as_deref());
        }

        // Walk up directory tree looking for node_modules
        let mut current = from.parent();

        while let Some(dir) = current {
            let node_modules = dir.join("node_modules").join(&package_name);

            if node_modules.exists() {
                // Cache the package location
                self.node_modules_cache.insert(package_name.clone(), node_modules.clone());
                return self.resolve_package_subpath(&node_modules, subpath.as_deref());
            }

            current = dir.parent();
        }

        Ok(None)
    }

    /// Resolve package subpath
    fn resolve_package_subpath(
        &self,
        package_dir: &Path,
        subpath: Option<&str>,
    ) -> Result<Option<PathBuf>> {
        let package_json_path = package_dir.join("package.json");

        if !self.check_file_exists(&package_json_path) {
            return Ok(None);
        }

        // Get package.json from cache or parse it
        let details = {
            let mut cache = self.package_json_cache.lock();
            if let Some(cached) = cache.get(&package_json_path) {
                cached.clone()
            } else {
                let details = PackageJsonParser::parse_file(&package_json_path)?;
                cache.put(package_json_path.clone(), details.clone());
                details
            }
        };

        // Handle exports field if present
        if let Some(exports) = &details.exports {
            if let Some(resolved) = self.resolve_exports_field(
                exports,
                subpath,
                package_dir,
                &details,
            )? {
                return Ok(Some(resolved));
            }
        }

        // If no exports field or subpath, try traditional resolution
        if let Some(subpath) = subpath {
            // Direct subpath resolution
            self.resolve_relative(&format!("./{}", subpath), package_dir)
        } else {
            // Resolve main entry point
            self.resolve_package_main(&details, package_dir)
        }
    }

    /// Resolve exports field
    fn resolve_exports_field(
        &self,
        exports: &Value,
        subpath: Option<&str>,
        package_dir: &Path,
        _details: &PackageDetails,
    ) -> Result<Option<PathBuf>> {
        match exports {
            Value::String(path) if subpath.is_none() => {
                // Simple string export for main entry
                Ok(Some(package_dir.join(path)))
            }
            Value::Object(map) => {
                // Conditional exports or subpath exports
                let export_key = subpath.map(|s| format!("./{}", s))
                    .unwrap_or_else(|| ".".to_string());

                // Look for exact match
                if let Some(export_value) = map.get(&export_key) {
                    return self.resolve_export_value(export_value, package_dir);
                }

                // Check conditional exports (import, require, default)
                if subpath.is_none() {
                    for condition in ["import", "require", "node", "default"] {
                        if let Some(value) = map.get(condition) {
                            if let Some(resolved) = self.resolve_export_value(value, package_dir)? {
                                return Ok(Some(resolved));
                            }
                        }
                    }
                }

                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Resolve export value
    fn resolve_export_value(
        &self,
        value: &Value,
        package_dir: &Path,
    ) -> Result<Option<PathBuf>> {
        match value {
            Value::String(path) => {
                let resolved = package_dir.join(path);
                if self.check_file_exists(&resolved) {
                    Ok(Some(resolved))
                } else {
                    Ok(None)
                }
            }
            Value::Object(conditions) => {
                // Nested conditions
                for condition in ["import", "require", "node", "default"] {
                    if let Some(path) = conditions.get(condition) {
                        if let Some(resolved) = self.resolve_export_value(path, package_dir)? {
                            return Ok(Some(resolved));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Resolve package main entry
    fn resolve_package_main(
        &self,
        details: &PackageDetails,
        package_dir: &Path,
    ) -> Result<Option<PathBuf>> {
        // Priority: module > main > index.js
        let entry_fields = [
            details.module.as_deref(),
            details.main.as_deref(),
        ];

        for field in entry_fields.iter().flatten() {
            let entry_path = package_dir.join(field);
            if self.check_file_exists(&entry_path) {
                return Ok(Some(entry_path));
            }

            // Try with extensions
            if let Some(resolved) = self.resolve_relative(field, package_dir)? {
                return Ok(Some(resolved));
            }
        }

        // Default to index.js
        let index = package_dir.join("index.js");
        if self.check_file_exists(&index) {
            Ok(Some(index))
        } else {
            // Try other index files
            self.resolve_relative("./index", package_dir)
        }
    }

    /// Resolve subpath imports (#imports)
    fn resolve_subpath_import(&self, _specifier: &str, _from: &Path) -> Result<Option<PathBuf>> {
        // TODO: Implement Node.js subpath imports
        Ok(None)
    }

    /// Resolve TypeScript paths
    fn resolve_typescript_paths(
        &self,
        specifier: &str,
        tsconfig: &TsConfig,
    ) -> Result<Option<PathBuf>> {
        // Check path mappings
        for (pattern, replacements) in &tsconfig.paths {
            if self.matches_path_pattern(specifier, pattern) {
                for replacement in replacements {
                    if let Some(path) = self.apply_path_mapping(specifier, pattern, replacement, tsconfig)? {
                        if self.check_file_exists(&path) {
                            return Ok(Some(path));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Check if specifier matches path pattern
    fn matches_path_pattern(&self, specifier: &str, pattern: &str) -> bool {
        if pattern.contains('*') {
            let prefix = pattern.split('*').next().unwrap_or("");
            let suffix = pattern.split('*').last().unwrap_or("");
            specifier.starts_with(prefix) && specifier.ends_with(suffix)
        } else {
            specifier == pattern
        }
    }

    /// Apply path mapping
    fn apply_path_mapping(
        &self,
        specifier: &str,
        pattern: &str,
        replacement: &str,
        tsconfig: &TsConfig,
    ) -> Result<Option<PathBuf>> {
        let base = tsconfig.base_url.as_ref()
            .unwrap_or(&self.package_root);

        if pattern.contains('*') && replacement.contains('*') {
            let prefix = pattern.split('*').next().unwrap_or("");
            let wildcard = &specifier[prefix.len()..];
            let replaced = replacement.replace('*', wildcard);
            Ok(Some(base.join(replaced)))
        } else {
            Ok(Some(base.join(replacement)))
        }
    }

    /// Check if file exists with caching
    fn check_file_exists(&self, path: &Path) -> bool {
        if let Some(cached) = self.file_exists_cache.get(path) {
            return *cached;
        }

        let exists = path.exists();
        self.file_exists_cache.insert(path.to_path_buf(), exists);
        exists
    }

    /// Split package specifier into name and subpath
    fn split_package_specifier(&self, specifier: &str) -> (String, Option<String>) {
        if specifier.starts_with('@') {
            // Scoped package
            let parts: Vec<&str> = specifier.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let package_name = format!("{}/{}", parts[0], parts[1]);
                let subpath = if parts.len() == 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                (package_name, subpath)
            } else {
                (specifier.to_string(), None)
            }
        } else {
            // Regular package
            let parts: Vec<&str> = specifier.splitn(2, '/').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), Some(parts[1].to_string()))
            } else {
                (specifier.to_string(), None)
            }
        }
    }
}
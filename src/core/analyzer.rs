//! Package analysis functionality
//!
//! This module provides comprehensive package analysis with caching support,
//! extracting detailed information from package.json files.

use crate::core::cache::{ThreadSafeCache};
use crate::error::{Result, WalkerError};
use crate::models::{analysis::PackageAnalysis, package::PackageDetails};
use crate::parsers::{package_json::PackageJsonParser};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Package analyzer for extracting package information
pub struct Analyzer {
    cache: Option<Arc<ThreadSafeCache>>,
    calculate_size: bool,
}

impl Analyzer {
    /// Create a new analyzer with optional caching
    pub fn new(cache_enabled: bool, calculate_size: bool) -> Self {
        let cache = if cache_enabled {
            Some(Arc::new(ThreadSafeCache::new()))
        } else {
            None
        };

        Self {
            cache,
            calculate_size,
        }
    }

    /// Analyze a package at the given path
    pub fn analyze_package(path: &Path) -> Result<PackageAnalysis> {
        // Find package.json file
        let package_json_path = path.join("package.json");

        // Check if package.json exists
        if !package_json_path.exists() {
            return Err(WalkerError::PackageJsonNotFound {
                path: path.to_path_buf(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }

        // Parse package.json
        let details = PackageJsonParser::parse_file(&package_json_path)?;

        // Create package analysis
        let analysis = PackageAnalysis::new(path.to_path_buf(), details);

        Ok(analysis)
    }

    /// Analyze a package with caching and size calculation options
    pub fn analyze_package_with_options(&self, path: &Path) -> Result<PackageAnalysis> {
        // Check cache first if enabled
        if let Some(cache_arc) = &self.cache {
            // Try to get the cached result
            if let Ok(Some(cached)) = cache_arc.get(&path.to_path_buf()) {
                return Ok(cached);
            }
        }

        // Analyze the package
        let mut analysis = Self::analyze_package(path)?;

        // Calculate size if requested
        if self.calculate_size {
            // Ignore size calculation errors
            let _ = analysis.calculate_size();
        }

        // Cache the result if caching is enabled
        if let Some(cache_arc) = &self.cache {
            // Insert the analysis into the cache
            let _ = cache_arc.insert(path.to_path_buf(), analysis.clone());
        }

        Ok(analysis)
    }

    /// Parse package.json content
    pub fn parse_package_json(content: &str) -> Result<PackageDetails> {
        PackageJsonParser::parse(content)
    }

    /// Find all package.json files in a directory recursively
    pub fn find_package_json_files(
        dir: &Path,
        exclude_patterns: &[String],
        max_depth: Option<usize>,
    ) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        Self::find_package_json_files_recursive(dir, &mut result, exclude_patterns, max_depth, 0)?;
        Ok(result)
    }

    /// Recursive helper for finding package.json files
    fn find_package_json_files_recursive(
        dir: &Path,
        result: &mut Vec<PathBuf>,
        exclude_patterns: &[String],
        max_depth: Option<usize>,
        current_depth: usize,
    ) -> Result<()> {
        // Check max depth
        if let Some(max) = max_depth {
            if current_depth > max {
                return Ok(());
            }
        }

        // Check if this directory should be excluded
        let dir_str = dir.to_string_lossy();
        for pattern in exclude_patterns {
            if glob::Pattern::new(pattern)
                .map_err(|e| WalkerError::GlobPattern {
                    source: e,
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                })?
                .matches(&dir_str)
            {
                return Ok(());
            }
        }

        // Check for package.json in this directory
        let package_json_path = dir.join("package.json");
        if package_json_path.exists() {
            result.push(dir.to_path_buf());
        }

        // Recursively check subdirectories
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let _ = Self::find_package_json_files_recursive(
                        &path,
                        result,
                        exclude_patterns,
                        max_depth,
                        current_depth + 1,
                    );
                }
            }
        }

        Ok(())
    }

    /// Analyze multiple packages with caching and progress reporting
    pub fn analyze_packages<F>(
        &self,
        paths: &[PathBuf],
        progress_fn: Option<F>,
    ) -> Result<HashMap<PathBuf, Result<PackageAnalysis>>>
    where
        F: Fn(usize, usize, &str),
    {
        let mut results = HashMap::new();
        let total = paths.len();

        for (i, path) in paths.iter().enumerate() {
            // Report progress if a progress function is provided
            if let Some(ref progress) = progress_fn {
                progress(i, total, &format!("Analyzing package: {}", path.display()));
            }

            // Analyze the package and store the result (or error)
            let result = self.analyze_package_with_options(path);
            results.insert(path.clone(), result);
        }

        // Report completion if a progress function is provided
        if let Some(ref progress) = progress_fn {
            progress(total, total, "Analysis complete");
        }

        Ok(results)
    }

    /// Get the cache if enabled
    pub fn cache(&self) -> Option<&Arc<ThreadSafeCache>> {
        self.cache.as_ref()
    }

    /// Clear the cache if enabled
    pub fn clear_cache(&self) -> Result<()> {
        if let Some(cache_arc) = &self.cache {
            cache_arc.clear()
        } else {
            Ok(())
        }
    }

    /// Get the number of cached entries
    pub fn cache_size(&self) -> Result<usize> {
        if let Some(cache_arc) = &self.cache {
            cache_arc.len()
        } else {
            Ok(0)
        }
    }

    /// Check if a package is in the cache
    pub fn is_cached(&self, path: &Path) -> Result<bool> {
        if let Some(cache_arc) = &self.cache {
            match cache_arc.get(&path.to_path_buf()) {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Ok(false),
                Err(e) => Err(e),
            }
        } else {
            Ok(false)
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> Result<(usize, usize, usize)> {
        if let Some(cache_arc) = &self.cache {
            cache_arc.stats()
        } else {
            Ok((0, 0, 0))
        }
    }
}

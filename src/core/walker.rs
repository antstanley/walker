//! Directory walking functionality
//!
//! This module provides robust directory traversal with error handling,
//! pattern-based exclusion, and depth limiting.

use crate::core::analyzer::Analyzer;
use crate::core::cache::ThreadSafeCache;
use crate::error::{Result, WalkerError};
use crate::models::{analysis::AnalysisResults, config::Settings};
use glob::Pattern;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Main walker for directory traversal and analysis
pub struct Walker {
    settings: Settings,
    analyzer: Analyzer,
    errors: Vec<(PathBuf, WalkerError)>, // Track non-critical errors
}

impl Walker {
    /// Create a new walker with the given settings
    pub fn new(settings: Settings) -> Self {
        let analyzer = Analyzer::new(
            settings.cache_enabled,
            settings.calculate_size
        );
        
        Self { 
            settings,
            analyzer,
            errors: Vec::new(),
        }
    }

    /// Analyze packages in the configured directory
    pub fn analyze(&self) -> Result<AnalysisResults> {
        let start_time = Instant::now();
        let mut results = AnalysisResults::new();
        
        // Check if the scan path exists
        if !self.settings.scan_path.exists() {
            return Err(WalkerError::InvalidPath {
                path: self.settings.scan_path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
        
        // Compile exclude patterns
        let exclude_patterns = match self.compile_exclude_patterns() {
            Ok(patterns) => patterns,
            Err(err) => {
                // Configuration errors are critical
                return Err(err);
            }
        };
        
        // Find all package.json files
        let package_dirs = match self.find_package_dirs(&exclude_patterns) {
            Ok(dirs) => dirs,
            Err(err) => {
                // Only fail if this is a critical error
                if err.is_critical() {
                    return Err(err);
                } else {
                    // For non-critical errors, log and continue with empty results
                    results.add_error(self.settings.scan_path.clone(), &err);
                    Vec::new()
                }
            }
        };
        
        // Add any collected errors during directory traversal
        for (path, err) in &self.errors {
            results.add_error(path.clone(), err);
        }
        
        // Analyze each package
        for dir in package_dirs {
            match self.analyze_package_dir(&dir) {
                Ok(analysis) => {
                    results.add_package(analysis);
                }
                Err(err) => {
                    // Add error to results and continue with next package
                    results.add_error(dir, &err);
                    
                    // If this is a critical error, stop processing
                    if err.is_critical() {
                        return Err(err);
                    }
                }
            }
        }
        
        // Set scan duration
        results.set_scan_duration(start_time.elapsed());
        
        Ok(results)
    }
    
    /// Compile exclude patterns into glob patterns
    fn compile_exclude_patterns(&self) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();
        
        for pattern_str in &self.settings.exclude_patterns {
            match Pattern::new(pattern_str) {
                Ok(pattern) => patterns.push(pattern),
                Err(err) => {
                    return Err(WalkerError::GlobPattern {
                        source: err,
                        #[cfg(not(tarpaulin_include))]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
            }
        }
        
        Ok(patterns)
    }
    
    /// Find all directories containing package.json files
    fn find_package_dirs(&self, exclude_patterns: &[Pattern]) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();
        let mut walker = Walker {
            settings: self.settings.clone(),
            analyzer: Analyzer::new(self.settings.cache_enabled, self.settings.calculate_size),
            errors: Vec::new(),
        };
        
        walker.find_package_dirs_recursive(
            &self.settings.scan_path,
            &mut result,
            exclude_patterns,
            0,
        )?;
        
        // Transfer any errors collected during traversal
        for (path, err) in walker.errors {
            self.errors.push((path, err));
        }
        
        Ok(result)
    }
    
    /// Recursively find directories containing package.json files
    fn find_package_dirs_recursive(
        &mut self,
        dir: &Path,
        result: &mut Vec<PathBuf>,
        exclude_patterns: &[Pattern],
        current_depth: usize,
    ) -> Result<()> {
        // Check max depth
        if let Some(max_depth) = self.settings.max_depth {
            if current_depth > max_depth {
                // Instead of returning an error, just stop recursion at this depth
                return Ok(());
            }
        }
        
        // Check if this directory should be excluded
        let dir_str = dir.to_string_lossy();
        for pattern in exclude_patterns {
            if pattern.matches(&dir_str) {
                return Ok(());
            }
        }
        
        // Check for package.json in this directory
        let package_json_path = dir.join("package.json");
        if package_json_path.exists() {
            result.push(dir.to_path_buf());
        }
        
        // Recursively check subdirectories
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry_result in entries {
                    match entry_result {
                        Ok(entry) => {
                            let path = entry.path();
                            if path.is_dir() {
                                // Follow symbolic links if configured
                                let should_follow = if path.is_symlink() {
                                    self.settings.follow_links
                                } else {
                                    true
                                };
                                
                                if should_follow {
                                    // Continue recursion, but handle errors gracefully
                                    if let Err(err) = self.find_package_dirs_recursive(
                                        &path,
                                        result,
                                        exclude_patterns,
                                        current_depth + 1,
                                    ) {
                                        // Store non-critical errors and continue
                                        if !err.is_critical() {
                                            self.errors.push((path.clone(), err));
                                        } else {
                                            // Critical errors should stop processing
                                            return Err(err);
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            // Handle permission denied errors gracefully
                            if err.kind() == std::io::ErrorKind::PermissionDenied {
                                // Store the error and continue
                                self.errors.push((
                                    dir.to_path_buf(),
                                    WalkerError::permission_denied(dir),
                                ));
                                continue;
                            } else {
                                // Store other IO errors and continue
                                self.errors.push((
                                    dir.to_path_buf(),
                                    WalkerError::io_error(err),
                                ));
                                continue;
                            }
                        }
                    }
                }
            }
            Err(err) => {
                // Handle permission denied errors gracefully
                if err.kind() == std::io::ErrorKind::PermissionDenied {
                    // Store the error and continue with other directories
                    self.errors.push((
                        dir.to_path_buf(),
                        WalkerError::permission_denied(dir),
                    ));
                    return Ok(());
                } else {
                    // Store other directory traversal errors
                    self.errors.push((
                        dir.to_path_buf(),
                        WalkerError::directory_traversal_error(
                            dir,
                            format!("Failed to read directory: {}", err),
                        ),
                    ));
                    return Ok(());
                }
            }
        }
        
        Ok(())
    }
    
    /// Analyze a package directory
    fn analyze_package_dir(&self, dir: &Path) -> Result<crate::models::analysis::PackageAnalysis> {
        // Use the analyzer with caching and size calculation options
        self.analyzer.analyze_package_with_options(dir)
    }

    /// Get the current settings
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    
    /// Get any non-critical errors that occurred during directory traversal
    pub fn errors(&self) -> &[(PathBuf, WalkerError)] {
        &self.errors
    }
    
    /// Check if a path matches any exclude pattern
    pub fn is_excluded(&self, path: &Path, patterns: &[Pattern]) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in patterns {
            if pattern.matches(&path_str) {
                return true;
            }
        }
        false
    }
    
    /// Analyze with progress reporting
    pub fn analyze_with_progress<F>(&self, progress_fn: F) -> Result<AnalysisResults>
    where
        F: Fn(usize, usize, &str),
    {
        let start_time = Instant::now();
        let mut results = AnalysisResults::new();
        
        // Check if the scan path exists
        if !self.settings.scan_path.exists() {
            return Err(WalkerError::InvalidPath {
                path: self.settings.scan_path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
        
        // Compile exclude patterns
        let exclude_patterns = match self.compile_exclude_patterns() {
            Ok(patterns) => patterns,
            Err(err) => {
                // Configuration errors are critical
                return Err(err);
            }
        };
        
        // Report progress: starting directory scan
        progress_fn(0, 0, &format!("Scanning directory: {}", self.settings.scan_path.display()));
        
        // Find all package.json files
        let package_dirs = match self.find_package_dirs(&exclude_patterns) {
            Ok(dirs) => dirs,
            Err(err) => {
                // Only fail if this is a critical error
                if err.is_critical() {
                    return Err(err);
                } else {
                    // For non-critical errors, log and continue with empty results
                    results.add_error(self.settings.scan_path.clone(), &err);
                    Vec::new()
                }
            }
        };
        
        // Add any collected errors during directory traversal
        for (path, err) in &self.errors {
            results.add_error(path.clone(), err);
        }
        
        // Report progress: found packages
        progress_fn(0, package_dirs.len(), &format!("Found {} packages", package_dirs.len()));
        
        // Analyze each package
        for (i, dir) in package_dirs.iter().enumerate() {
            // Report progress: analyzing package
            progress_fn(i, package_dirs.len(), &format!("Analyzing package: {}", dir.display()));
            
            match self.analyze_package_dir(dir) {
                Ok(analysis) => {
                    results.add_package(analysis);
                }
                Err(err) => {
                    // Add error to results and continue with next package
                    results.add_error(dir.clone(), &err);
                    
                    // If this is a critical error, stop processing
                    if err.is_critical() {
                        return Err(err);
                    }
                }
            }
        }
        
        // Set scan duration
        results.set_scan_duration(start_time.elapsed());
        
        // Report progress: completed
        progress_fn(package_dirs.len(), package_dirs.len(), "Analysis complete");
        
        Ok(results)
    }
}
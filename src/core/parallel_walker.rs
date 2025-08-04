//! Parallel directory walking functionality
//!
//! This module provides concurrent directory traversal with thread-safe progress reporting
//! and error collection, using rayon for parallel processing.

use crate::core::analyzer::Analyzer;
use crate::core::cache::ThreadSafeCache;
use crate::core::parallel::ProgressUpdate;
// Temporarily comment out the streaming implementation until we fix the compilation errors
// use crate::core::streaming::StreamingProcessor;
use crate::error::{Result, WalkerError};
use crate::models::{analysis::AnalysisResults, config::Settings};
use glob::Pattern;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Parallel walker for concurrent directory traversal and analysis
pub struct ParallelWalker {
    settings: Settings,
    analyzer: Analyzer,
    errors: Arc<Mutex<Vec<(PathBuf, WalkerError)>>>, // Thread-safe error collection
}

impl ParallelWalker {
    /// Create a new parallel walker with the given settings
    pub fn new(settings: Settings) -> Self {
        let analyzer = Analyzer::new(
            settings.cache_enabled,
            settings.calculate_size
        );
        
        Self { 
            settings,
            analyzer,
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Analyze packages in the configured directory using parallel processing
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
        
        // Find all package.json files in parallel
        let package_dirs = match self.find_package_dirs_parallel(&exclude_patterns) {
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
        if let Ok(errors) = self.errors.lock() {
            for (path, err) in errors.iter() {
                results.add_error(path.clone(), err);
            }
        }
        
        // Analyze each package in parallel
        if !package_dirs.is_empty() {
            // Check if we should use streaming mode for large result sets
            if self.settings.stream_results || package_dirs.len() > 1000 {
                // Process in batches to reduce memory usage
                let batch_size = self.settings.batch_size;
                let total_packages = package_dirs.len();
                let num_batches = (total_packages + batch_size - 1) / batch_size;
                
                for batch_idx in 0..num_batches {
                    let start_idx = batch_idx * batch_size;
                    let end_idx = std::cmp::min(start_idx + batch_size, total_packages);
                    
                    // Get current batch
                    let batch = &package_dirs[start_idx..end_idx];
                    
                    // Process batch in parallel
                    let batch_results: Vec<_> = batch.par_iter()
                        .map(|dir| {
                            match self.analyze_package_dir(dir) {
                                Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                                Err(err) => Some((dir.clone(), Err(err))),
                            }
                        })
                        .collect();
                    
                    // Process batch results
                    for result in batch_results {
                        if let Some((dir, analysis_result)) = result {
                            match analysis_result {
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
                    }
                    
                    // If memory usage is high, offload to disk (simplified version)
                    if results.packages.len() > self.settings.memory_limit_mb * 10 {
                        // In a real implementation, we would serialize to disk here
                        // For now, we'll just track that we've processed this batch
                        results.offloaded_files.push(PathBuf::from(format!("batch_{}", batch_idx)));
                    }
                }
                
                // Add any collected errors during directory traversal
                if let Ok(errors) = self.errors.lock() {
                    for (path, err) in errors.iter() {
                        results.add_error(path.clone(), err);
                    }
                }
                
                // Set scan duration
                results.set_scan_duration(start_time.elapsed());
                
                return Ok(results);
            } else {
                // Standard processing for smaller result sets
                let analyzed_packages: Vec<_> = package_dirs.par_iter()
                    .map(|dir| {
                        match self.analyze_package_dir(dir) {
                            Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                            Err(err) => Some((dir.clone(), Err(err))),
                        }
                    })
                    .collect();
                
                // Process analysis results
                for result in analyzed_packages {
                    if let Some((dir, analysis_result)) = result {
                        match analysis_result {
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
                }
            }
        }
        
        // Set scan duration
        results.set_scan_duration(start_time.elapsed());
        
        Ok(results)
    }
    
    /// Analyze with progress reporting
    pub fn analyze_with_progress<F>(&self, progress_callback: F) -> Result<AnalysisResults>
    where
        F: Fn(ProgressUpdate) + Send + Sync,
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
        
        // Report progress: starting
        progress_callback(ProgressUpdate::new(
            0, 
            0, 
            format!("Scanning directory: {}", self.settings.scan_path.display())
        ));
        
        // Compile exclude patterns
        let exclude_patterns = match self.compile_exclude_patterns() {
            Ok(patterns) => patterns,
            Err(err) => {
                // Configuration errors are critical
                return Err(err);
            }
        };
        
        // Find all package.json files in parallel with progress reporting
        let progress_counter = Arc::new(Mutex::new(0));
        let progress_callback_clone = Arc::new(progress_callback);
        
        let package_dirs = match self.find_package_dirs_parallel_with_progress(
            &exclude_patterns,
            progress_counter.clone(),
            progress_callback_clone.clone(),
        ) {
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
        if let Ok(errors) = self.errors.lock() {
            for (path, err) in errors.iter() {
                results.add_error(path.clone(), err);
            }
        }
        
        // Report progress: found packages
        progress_callback_clone(ProgressUpdate::new(
            0, 
            package_dirs.len(), 
            format!("Found {} packages", package_dirs.len())
        ));
        
        // Analyze each package in parallel with progress reporting
        if !package_dirs.is_empty() {
            // Check if we should use streaming mode for large result sets
            if self.settings.stream_results || package_dirs.len() > 1000 {
                // Process in batches to reduce memory usage
                let batch_size = self.settings.batch_size;
                let total_packages = package_dirs.len();
                let num_batches = (total_packages + batch_size - 1) / batch_size;
                
                let analysis_counter = Arc::new(Mutex::new(0));
                
                for batch_idx in 0..num_batches {
                    let start_idx = batch_idx * batch_size;
                    let end_idx = std::cmp::min(start_idx + batch_size, total_packages);
                    
                    // Get current batch
                    let batch = &package_dirs[start_idx..end_idx];
                    
                    // Report batch progress
                    progress_callback_clone(ProgressUpdate::new(
                        start_idx,
                        total_packages,
                        format!("Processing batch {}/{} ({} packages)", 
                            batch_idx + 1, 
                            num_batches,
                            batch.len()
                        )
                    ));
                    
                    // Process batch in parallel
                    let batch_results: Vec<_> = batch.par_iter()
                        .map(|dir| {
                            // Update progress
                            let current = {
                                let mut counter = match analysis_counter.lock() {
                                    Ok(guard) => guard,
                                    Err(_) => {
                                        return Some((dir.clone(), Err(WalkerError::ParallelExecution {
                                            message: "Failed to lock progress counter".to_string(),
                                            #[cfg(not(tarpaulin_include))]
                                            backtrace: std::backtrace::Backtrace::capture(),
                                        })));
                                    }
                                };
                                *counter += 1;
                                *counter
                            };
                            
                            // Report progress for individual packages
                            if current % 10 == 0 || current == total_packages {
                                progress_callback_clone(ProgressUpdate::new(
                                    current,
                                    total_packages,
                                    format!("Analyzing package: {}", dir.display())
                                ));
                            }
                            
                            // Analyze package
                            match self.analyze_package_dir(dir) {
                                Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                                Err(err) => Some((dir.clone(), Err(err))),
                            }
                        })
                        .collect();
                    
                    // Process batch results
                    for result in batch_results {
                        if let Some((dir, analysis_result)) = result {
                            match analysis_result {
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
                    }
                    
                    // If memory usage is high, offload to disk (simplified version)
                    if results.packages.len() > self.settings.memory_limit_mb * 10 {
                        // In a real implementation, we would serialize to disk here
                        // For now, we'll just track that we've processed this batch
                        results.offloaded_files.push(PathBuf::from(format!("batch_{}", batch_idx)));
                    }
                    
                    // Report batch completion
                    progress_callback_clone(ProgressUpdate::new(
                        end_idx,
                        total_packages,
                        format!("Completed batch {}/{}", batch_idx + 1, num_batches)
                    ));
                }
                
                // Add any collected errors during directory traversal
                if let Ok(errors) = self.errors.lock() {
                    for (path, err) in errors.iter() {
                        results.add_error(path.clone(), err);
                    }
                }
                
                // Set scan duration
                results.set_scan_duration(start_time.elapsed());
                
                // Report progress: completed
                progress_callback_clone(ProgressUpdate::new(
                    total_packages,
                    total_packages,
                    "Analysis complete".to_string()
                ));
                
                return Ok(results);
            } else {
                // Standard processing for smaller result sets
                let analysis_counter = Arc::new(Mutex::new(0));
                let total_packages = package_dirs.len();
                
                let analyzed_packages: Vec<_> = package_dirs.par_iter()
                    .map(|dir| {
                        // Update progress
                        let current = {
                            let mut counter = match analysis_counter.lock() {
                                Ok(guard) => guard,
                                Err(_) => {
                                    return Some((dir.clone(), Err(WalkerError::ParallelExecution {
                                        message: "Failed to lock progress counter".to_string(),
                                        #[cfg(not(tarpaulin_include))]
                                        backtrace: std::backtrace::Backtrace::capture(),
                                    })));
                                }
                            };
                            *counter += 1;
                            *counter
                        };
                        
                        // Report progress
                        progress_callback_clone(ProgressUpdate::new(
                            current,
                            total_packages,
                            format!("Analyzing package: {}", dir.display())
                        ));
                        
                        // Analyze package
                        match self.analyze_package_dir(dir) {
                            Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                            Err(err) => Some((dir.clone(), Err(err))),
                        }
                    })
                    .collect();
                
                // Process analysis results
                for result in analyzed_packages {
                    if let Some((dir, analysis_result)) = result {
                        match analysis_result {
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
                }
            }
        }
        
        // Set scan duration
        results.set_scan_duration(start_time.elapsed());
        
        // Report progress: completed
        progress_callback_clone(ProgressUpdate::new(
            package_dirs.len(),
            package_dirs.len(),
            "Analysis complete".to_string()
        ));
        
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
    
    /// Find all directories containing package.json files using parallel processing
    fn find_package_dirs_parallel(&self, exclude_patterns: &[Pattern]) -> Result<Vec<PathBuf>> {
        // First, collect all directories to scan
        let dirs_to_scan = self.collect_directories_to_scan(&self.settings.scan_path, exclude_patterns, 0)?;
        
        // Then, check each directory for package.json in parallel
        let package_dirs: Vec<PathBuf> = dirs_to_scan.par_iter()
            .filter_map(|dir| {
                let package_json_path = dir.join("package.json");
                if package_json_path.exists() {
                    Some(dir.clone())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(package_dirs)
    }
    
    /// Find all directories containing package.json files using parallel processing with progress reporting
    fn find_package_dirs_parallel_with_progress(
        &self,
        exclude_patterns: &[Pattern],
        progress_counter: Arc<Mutex<usize>>,
        progress_callback: Arc<impl Fn(ProgressUpdate) + Send + Sync>,
    ) -> Result<Vec<PathBuf>> {
        // First, collect all directories to scan
        let dirs_to_scan = self.collect_directories_to_scan(&self.settings.scan_path, exclude_patterns, 0)?;
        
        // Report initial progress
        progress_callback(ProgressUpdate::new(
            0,
            dirs_to_scan.len(),
            format!("Scanning {} directories", dirs_to_scan.len())
        ));
        
        // Then, check each directory for package.json in parallel with progress reporting
        let package_dirs: Vec<PathBuf> = dirs_to_scan.par_iter()
            .filter_map(|dir| {
                // Update progress counter
                let current = {
                    let mut counter = match progress_counter.lock() {
                        Ok(guard) => guard,
                        Err(_) => {
                            // If we can't lock the counter, just continue without updating progress
                            return None;
                        }
                    };
                    *counter += 1;
                    *counter
                };
                
                // Report progress every 100 directories to avoid too many updates
                if current % 100 == 0 || current == dirs_to_scan.len() {
                    progress_callback(ProgressUpdate::new(
                        current,
                        dirs_to_scan.len(),
                        format!("Scanning directory {}/{}", current, dirs_to_scan.len())
                    ));
                }
                
                // Check for package.json
                let package_json_path = dir.join("package.json");
                if package_json_path.exists() {
                    Some(dir.clone())
                } else {
                    None
                }
            })
            .collect();
        
        Ok(package_dirs)
    }
    
    /// Collect all directories to scan recursively
    fn collect_directories_to_scan(
        &self,
        dir: &Path,
        exclude_patterns: &[Pattern],
        current_depth: usize,
    ) -> Result<Vec<PathBuf>> {
        let mut dirs = Vec::new();
        
        // Check max depth
        if let Some(max_depth) = self.settings.max_depth {
            if current_depth > max_depth {
                return Ok(dirs);
            }
        }
        
        // Check if this directory should be excluded
        let dir_str = dir.to_string_lossy();
        for pattern in exclude_patterns {
            if pattern.matches(&dir_str) {
                return Ok(dirs);
            }
        }
        
        // Add this directory to the list
        dirs.push(dir.to_path_buf());
        
        // Recursively check subdirectories
        match std::fs::read_dir(dir) {
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
                                    match self.collect_directories_to_scan(
                                        &path,
                                        exclude_patterns,
                                        current_depth + 1,
                                    ) {
                                        Ok(subdirs) => dirs.extend(subdirs),
                                        Err(err) => {
                                            // Store non-critical errors and continue
                                            if !err.is_critical() {
                                                if let Ok(mut errors) = self.errors.lock() {
                                                    errors.push((path.clone(), err));
                                                }
                                            } else {
                                                // Critical errors should stop processing
                                                return Err(err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            // Handle permission denied errors gracefully
                            if err.kind() == std::io::ErrorKind::PermissionDenied {
                                // Store the error and continue
                                if let Ok(mut errors) = self.errors.lock() {
                                    errors.push((
                                        dir.to_path_buf(),
                                        WalkerError::permission_denied(dir),
                                    ));
                                }
                                continue;
                            } else {
                                // Store other IO errors and continue
                                if let Ok(mut errors) = self.errors.lock() {
                                    errors.push((
                                        dir.to_path_buf(),
                                        WalkerError::io_error(err),
                                    ));
                                }
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
                    if let Ok(mut errors) = self.errors.lock() {
                        errors.push((
                            dir.to_path_buf(),
                            WalkerError::permission_denied(dir),
                        ));
                    }
                    return Ok(dirs);
                } else {
                    // Store other directory traversal errors
                    if let Ok(mut errors) = self.errors.lock() {
                        errors.push((
                            dir.to_path_buf(),
                            WalkerError::directory_traversal_error(
                                dir,
                                format!("Failed to read directory: {}", err),
                            ),
                        ));
                    }
                    return Ok(dirs);
                }
            }
        }
        
        Ok(dirs)
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
}
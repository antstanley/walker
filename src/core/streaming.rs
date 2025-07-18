//! Streaming implementation for large result sets
//!
//! This module provides functionality for processing large result sets in batches
//! to optimize memory usage for large codebases.

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use rayon::prelude::*;
use crate::models::analysis::{PackageAnalysis, AnalysisResults};
use crate::error::{Result, WalkerError};
use crate::core::analyzer::Analyzer;

/// Streaming processor for handling large result sets
pub struct StreamingProcessor {
    /// Batch size for processing
    batch_size: usize,
    
    /// Memory limit in MB before switching to disk-based processing
    memory_limit_mb: usize,
    
    /// Analyzer for package analysis
    analyzer: Arc<Analyzer>,
    
    /// Results accumulator
    results: Arc<Mutex<AnalysisResults>>,
}

impl StreamingProcessor {
    /// Create a new streaming processor
    pub fn new(batch_size: usize, memory_limit_mb: usize, analyzer: Arc<Analyzer>) -> Self {
        Self {
            batch_size,
            memory_limit_mb,
            analyzer,
            results: Arc::new(Mutex::new(AnalysisResults::new())),
        }
    }
    
    /// Process packages in batches
    pub fn process_packages(&self, package_dirs: Vec<PathBuf>) -> Result<AnalysisResults> {
        // Calculate number of batches
        let total_packages = package_dirs.len();
        let num_batches = (total_packages + self.batch_size - 1) / self.batch_size;
        
        // Process in batches
        for batch_idx in 0..num_batches {
            let start_idx = batch_idx * self.batch_size;
            let end_idx = std::cmp::min(start_idx + self.batch_size, total_packages);
            
            // Get current batch
            let batch = &package_dirs[start_idx..end_idx];
            
            // Process batch in parallel
            let batch_results: Vec<_> = batch.par_iter()
                .map(|dir| {
                    match self.analyzer.analyze_package_with_options(dir) {
                        Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                        Err(err) => Some((dir.clone(), Err(err))),
                    }
                })
                .collect();
            
            // Accumulate results
            let mut results_guard = match self.results.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    return Err(WalkerError::ParallelExecution {
                        message: "Failed to lock results for batch processing".to_string(),
                        #[cfg(not(tarpaulin_include))]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
            };
            
            // Process batch results
            for result in batch_results {
                if let Some((dir, analysis_result)) = result {
                    match analysis_result {
                        Ok(analysis) => {
                            results_guard.add_package(analysis);
                        }
                        Err(err) => {
                            // Add error to results and continue with next package
                            results_guard.add_error(dir, &err);
                            
                            // If this is a critical error, stop processing
                            if err.is_critical() {
                                return Err(err);
                            }
                        }
                    }
                }
            }
            
            // Check memory usage and potentially switch to disk-based storage
            if self.should_use_disk_storage() {
                self.offload_to_disk(&mut results_guard)?;
            }
        }
        
        // Return final results
        match self.results.lock() {
            Ok(guard) => Ok(guard.clone()),
            Err(_) => {
                Err(WalkerError::ParallelExecution {
                    message: "Failed to lock results for final retrieval".to_string(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                })
            }
        }
    }
    
    /// Process packages in batches with progress reporting
    pub fn process_packages_with_progress<F>(
        &self, 
        package_dirs: Vec<PathBuf>,
        progress_callback: F
    ) -> Result<AnalysisResults>
    where
        F: Fn(crate::core::parallel::ProgressUpdate) + Send + Sync,
    {
        // Calculate number of batches
        let total_packages = package_dirs.len();
        let num_batches = (total_packages + self.batch_size - 1) / self.batch_size;
        
        // Process in batches
        for batch_idx in 0..num_batches {
            let start_idx = batch_idx * self.batch_size;
            let end_idx = std::cmp::min(start_idx + self.batch_size, total_packages);
            
            // Get current batch
            let batch = &package_dirs[start_idx..end_idx];
            
            // Report progress
            progress_callback(crate::core::parallel::ProgressUpdate::new(
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
                    match self.analyzer.analyze_package_with_options(dir) {
                        Ok(analysis) => Some((dir.clone(), Ok(analysis))),
                        Err(err) => Some((dir.clone(), Err(err))),
                    }
                })
                .collect();
            
            // Accumulate results
            let mut results_guard = match self.results.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    return Err(WalkerError::ParallelExecution {
                        message: "Failed to lock results for batch processing".to_string(),
                        #[cfg(not(tarpaulin_include))]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
            };
            
            // Process batch results
            for result in batch_results {
                if let Some((dir, analysis_result)) = result {
                    match analysis_result {
                        Ok(analysis) => {
                            results_guard.add_package(analysis);
                        }
                        Err(err) => {
                            // Add error to results and continue with next package
                            results_guard.add_error(dir, &err);
                            
                            // If this is a critical error, stop processing
                            if err.is_critical() {
                                return Err(err);
                            }
                        }
                    }
                }
            }
            
            // Check memory usage and potentially switch to disk-based storage
            if self.should_use_disk_storage() {
                self.offload_to_disk(&mut results_guard)?;
            }
            
            // Report batch completion
            progress_callback(crate::core::parallel::ProgressUpdate::new(
                end_idx,
                total_packages,
                format!("Completed batch {}/{}", batch_idx + 1, num_batches)
            ));
        }
        
        // Report final progress
        progress_callback(crate::core::parallel::ProgressUpdate::new(
            total_packages,
            total_packages,
            "Processing complete".to_string()
        ));
        
        // Return final results
        match self.results.lock() {
            Ok(guard) => Ok(guard.clone()),
            Err(_) => {
                Err(WalkerError::ParallelExecution {
                    message: "Failed to lock results for final retrieval".to_string(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                })
            }
        }
    }
    
    /// Check if memory usage exceeds limit and we should use disk storage
    fn should_use_disk_storage(&self) -> bool {
        // Get current memory usage
        if let Some(memory_usage) = self.get_memory_usage_mb() {
            return memory_usage > self.memory_limit_mb;
        }
        
        // If we can't determine memory usage, be conservative
        false
    }
    
    /// Get current memory usage in MB
    fn get_memory_usage_mb(&self) -> Option<usize> {
        // This is a platform-specific implementation
        // For now, we'll use a simple heuristic based on results size
        
        if let Ok(results) = self.results.lock() {
            // Rough estimate: each package analysis is about 1KB
            let package_memory = results.packages.len() * 1024;
            
            // Errors are smaller, about 100 bytes each
            let error_memory = results.errors.len() * 100;
            
            // Convert to MB
            Some((package_memory + error_memory) / (1024 * 1024))
        } else {
            None
        }
    }
    
    /// Offload results to disk to reduce memory usage
    fn offload_to_disk(&self, results: &mut AnalysisResults) -> Result<()> {
        // Create a temporary file for storing results
        let temp_dir = std::env::temp_dir().join("walker_results");
        std::fs::create_dir_all(&temp_dir)?;
        
        let temp_file = temp_dir.join(format!("batch_{}.json", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()));
        
        // Serialize packages to disk
        let packages = std::mem::take(&mut results.packages);
        let json = serde_json::to_string(&packages)?;
        std::fs::write(&temp_file, json)?;
        
        // Store file path in results for later retrieval
        results.offloaded_files.push(temp_file);
        
        Ok(())
    }
}

/// Extension trait for AnalysisResults to support streaming
pub trait StreamingResults {
    /// Offloaded files containing serialized packages
    fn offloaded_files(&self) -> &Vec<PathBuf>;
    
    /// Load all packages from offloaded files
    fn load_all_packages(&self) -> Result<Vec<PackageAnalysis>>;
}

impl StreamingResults for AnalysisResults {
    fn offloaded_files(&self) -> &Vec<PathBuf> {
        &self.offloaded_files
    }
    
    fn load_all_packages(&self) -> Result<Vec<PackageAnalysis>> {
        let mut all_packages = self.packages.clone();
        
        // Load packages from offloaded files
        for file_path in &self.offloaded_files {
            let json = std::fs::read_to_string(file_path)?;
            let packages: Vec<PackageAnalysis> = serde_json::from_str(&json)?;
            all_packages.extend(packages);
        }
        
        Ok(all_packages)
    }
}

// Note: The offloaded_files field has already been added to AnalysisResults in models/analysis.rs
//! Analysis result structures

use super::package::{DependencyInfo, ModuleSupport, PackageDetails};
use crate::error::{Result, WalkerError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Complete analysis of a single package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAnalysis {
    pub path: PathBuf,
    pub details: PackageDetails,
    pub module_support: ModuleSupport,
    pub size: Option<u64>,
    pub dependencies: DependencyInfo,
    pub typescript_support: bool,
    pub browser_support: bool,
    pub node_version_requirement: Option<String>,
    pub license: Option<String>,
    pub has_bin: bool,
    pub is_private: bool,
    pub has_scripts: HashMap<String, bool>,
    pub analysis_date: chrono::DateTime<chrono::Utc>,
}

impl PackageAnalysis {
    /// Create a new PackageAnalysis from package details and path
    pub fn new(path: PathBuf, details: PackageDetails) -> Self {
        // Use enhanced module support detection
        let module_support = ModuleSupport::from_package_details(&details);
        let dependencies = DependencyInfo::from_package_details(&details);
        
        // Use the enhanced module support detection for TypeScript and browser support
        let typescript_support = module_support.has_typescript_support();
        let browser_support = module_support.has_browser_support();
        
        // Extract other package information
        let node_version_requirement = details.node_version_requirement();
        let license = details.license.clone();
        let has_bin = details.bin.is_some();
        let is_private = details.private.unwrap_or(false);
        
        // Check for common scripts
        let mut has_scripts = HashMap::new();
        if let Some(scripts) = &details.scripts {
            for script_name in &["test", "build", "start", "lint", "dev", "prepare"] {
                has_scripts.insert(script_name.to_string(), scripts.contains_key(*script_name));
            }
        }
        
        Self {
            path,
            details,
            module_support,
            size: None, // Size will be calculated separately
            dependencies,
            typescript_support,
            browser_support,
            node_version_requirement,
            license,
            has_bin,
            is_private,
            has_scripts,
            analysis_date: chrono::Utc::now(),
        }
    }
    
    /// Calculate and set the package size
    pub fn calculate_size(&mut self) -> Result<u64> {
        let size = calculate_directory_size(&self.path)?;
        self.size = Some(size);
        Ok(size)
    }
    
    /// Check if the package is dual-mode (supports both ESM and CJS)
    pub fn is_dual_mode(&self) -> bool {
        self.module_support.is_dual_mode()
    }
    
    /// Check if the package is ESM-only
    pub fn is_esm_only(&self) -> bool {
        self.module_support.is_esm_only()
    }
    
    /// Check if the package is CJS-only
    pub fn is_cjs_only(&self) -> bool {
        self.module_support.is_cjs_only()
    }
}

/// Collection of all analysis results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub packages: Vec<PackageAnalysis>,
    pub summary: AnalysisSummary,
    pub errors: Vec<AnalysisError>,
    /// Files containing offloaded packages for streaming mode
    pub offloaded_files: Vec<PathBuf>,
}

impl AnalysisResults {
    /// Create a new empty AnalysisResults instance
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            summary: AnalysisSummary::default(),
            errors: Vec::new(),
            offloaded_files: Vec::new(),
        }
    }
    
    /// Add a package analysis to the results
    pub fn add_package(&mut self, package: PackageAnalysis) {
        // Update summary statistics
        self.summary.total_packages += 1;
        
        if package.module_support.esm.overall {
            self.summary.esm_supported += 1;
        }
        
        if package.module_support.cjs.overall {
            self.summary.cjs_supported += 1;
        }
        
        if package.typescript_support {
            self.summary.typescript_supported += 1;
        }
        
        if package.browser_support {
            self.summary.browser_supported += 1;
        }
        
        if let Some(size) = package.size {
            self.summary.total_size += size;
        }
        
        // Add package to the list
        self.packages.push(package);
    }
    
    /// Add an error to the results
    pub fn add_error(&mut self, path: PathBuf, error: &WalkerError) {
        let severity = ErrorSeverity::from(error);
        
        let analysis_error = AnalysisError {
            path,
            error: error.user_message(),
            severity,
        };
        
        self.summary.errors_encountered += 1;
        self.errors.push(analysis_error);
    }
    
    /// Set the scan duration in the summary
    pub fn set_scan_duration(&mut self, duration: Duration) {
        self.summary.scan_duration = duration;
    }
    
    /// Get packages that support ESM
    pub fn esm_packages(&self) -> Vec<&PackageAnalysis> {
        self.packages.iter()
            .filter(|p| p.module_support.esm.overall)
            .collect()
    }
    
    /// Get packages that support CJS
    pub fn cjs_packages(&self) -> Vec<&PackageAnalysis> {
        self.packages.iter()
            .filter(|p| p.module_support.cjs.overall)
            .collect()
    }
    
    /// Get packages that are dual-mode (support both ESM and CJS)
    pub fn dual_mode_packages(&self) -> Vec<&PackageAnalysis> {
        self.packages.iter()
            .filter(|p| p.is_dual_mode())
            .collect()
    }
    
    /// Get packages that support TypeScript
    pub fn typescript_packages(&self) -> Vec<&PackageAnalysis> {
        self.packages.iter()
            .filter(|p| p.typescript_support)
            .collect()
    }
    
    /// Get packages that support browser usage
    pub fn browser_packages(&self) -> Vec<&PackageAnalysis> {
        self.packages.iter()
            .filter(|p| p.browser_support)
            .collect()
    }
    
    /// Get critical errors that occurred during analysis
    pub fn critical_errors(&self) -> Vec<&AnalysisError> {
        self.errors.iter()
            .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
            .collect()
    }
    
    /// Check if there were any critical errors
    pub fn has_critical_errors(&self) -> bool {
        self.errors.iter().any(|e| matches!(e.severity, ErrorSeverity::Critical))
    }

    /// Finalize the results (calculate summary statistics)
    pub fn finalize(&mut self) {
        // Update summary statistics
        self.summary.total_packages = self.packages.len();

        // Count module support types
        for package in &self.packages {
            if package.module_support.is_dual_mode() {
                self.summary.dual_mode += 1;
                self.summary.esm_supported += 1;
                self.summary.cjs_supported += 1;
            } else if package.module_support.is_esm_only() {
                self.summary.esm_only += 1;
                self.summary.esm_supported += 1;
            } else if package.module_support.is_cjs_only() {
                self.summary.cjs_only += 1;
                self.summary.cjs_supported += 1;
            }

            if package.typescript_support {
                self.summary.typescript_supported += 1;
            }

            if package.browser_support {
                self.summary.browser_supported += 1;
            }
        }

        // Update error counts
        self.summary.errors_encountered = self.errors.len();
        self.summary.warnings_count = self.errors.iter()
            .filter(|e| matches!(e.severity, ErrorSeverity::Warning))
            .count();
        self.summary.critical_errors_count = self.errors.iter()
            .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
            .count();
    }
}

/// Summary statistics from analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    pub total_packages: usize,
    pub esm_supported: usize,
    pub cjs_supported: usize,
    pub dual_mode: usize,
    pub esm_only: usize,
    pub cjs_only: usize,
    pub typescript_supported: usize,
    pub browser_supported: usize,
    pub total_size: u64,
    pub scan_duration: Duration,
    pub errors_encountered: usize,
    pub warnings_count: usize,
    pub critical_errors_count: usize,
    pub total_dependencies: usize,
    pub avg_dependencies_per_package: f64,
    pub largest_package_size: u64,
    pub largest_package_name: Option<String>,
    pub most_deps_package_name: Option<String>,
    pub most_deps_count: usize,
    // New fields for enhanced statistics
    pub median_package_size: u64,
    pub median_dependencies: usize,
    pub size_percentiles: Option<SizePercentiles>,
    pub dependency_percentiles: Option<DependencyPercentiles>,
    pub performance_metrics: Option<PerformanceMetrics>,
    pub directories_scanned: usize,
    pub files_processed: usize,
    pub smallest_package_size: u64,
    pub smallest_package_name: Option<String>,
    pub least_deps_package_name: Option<String>,
    pub least_deps_count: usize,
}

impl Default for AnalysisSummary {
    fn default() -> Self {
        Self {
            total_packages: 0,
            esm_supported: 0,
            cjs_supported: 0,
            dual_mode: 0,
            esm_only: 0,
            cjs_only: 0,
            typescript_supported: 0,
            browser_supported: 0,
            total_size: 0,
            scan_duration: Duration::from_secs(0),
            errors_encountered: 0,
            warnings_count: 0,
            critical_errors_count: 0,
            total_dependencies: 0,
            avg_dependencies_per_package: 0.0,
            largest_package_size: 0,
            largest_package_name: None,
            most_deps_package_name: None,
            most_deps_count: 0,
            // Initialize new fields
            median_package_size: 0,
            median_dependencies: 0,
            size_percentiles: None,
            dependency_percentiles: None,
            performance_metrics: None,
            directories_scanned: 0,
            files_processed: 0,
            smallest_package_size: u64::MAX,
            smallest_package_name: None,
            least_deps_package_name: None,
            least_deps_count: usize::MAX,
        }
    }
}

impl AnalysisSummary {
    /// Update the summary with a package analysis
    pub fn update_with_package(&mut self, package: &PackageAnalysis) {
        // Update module support counts
        if package.module_support.is_dual_mode() {
            self.dual_mode += 1;
        } else if package.module_support.is_esm_only() {
            self.esm_only += 1;
        } else if package.module_support.is_cjs_only() {
            self.cjs_only += 1;
        }
        
        // Update dependency statistics
        let deps_count = package.dependencies.total_count;
        self.total_dependencies += deps_count;
        
        // Track most dependencies
        if deps_count > self.most_deps_count {
            self.most_deps_count = deps_count;
            self.most_deps_package_name = Some(package.details.name.clone());
        }
        
        // Track least dependencies (if not zero, to avoid skewing stats with empty packages)
        if deps_count > 0 && deps_count < self.least_deps_count {
            self.least_deps_count = deps_count;
            self.least_deps_package_name = Some(package.details.name.clone());
        }
        
        // Update size statistics
        if let Some(size) = package.size {
            // Track largest package
            if size > self.largest_package_size {
                self.largest_package_size = size;
                self.largest_package_name = Some(package.details.name.clone());
            }
            
            // Track smallest package (if not zero, to avoid skewing stats with empty packages)
            if size > 0 && size < self.smallest_package_size {
                self.smallest_package_size = size;
                self.smallest_package_name = Some(package.details.name.clone());
            }
            
            // Update total size
            self.total_size += size;
        }
        
        // Recalculate average dependencies
        if self.total_packages > 0 {
            self.avg_dependencies_per_package = self.total_dependencies as f64 / self.total_packages as f64;
        }
        
        // Increment file processing counters
        self.files_processed += 1; // At minimum, we processed the package.json file
        
        // Increment directory counter (each package is at least one directory)
        self.directories_scanned += 1;
    }
    
    /// Update error statistics based on an error
    pub fn update_with_error(&mut self, error: &AnalysisError) {
        match error.severity {
            ErrorSeverity::Warning => self.warnings_count += 1,
            ErrorSeverity::Critical => self.critical_errors_count += 1,
            _ => {} // Regular errors are already counted in errors_encountered
        }
    }
    
    /// Calculate percentages of packages with different module support
    pub fn esm_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            return 0.0;
        }
        (self.esm_supported as f64 / self.total_packages as f64) * 100.0
    }
    
    /// Calculate percentages of packages with different module support
    pub fn cjs_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            return 0.0;
        }
        (self.cjs_supported as f64 / self.total_packages as f64) * 100.0
    }
    
    /// Calculate percentages of packages with TypeScript support
    pub fn typescript_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            return 0.0;
        }
        (self.typescript_supported as f64 / self.total_packages as f64) * 100.0
    }
    
    /// Calculate percentages of packages with browser support
    pub fn browser_percentage(&self) -> f64 {
        if self.total_packages == 0 {
            return 0.0;
        }
        (self.browser_supported as f64 / self.total_packages as f64) * 100.0
    }
    
    /// Format the scan duration as a human-readable string
    pub fn format_duration(&self) -> String {
        let secs = self.scan_duration.as_secs();
        let millis = self.scan_duration.subsec_millis();
        
        if secs == 0 {
            format!("{}ms", millis)
        } else if secs < 60 {
            format!("{}.{:03}s", secs, millis)
        } else {
            let mins = secs / 60;
            let secs = secs % 60;
            format!("{}m {}s", mins, secs)
        }
    }
    
    /// Format the total size as a human-readable string
    pub fn format_size(&self) -> String {
        if self.total_size < 1024 {
            format!("{}B", self.total_size)
        } else if self.total_size < 1024 * 1024 {
            format!("{:.2}KB", self.total_size as f64 / 1024.0)
        } else if self.total_size < 1024 * 1024 * 1024 {
            format!("{:.2}MB", self.total_size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2}GB", self.total_size as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

/// Error that occurred during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisError {
    pub path: PathBuf,
    pub error: String,
    pub severity: ErrorSeverity,
}

/// Severity level of analysis errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Warning,  // Log and continue (permission denied, corrupted file)
    Error,    // Log and skip current item (invalid JSON, missing file)
    Critical, // Stop execution (invalid configuration, system error)
}

impl From<&WalkerError> for ErrorSeverity {
    fn from(error: &WalkerError) -> Self {
        match error {
            WalkerError::PermissionDenied { .. } => ErrorSeverity::Warning,
            WalkerError::JsonParse { .. } => ErrorSeverity::Error,
            WalkerError::Config { .. } => ErrorSeverity::Critical,
            WalkerError::InvalidPath { .. } => ErrorSeverity::Error,
            _ => ErrorSeverity::Error,
        }
    }
}

/// Calculate the total size of a directory recursively
fn calculate_directory_size(path: &Path) -> Result<u64> {
    let mut total_size = 0;
    
    if path.is_file() {
        // If path is a file, just return its size
        let metadata = fs::metadata(path)
            .map_err(|e| WalkerError::io_error(e))?;
        return Ok(metadata.len());
    }
    
    // Read directory entries
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) => {
            if e.kind() == io::ErrorKind::PermissionDenied {
                // Return 0 for permission denied errors
                return Ok(0);
            } else {
                return Err(WalkerError::io_error(e));
            }
        }
    };
    
    // Process each entry
    for entry in entries {
        let entry = entry.map_err(|e| WalkerError::io_error(e))?;
        let path = entry.path();
        
        if path.is_file() {
            // Add file size
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue, // Skip files we can't access
            };
            total_size += metadata.len();
        } else if path.is_dir() {
            // Recursively calculate directory size
            match calculate_directory_size(&path) {
                Ok(size) => total_size += size,
                Err(_) => continue, // Skip directories we can't access
            }
        }
    }
    
    Ok(total_size)
}

/// Performance metrics for the analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_duration: Duration,
    pub packages_per_second: f64,
    pub parallel_execution: bool,
    pub cache_enabled: bool,
    pub size_calculation_enabled: bool,
    pub directories_scanned: usize,
    pub files_processed: usize,
    pub memory_usage: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub streaming_enabled: bool,
    pub batch_size: Option<usize>,
    pub offloaded_batches: usize,
}

/// Size percentiles for statistical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizePercentiles {
    pub p10: u64,
    pub p25: u64,
    pub p50: u64,
    pub p75: u64,
    pub p90: u64,
}

/// Dependency count percentiles for statistical analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyPercentiles {
    pub p10: usize,
    pub p25: usize,
    pub p50: usize,
    pub p75: usize,
    pub p90: usize,
}
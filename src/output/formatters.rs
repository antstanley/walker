//! Output formatting functionality
//!
//! This module provides formatters for different output formats.

use crate::error::{Result, WalkerError};
use crate::models::analysis::{AnalysisResults, PackageAnalysis, ErrorSeverity};
use ansi_term::Colour::{Red, Green, Yellow, Blue, Cyan, Purple};
use ansi_term::Style;
use std::io::Write;
use serde_json;
use csv;

/// Format a package analysis as text
pub fn format_package_text(
    package: &PackageAnalysis,
    use_colors: bool,
    verbose: bool,
) -> String {
    let mut output = String::new();
    
    // Package name and version
    if use_colors {
        output.push_str(&format!("{} {}\n", 
            Blue.bold().paint(&package.details.name),
            Style::new().dimmed().paint(&package.details.version)
        ));
    } else {
        output.push_str(&format!("{} {}\n", 
            package.details.name,
            package.details.version
        ));
    }
    
    // Package path
    if use_colors {
        output.push_str(&format!("  Path: {}\n", 
            Style::new().dimmed().paint(package.path.display().to_string())
        ));
    } else {
        output.push_str(&format!("  Path: {}\n", 
            package.path.display()
        ));
    }
    
    // Module support
    let esm_status = if package.module_support.esm.overall { 
        if use_colors { Green.paint("✓").to_string() } else { "Yes".to_string() }
    } else { 
        if use_colors { Red.paint("✗").to_string() } else { "No".to_string() }
    };
    
    let cjs_status = if package.module_support.cjs.overall { 
        if use_colors { Green.paint("✓").to_string() } else { "Yes".to_string() }
    } else { 
        if use_colors { Red.paint("✗").to_string() } else { "No".to_string() }
    };
    
    output.push_str(&format!("  ESM Support: {}\n", esm_status));
    output.push_str(&format!("  CJS Support: {}\n", cjs_status));
    
    // TypeScript and Browser support
    let ts_status = if package.typescript_support { 
        if use_colors { Green.paint("✓").to_string() } else { "Yes".to_string() }
    } else { 
        if use_colors { Red.paint("✗").to_string() } else { "No".to_string() }
    };
    
    let browser_status = if package.browser_support { 
        if use_colors { Green.paint("✓").to_string() } else { "Yes".to_string() }
    } else { 
        if use_colors { Red.paint("✗").to_string() } else { "No".to_string() }
    };
    
    output.push_str(&format!("  TypeScript: {}\n", ts_status));
    output.push_str(&format!("  Browser: {}\n", browser_status));
    
    // Package size
    if let Some(size) = package.size {
        let size_str = format_size(size);
        output.push_str(&format!("  Size: {}\n", size_str));
    }
    
    // Dependencies
    output.push_str(&format!("  Dependencies: {}\n", 
        package.dependencies.total_count
    ));
    
    // Additional verbose information
    if verbose {
        output.push_str("\n  Module Support Details:\n");
        
        // ESM details
        output.push_str("    ESM:\n");
        output.push_str(&format!("      type:module: {}\n", 
            if package.module_support.esm.type_module { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      exports field: {}\n", 
            if package.module_support.esm.exports_import { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      module field: {}\n", 
            if package.module_support.esm.module_field { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      main .mjs: {}\n", 
            if package.module_support.esm.main_mjs { "Yes" } else { "No" }
        ));
        
        // CJS details
        output.push_str("    CJS:\n");
        output.push_str(&format!("      type:commonjs: {}\n", 
            if package.module_support.cjs.type_commonjs { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      exports field: {}\n", 
            if package.module_support.cjs.exports_require { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      default support: {}\n", 
            if package.module_support.cjs.default_support { "Yes" } else { "No" }
        ));
        
        // TypeScript details
        output.push_str("    TypeScript:\n");
        output.push_str(&format!("      types field: {}\n", 
            if package.module_support.typescript.types_field { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      typings field: {}\n", 
            if package.module_support.typescript.typings_field { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      exports .d.ts: {}\n", 
            if package.module_support.typescript.exports_dts { "Yes" } else { "No" }
        ));
        
        // Browser details
        output.push_str("    Browser:\n");
        output.push_str(&format!("      browser field: {}\n", 
            if package.module_support.browser.browser_field { "Yes" } else { "No" }
        ));
        output.push_str(&format!("      exports browser: {}\n", 
            if package.module_support.browser.exports_browser { "Yes" } else { "No" }
        ));
        
        // Node version requirement
        if let Some(node_version) = &package.node_version_requirement {
            output.push_str(&format!("  Node Version: {}\n", node_version));
        }
        
        // License
        if let Some(license) = &package.license {
            output.push_str(&format!("  License: {}\n", license));
        }
        
        // Scripts
        if !package.has_scripts.is_empty() {
            output.push_str("  Scripts:\n");
            for (script, has_script) in &package.has_scripts {
                output.push_str(&format!("    {}: {}\n", 
                    script, if *has_script { "Yes" } else { "No" }
                ));
            }
        }
        
        // Dependency details
        output.push_str("  Dependency Counts:\n");
        output.push_str(&format!("    Production: {}\n", package.dependencies.production_count));
        output.push_str(&format!("    Development: {}\n", package.dependencies.development_count));
        output.push_str(&format!("    Peer: {}\n", package.dependencies.peer_count));
        output.push_str(&format!("    Optional: {}\n", package.dependencies.optional_count));
        
        // List actual dependencies if available
        if let Some(deps) = &package.dependencies.production_deps {
            if !deps.is_empty() {
                output.push_str("  Production Dependencies:\n");
                for dep in deps.iter().take(10) { // Limit to 10 to avoid excessive output
                    output.push_str(&format!("    {} ({})\n", dep.name, dep.version_spec));
                }
                if deps.len() > 10 {
                    output.push_str(&format!("    ... and {} more\n", deps.len() - 10));
                }
            }
        }
    }
    
    output.push('\n');
    output
}

/// Format analysis results as text
pub fn format_results_text(
    results: &AnalysisResults,
    use_colors: bool,
    verbose: bool,
) -> String {
    let mut output = String::new();
    
    // Summary header
    if use_colors {
        output.push_str(&format!("{}\n\n", 
            Blue.bold().paint("Package Analysis Summary")
        ));
    } else {
        output.push_str("Package Analysis Summary\n\n");
    }
    
    // Basic statistics
    let summary = &results.summary;
    
    output.push_str(&format!("Total packages: {}\n", summary.total_packages));
    output.push_str(&format!("ESM support: {} ({:.1}%)\n", 
        summary.esm_supported, 
        summary.esm_percentage()
    ));
    output.push_str(&format!("CJS support: {} ({:.1}%)\n", 
        summary.cjs_supported, 
        summary.cjs_percentage()
    ));
    output.push_str(&format!("TypeScript support: {} ({:.1}%)\n", 
        summary.typescript_supported, 
        summary.typescript_percentage()
    ));
    output.push_str(&format!("Browser support: {} ({:.1}%)\n", 
        summary.browser_supported, 
        summary.browser_percentage()
    ));
    
    // Module support breakdown
    output.push_str(&format!("Dual mode (ESM+CJS): {}\n", summary.dual_mode));
    output.push_str(&format!("ESM only: {}\n", summary.esm_only));
    output.push_str(&format!("CJS only: {}\n", summary.cjs_only));
    
    // Size information
    output.push_str(&format!("Total size: {}\n", summary.format_size()));
    
    // Scan duration
    output.push_str(&format!("Scan duration: {}\n", summary.format_duration()));
    
    // Error information
    if summary.errors_encountered > 0 {
        if use_colors {
            output.push_str(&format!("\n{}\n", 
                Yellow.bold().paint(format!("Errors encountered: {}", summary.errors_encountered))
            ));
        } else {
            output.push_str(&format!("\nErrors encountered: {}\n", summary.errors_encountered));
        }
        
        output.push_str(&format!("  Warnings: {}\n", summary.warnings_count));
        output.push_str(&format!("  Critical errors: {}\n", summary.critical_errors_count));
    }
    
    // Additional verbose information
    if verbose {
        output.push_str("\nDetailed Statistics:\n");
        
        // Dependency statistics
        output.push_str(&format!("Total dependencies: {}\n", summary.total_dependencies));
        output.push_str(&format!("Average dependencies per package: {:.1}\n", 
            summary.avg_dependencies_per_package
        ));
        
        // Largest package
        if let Some(name) = &summary.largest_package_name {
            output.push_str(&format!("Largest package: {} ({})\n", 
                name, format_size(summary.largest_package_size)
            ));
        }
        
        // Most dependencies
        if let Some(name) = &summary.most_deps_package_name {
            output.push_str(&format!("Most dependencies: {} ({})\n", 
                name, summary.most_deps_count
            ));
        }
        
        // List errors if any
        if !results.errors.is_empty() {
            output.push_str("\nErrors:\n");
            
            for error in &results.errors {
                let severity_str = match error.severity {
                    ErrorSeverity::Warning => {
                        if use_colors { Yellow.paint("WARNING").to_string() } 
                        else { "WARNING".to_string() }
                    },
                    ErrorSeverity::Error => {
                        if use_colors { Red.paint("ERROR").to_string() } 
                        else { "ERROR".to_string() }
                    },
                    ErrorSeverity::Critical => {
                        if use_colors { Red.bold().paint("CRITICAL").to_string() } 
                        else { "CRITICAL".to_string() }
                    },
                };
                
                output.push_str(&format!("  [{}] {}: {}\n", 
                    severity_str,
                    error.path.display(),
                    error.error
                ));
            }
        }
        
        // List all packages with their module support
        output.push_str("\nPackage Module Support:\n");
        
        for package in &results.packages {
            let support_str = if package.is_dual_mode() {
                if use_colors { Green.paint("Dual").to_string() } 
                else { "Dual".to_string() }
            } else if package.is_esm_only() {
                if use_colors { Blue.paint("ESM").to_string() } 
                else { "ESM".to_string() }
            } else if package.is_cjs_only() {
                if use_colors { Yellow.paint("CJS").to_string() } 
                else { "CJS".to_string() }
            } else {
                if use_colors { Red.paint("None").to_string() } 
                else { "None".to_string() }
            };
            
            output.push_str(&format!("  {} {} - {}\n", 
                support_str,
                package.details.name,
                package.details.version
            ));
        }
    } else {
        // In non-verbose mode, just show a summary of packages by type
        output.push_str("\nPackage Breakdown:\n");
        output.push_str(&format!("  Dual mode (ESM+CJS): {}\n", summary.dual_mode));
        output.push_str(&format!("  ESM only: {}\n", summary.esm_only));
        output.push_str(&format!("  CJS only: {}\n", summary.cjs_only));
    }
    
    output
}

/// Format a file size in a human-readable way
fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.2}KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.2}MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}/// For
mat analysis results as JSON
pub fn format_results_json(results: &AnalysisResults) -> Result<String> {
    serde_json::to_string_pretty(results)
        .map_err(|e| WalkerError::JsonSerialize { 
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })
}

/// Format analysis results as CSV
pub fn format_results_csv(results: &AnalysisResults) -> Result<String> {
    let mut writer = csv::Writer::from_writer(vec![]);
    
    // Write header row
    writer.write_record(&[
        "Package Name",
        "Version",
        "Path",
        "ESM Support",
        "CJS Support",
        "TypeScript Support",
        "Browser Support",
        "Size (bytes)",
        "Dependencies",
        "License",
        "Node Version",
        "Is Private",
        "Has Bin",
    ])?;
    
    // Write data rows
    for package in &results.packages {
        let size_str = match package.size {
            Some(size) => size.to_string(),
            None => "".to_string(),
        };
        
        let node_version = match &package.node_version_requirement {
            Some(version) => version.clone(),
            None => "".to_string(),
        };
        
        let license = match &package.license {
            Some(license) => license.clone(),
            None => "".to_string(),
        };
        
        writer.write_record(&[
            &package.details.name,
            &package.details.version,
            &package.path.display().to_string(),
            &package.module_support.esm.overall.to_string(),
            &package.module_support.cjs.overall.to_string(),
            &package.typescript_support.to_string(),
            &package.browser_support.to_string(),
            &size_str,
            &package.dependencies.total_count.to_string(),
            &license,
            &node_version,
            &package.is_private.to_string(),
            &package.has_bin.to_string(),
        ])?;
    }
    
    // Add summary row with empty cells for non-applicable fields
    writer.write_record(&[
        "SUMMARY",
        "",
        "",
        &format!("{}%", results.summary.esm_percentage()),
        &format!("{}%", results.summary.cjs_percentage()),
        &format!("{}%", results.summary.typescript_percentage()),
        &format!("{}%", results.summary.browser_percentage()),
        &results.summary.total_size.to_string(),
        &results.summary.total_dependencies.to_string(),
        "",
        "",
        "",
        "",
    ])?;
    
    // Get the CSV data as a string
    let data = String::from_utf8(writer.into_inner()?)
        .map_err(|e| WalkerError::CsvSerialize { 
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
    
    Ok(data)
}
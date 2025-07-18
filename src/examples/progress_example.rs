//! Example of using the progress reporting system

use crate::core::ParallelWalker;
use crate::models::config::Settings;
use crate::output::ProgressReporter;
use std::path::PathBuf;
use std::sync::Arc;

/// Run an analysis with progress reporting
pub fn run_with_progress(path: &str, quiet: bool, verbose: bool) -> crate::error::Result<()> {
    // Create settings
    let mut settings = Settings::default();
    settings.scan_path = PathBuf::from(path);
    settings.parallel = true;
    settings.quiet = quiet;
    settings.verbose = verbose;
    
    // Create a progress reporter
    let reporter = Arc::new(ProgressReporter::new(quiet, verbose));
    
    // Print initial message
    reporter.print(&format!("Analyzing packages in {}", path));
    
    // Create a parallel walker
    let walker = ParallelWalker::new(settings);
    
    // Create a progress callback
    let progress_callback = crate::output::create_progress_callback(reporter.clone());
    
    // Start the progress reporter
    reporter.start(0, "Scanning for packages");
    
    // Run analysis with progress reporting
    let results = walker.analyze_with_progress(progress_callback)?;
    
    // Finish the progress reporter
    reporter.finish(&format!("Found {} packages", results.packages.len()));
    
    // Print summary
    if !quiet {
        println!("\nAnalysis Summary:");
        println!("----------------");
        println!("Total packages: {}", results.summary.total_packages);
        println!("ESM supported: {} ({}%)", 
            results.summary.esm_supported,
            results.summary.esm_percentage().round()
        );
        println!("CJS supported: {} ({}%)", 
            results.summary.cjs_supported,
            results.summary.cjs_percentage().round()
        );
        println!("TypeScript supported: {} ({}%)", 
            results.summary.typescript_supported,
            results.summary.typescript_percentage().round()
        );
        println!("Browser supported: {} ({}%)", 
            results.summary.browser_supported,
            results.summary.browser_percentage().round()
        );
        println!("Total size: {}", results.summary.format_size());
        println!("Scan duration: {}", results.summary.format_duration());
        println!("Errors encountered: {}", results.summary.errors_encountered);
    }
    
    Ok(())
}

/// Example of using the progress reporter for a custom operation
pub fn custom_progress_example() {
    // Create a progress reporter
    let reporter = Arc::new(ProgressReporter::new(false, false));
    
    // Start the progress reporter
    reporter.start(100, "Processing items");
    
    // Simulate a long-running operation
    for i in 0..100 {
        // Update progress
        reporter.update(i, 100, &format!("Processing item {}", i));
        
        // Simulate work
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    
    // Finish the progress reporter
    reporter.finish("Processing complete");
}
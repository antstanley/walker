use std::process;
use std::time::Instant;
use walker::{
    cli::{Args, Command},
    config::{cli::CliConfig, ConfigBuilder, settings::Settings},
    core::{ParallelWalker, Walker},
    error::{Result, WalkerError, ErrorSeverity, context::ResultExt},
    models::analysis::{AnalysisResults, AnalysisError},
    output::{create_formatter, create_progress_callback, create_writer, ProgressReporter},
    VERSION, NAME,
};

fn main() {
    // Parse command-line arguments
    let args = Args::parse_args();
    
    // Create command from arguments
    let command = Command::from_args(args);
    
    // Run the command and get exit code
    let exit_code = run_command(command);
    
    // Exit with appropriate code
    process::exit(exit_code);
}

/// Run the command with proper error handling
fn run_command(command: Command) -> i32 {
    match execute_command(command) {
        Ok(_) => 0,
        Err(err) => {
            // Print user-friendly error message with context
            eprintln!("\nError: {}", err.user_message());
            
            // Print additional context if available
            if let Some(context) = err.context() {
                eprintln!("Context: {}", context);
            }
            
            // Print suggestion if available
            if let Some(suggestion) = err.suggestion() {
                eprintln!("Suggestion: {}", suggestion);
            } else {
                // Provide default suggestions based on error type
                match &err {
                    WalkerError::InvalidPath { .. } => {
                        eprintln!("Suggestion: Check that the path exists and is accessible");
                    }
                    WalkerError::ConfigNotFound { .. } => {
                        eprintln!("Suggestion: Create a .walker.toml file in your project directory or specify a config file with --config");
                    }
                    WalkerError::PermissionDenied { .. } => {
                        eprintln!("Suggestion: Try running with elevated permissions or check file permissions");
                    }
                    WalkerError::OutputDirectoryNotFound { .. } => {
                        eprintln!("Suggestion: Create the output directory first or specify a different path");
                    }
                    _ => {} // No default suggestion for other error types
                }
            }
            
            // Print backtrace in verbose mode
            if std::env::var("WALKER_VERBOSE").is_ok() || std::env::var("RUST_BACKTRACE").is_ok() {
                if let Some(backtrace) = err.backtrace() {
                    eprintln!("\nBacktrace:\n{}", backtrace);
                }
            }
            
            // Return appropriate exit code based on error severity
            let exit_code = match err.severity() {
                ErrorSeverity::Warning => 0, // Warnings don't cause failure
                ErrorSeverity::Error => 1,   // Regular errors
                ErrorSeverity::Critical => 2, // Critical errors
            };
            
            // Print a helpful message about exit code if it's non-zero
            if exit_code > 0 {
                eprintln!("\nExiting with code {} due to {}", exit_code, err.severity());
            }
            
            exit_code
        }
    }
}

/// Execute the command with proper orchestration
fn execute_command(command: Command) -> Result<()> {
    match command {
        Command::Init => {
            // Create a default configuration file in the current directory
            let config_path = std::path::PathBuf::from(".walker.toml");
            
            // Check if the file already exists
            if config_path.exists() {
                println!("Configuration file already exists at: {}", config_path.display());
                println!("To overwrite it, delete the file first and run this command again.");
                return Ok(());
            }
            
            // Create the configuration file
            crate::config::parser::create_default_config(&config_path)?;
            
            println!("Created default configuration file at: {}", config_path.display());
            println!("\nThe configuration file contains default settings that you can customize.");
            println!("You can now edit this file to configure Walker according to your needs.");
            println!("\nExample configuration options:");
            println!("  - scan_path: Directory to scan for packages");
            println!("  - exclude_patterns: Patterns for directories to exclude");
            println!("  - max_depth: Maximum directory depth to traverse");
            println!("  - output_format: Output format (text, json, csv)");
            println!("  - calculate_size: Whether to calculate package sizes");
            println!("\nFor more information, see the documentation.");
            
            Ok(())
        },
        Command::Analyze(args) => {
            // Start timing
            let start_time = Instant::now();
            
            // Validate arguments with improved error handling
            validate_args(&args).with_context(|| "Failed to validate command arguments")?;
            
            // Convert Args to CliConfig
            let cli_config = CliConfig::from_args(&args);
            
            // Load settings from CLI config with improved error handling
            let partial_settings = cli_config.load()
                .with_context(|| "Failed to load CLI configuration")?;
            
            // Build final settings
            let config_builder = ConfigBuilder::new();
            
            // Add config file if specified with improved error handling
            let config_builder = if let Some(config_path) = cli_config.config_path() {
                config_builder.add_config_file(config_path)
                    .with_context(|| format!("Failed to load config file: {}", config_path.display()))?
            } else {
                // Try to load default config file
                config_builder.try_add_default_config_file()
            };
            
            // Merge CLI settings (highest priority) with improved error handling
            let settings = config_builder
                .merge(partial_settings)
                .build()
                .with_context(|| "Failed to build final configuration")?;
            
            // Display startup information
            if !settings.quiet {
                println!("{} v{} - Node.js package analyzer", NAME, VERSION);
                println!("Scanning path: {}", settings.scan_path.display());
                println!("Output format: {}", settings.output_format);
                
                if settings.exclude_patterns.is_empty() {
                    println!("No exclusion patterns");
                } else {
                    println!("Excluding: {}", settings.exclude_patterns.join(", "));
                }
                
                if let Some(depth) = settings.max_depth {
                    println!("Maximum depth: {}", depth);
                }
                
                if settings.verbose {
                    println!("\nDetailed settings:");
                    println!("  Parallel processing: {}", if settings.parallel { "enabled" } else { "disabled" });
                    println!("  Size calculation: {}", if settings.calculate_size { "enabled" } else { "disabled" });
                    println!("  Cache: {}", if settings.cache_enabled { "enabled" } else { "disabled" });
                    println!("  Colors: {}", if settings.use_colors { "enabled" } else { "disabled" });
                    println!("  Progress reporting: {}", if settings.show_progress { "enabled" } else { "disabled" });
                    println!("  Streaming results: {}", if settings.stream_results { "enabled" } else { "disabled" });
                    
                    if settings.stream_results {
                        println!("  Batch size: {}", settings.batch_size);
                        println!("  Memory limit: {}MB", settings.memory_limit_mb);
                    }
                    
                    if let Some(output_file) = &settings.output_file {
                        println!("  Output file: {}", output_file.display());
                    }
                }
            }
            
            // Create progress reporter
            let progress_reporter = std::sync::Arc::new(
                ProgressReporter::new(settings.quiet, settings.verbose)
            );
            
            // Execute the analysis with improved error handling
            let mut results = if settings.parallel {
                // Use parallel walker with progress reporting
                let walker = ParallelWalker::new(settings.clone());
                
                // Create progress callback
                let progress_callback = create_progress_callback(progress_reporter.clone());
                
                // Start the progress reporter
                if !settings.quiet && settings.show_progress {
                    progress_reporter.start(0, &format!("Scanning {}", settings.scan_path.display()));
                }
                
                // Run analysis with progress reporting and improved error handling
                let analysis_results = walker.analyze_with_progress(progress_callback)
                    .with_context(|| format!("Failed to analyze directory: {}", settings.scan_path.display()))?;
                
                // Finish the progress reporter
                if !settings.quiet && settings.show_progress {
                    progress_reporter.finish(&format!("Found {} packages", analysis_results.packages.len()));
                }
                
                analysis_results
            } else {
                // Use regular walker
                let walker = Walker::new(settings.clone());
                
                // Run analysis with progress reporting function and improved error handling
                if !settings.quiet && settings.show_progress {
                    walker.analyze_with_progress(|current, total, message| {
                        println!("[{}/{}] {}", current, total, message);
                    }).with_context(|| format!("Failed to analyze directory: {}", settings.scan_path.display()))?
                } else {
                    walker.analyze()
                        .with_context(|| format!("Failed to analyze directory: {}", settings.scan_path.display()))?
                }
            };
            
            // Set the scan duration in the results
            let elapsed = start_time.elapsed();
            results.set_scan_duration(elapsed);
            
            // Update summary statistics for each package
            for package in &results.packages {
                results.summary.update_with_package(package);
            }
            
            // Update summary statistics for each error
            for error in &results.errors {
                results.summary.update_with_error(error);
            }
            
            // Generate and add performance metrics
            generate_performance_metrics(&mut results, &settings, elapsed);
            
            // Check if any packages were found
            if results.packages.is_empty() && !settings.quiet {
                println!("\nNo packages found in {}. If this is unexpected, check:", settings.scan_path.display());
                println!("  - The path is correct and contains Node.js packages");
                println!("  - Exclude patterns aren't filtering out all packages");
                println!("  - You have sufficient permissions to read the directories");
                
                if let Some(depth) = settings.max_depth {
                    println!("  - The max-depth setting ({}) isn't too restrictive", depth);
                }
            }
            
            // Display summary information if not in quiet mode
            if !settings.quiet && !results.packages.is_empty() {
                print_summary(&results, &settings);
            }
            
            // Create formatter based on output format with improved error handling
            let formatter = create_formatter(
                &settings.output_format,
                settings.use_colors,
                settings.verbose,
                settings.quiet
            );
            
            // Format the results with improved error handling
            let formatted_output = formatter.format(&results)
                .with_context(|| format!("Failed to format results as {}", settings.output_format))?;
            
            // Create writer based on output file
            let writer = create_writer(settings.output_file.as_ref());
            
            // Write the results with improved error handling
            writer.write(&formatted_output)
                .with_context(|| {
                    if let Some(path) = &settings.output_file {
                        format!("Failed to write results to {}", path.display())
                    } else {
                        "Failed to write results to stdout".to_string()
                    }
                })?;
            
            // Print summary to stdout if writing to file
            if settings.output_file.is_some() && !settings.quiet {
                println!("\nResults written to: {}", settings.output_file.as_ref().unwrap().display());
            }
            
            // Print timing information if not already shown in summary
            if settings.verbose && !settings.quiet {
                println!("\nTotal execution time: {:.2?}", elapsed);
            }
            
            // Print error summary if there were errors
            if !results.errors.is_empty() && !settings.quiet {
                print_error_summary(&results);
            }
            
            // Print a success message if not in quiet mode
            if !settings.quiet {
                println!("\nAnalysis completed successfully.");
            }
            
            Ok(())
        }
        Command::Version => {
            println!("{} v{}", NAME, VERSION);
            println!("A Node.js package analyzer for module system support detection");
            println!("Copyright (c) 2023-2025");
            println!("License: MIT");
            Ok(())
        }
        Command::Help => {
            // We don't need to implement this as clap handles help automatically
            // But we can add additional help information here if needed
            println!("{} v{} - Node.js package analyzer", NAME, VERSION);
            println!("\nUsage Examples:");
            println!("  walker                           # Analyze current directory");
            println!("  walker --path ./my-project       # Analyze specific directory");
            println!("  walker --exclude node_modules    # Skip node_modules directories");
            println!("  walker --output json             # Output in JSON format");
            println!("  walker --output-file report.json # Write results to file");
            println!("\nFor more options, use --help");
            Ok(())
        }
    }
}

/// Validate command arguments
fn validate_args(args: &Args) -> Result<()> {
    // Validate path if provided
    if let Some(path) = &args.path {
        if !path.exists() {
            return Err(WalkerError::InvalidPath { 
                path: path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
        
        // Check if path is a directory
        if !path.is_dir() {
            return Err(WalkerError::Config { 
                message: format!("Path '{}' is not a directory", path.display()),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
        
        // Check if path is readable
        match std::fs::read_dir(path) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(WalkerError::PermissionDenied { 
                    path: path.clone(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Err(e) => {
                return Err(WalkerError::io_error(e));
            }
            Ok(_) => {}
        }
    }
    
    // Validate config file if provided
    if let Some(config_path) = &args.config {
        if !config_path.exists() {
            return Err(WalkerError::ConfigNotFound { 
                path: config_path.clone(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            });
        }
        
        // Check if config file is readable
        match std::fs::File::open(config_path) {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                return Err(WalkerError::PermissionDenied { 
                    path: config_path.clone(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Err(e) => {
                return Err(WalkerError::ConfigRead { 
                    path: config_path.clone(),
                    source: e,
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Ok(_) => {}
        }
    }
    
    // Validate output file directory exists if provided
    if let Some(output_path) = &args.output_file {
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                return Err(WalkerError::OutputDirectoryNotFound { 
                    path: parent.to_path_buf(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            
            // Check if output directory is writable
            let test_file = parent.join(".walker_write_test");
            match std::fs::File::create(&test_file) {
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    return Err(WalkerError::PermissionDenied { 
                        path: parent.to_path_buf(),
                        #[cfg(not(tarpaulin_include))]
                        backtrace: std::backtrace::Backtrace::capture(),
                    });
                }
                Err(e) => {
                    return Err(WalkerError::io_error(e));
                }
                Ok(_) => {
                    // Clean up test file
                    let _ = std::fs::remove_file(test_file);
                }
            }
        }
    }
    
    // Validate exclude patterns
    for pattern in &args.exclude {
        match glob::Pattern::new(pattern) {
            Err(e) => {
                return Err(WalkerError::GlobPattern { 
                    source: e,
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
            Ok(_) => {}
        }
    }
    
    Ok(())
}
/// Print a summary of the analysis results
fn print_summary(results: &AnalysisResults, settings: &Settings) {
    println!("\n=== Analysis Summary ===");
    
    // Package statistics
    println!("Total packages analyzed: {}", results.summary.total_packages);
    
    if results.summary.total_packages > 0 {
        // Module support statistics
        println!("\nModule Support:");
        println!("  - ESM: {} ({:.1}%)", 
            results.summary.esm_supported, 
            results.summary.esm_percentage());
        println!("  - CJS: {} ({:.1}%)", 
            results.summary.cjs_supported, 
            results.summary.cjs_percentage());
        println!("  - Dual mode: {} ({:.1}%)", 
            results.summary.dual_mode, 
            (results.summary.dual_mode as f64 / results.summary.total_packages as f64) * 100.0);
        println!("  - ESM only: {} ({:.1}%)", 
            results.summary.esm_only, 
            (results.summary.esm_only as f64 / results.summary.total_packages as f64) * 100.0);
        println!("  - CJS only: {} ({:.1}%)", 
            results.summary.cjs_only, 
            (results.summary.cjs_only as f64 / results.summary.total_packages as f64) * 100.0);
        
        // TypeScript and browser support
        println!("\nFeature Support:");
        println!("  - TypeScript: {} ({:.1}%)", 
            results.summary.typescript_supported, 
            results.summary.typescript_percentage());
        println!("  - Browser: {} ({:.1}%)", 
            results.summary.browser_supported, 
            results.summary.browser_percentage());
        
        // Size information
        if settings.calculate_size {
            println!("\nSize Information:");
            println!("  - Total size: {}", results.summary.format_size());
            
            if let Some(largest_name) = &results.summary.largest_package_name {
                println!("  - Largest package: {} ({})", 
                    largest_name, 
                    format_size(results.summary.largest_package_size));
            }
            
            if let Some(smallest_name) = &results.summary.smallest_package_name {
                if results.summary.smallest_package_size < u64::MAX {
                    println!("  - Smallest package: {} ({})", 
                        smallest_name, 
                        format_size(results.summary.smallest_package_size));
                }
            }
            
            // Calculate average package size
            let avg_size = if results.summary.total_packages > 0 {
                results.summary.total_size as f64 / results.summary.total_packages as f64
            } else {
                0.0
            };
            
            println!("  - Average package size: {}", format_size(avg_size as u64));
            
            // Display median package size if available
            if results.summary.median_package_size > 0 {
                println!("  - Median package size: {}", format_size(results.summary.median_package_size));
            }
            
            // Display size percentiles if available
            if let Some(percentiles) = &results.summary.size_percentiles {
                println!("\n  Size Distribution:");
                println!("    - 10th percentile: {}", format_size(percentiles.p10));
                println!("    - 25th percentile: {}", format_size(percentiles.p25));
                println!("    - 50th percentile: {}", format_size(percentiles.p50));
                println!("    - 75th percentile: {}", format_size(percentiles.p75));
                println!("    - 90th percentile: {}", format_size(percentiles.p90));
            }
        }
        
        // Dependency information
        println!("\nDependency Information:");
        println!("  - Total dependencies: {}", results.summary.total_dependencies);
        println!("  - Average dependencies per package: {:.1}", 
            results.summary.avg_dependencies_per_package);
        
        if results.summary.median_dependencies > 0 {
            println!("  - Median dependencies per package: {}", 
                results.summary.median_dependencies);
        }
        
        if let Some(most_deps_name) = &results.summary.most_deps_package_name {
            println!("  - Most dependencies: {} ({})", 
                most_deps_name, 
                results.summary.most_deps_count);
        }
        
        if let Some(least_deps_name) = &results.summary.least_deps_package_name {
            if results.summary.least_deps_count < usize::MAX {
                println!("  - Least dependencies: {} ({})", 
                    least_deps_name, 
                    results.summary.least_deps_count);
            }
        }
        
        // Display dependency percentiles if available
        if let Some(percentiles) = &results.summary.dependency_percentiles {
            println!("\n  Dependency Distribution:");
            println!("    - 10th percentile: {}", percentiles.p10);
            println!("    - 25th percentile: {}", percentiles.p25);
            println!("    - 50th percentile: {}", percentiles.p50);
            println!("    - 75th percentile: {}", percentiles.p75);
            println!("    - 90th percentile: {}", percentiles.p90);
        }
        
        // Migration insights
        println!("\nMigration Insights:");
        let esm_migration_candidates = results.summary.cjs_only;
        let esm_migration_percentage = if results.summary.total_packages > 0 {
            (esm_migration_candidates as f64 / results.summary.total_packages as f64) * 100.0
        } else {
            0.0
        };
        
        println!("  - ESM migration candidates: {} ({:.1}%)", 
            esm_migration_candidates, 
            esm_migration_percentage);
        
        let typescript_migration_candidates = results.summary.total_packages - results.summary.typescript_supported;
        let typescript_migration_percentage = if results.summary.total_packages > 0 {
            (typescript_migration_candidates as f64 / results.summary.total_packages as f64) * 100.0
        } else {
            0.0
        };
        
        println!("  - TypeScript migration candidates: {} ({:.1}%)", 
            typescript_migration_candidates, 
            typescript_migration_percentage);
    }
    
    // Error statistics
    if results.summary.errors_encountered > 0 {
        println!("\nError Statistics:");
        println!("  - Total errors: {}", results.summary.errors_encountered);
        println!("  - Warnings: {}", results.summary.warnings_count);
        println!("  - Critical errors: {}", results.summary.critical_errors_count);
    }
    
    // Performance metrics
    println!("\nPerformance Metrics:");
    println!("  - Scan completed in: {}", format_duration(results.summary.scan_duration));
    
    // Display detailed performance metrics if available
    if let Some(metrics) = &results.summary.performance_metrics {
        println!("  - Processing speed: {:.1} packages/second", metrics.packages_per_second);
        println!("  - Directories scanned: {}", metrics.directories_scanned);
        println!("  - Files processed: {}", metrics.files_processed);
        
        if let Some(memory) = metrics.memory_usage {
            println!("  - Memory usage: {}", format_size(memory));
        }
        
        println!("  - Parallel execution: {}", if metrics.parallel_execution { "enabled" } else { "disabled" });
        println!("  - Cache: {}", if metrics.cache_enabled { "enabled" } else { "disabled" });
        println!("  - Size calculation: {}", if metrics.size_calculation_enabled { "enabled" } else { "disabled" });
        println!("  - Streaming mode: {}", if metrics.streaming_enabled { "enabled" } else { "disabled" });
        
        if metrics.streaming_enabled {
            if let Some(batch_size) = metrics.batch_size {
                println!("  - Batch size: {}", batch_size);
            }
            println!("  - Offloaded batches: {}", metrics.offloaded_batches);
            
            // Calculate total packages including offloaded batches
            let total_offloaded = if let Some(batch_size) = metrics.batch_size {
                metrics.offloaded_batches * batch_size
            } else {
                0
            };
            
            if total_offloaded > 0 {
                println!("  - Packages in memory: {}", results.packages.len());
                println!("  - Packages offloaded to disk: ~{}", total_offloaded);
            }
        }
    } else {
        // Calculate packages per second if performance metrics aren't available
        let packages_per_second = if results.summary.scan_duration.as_secs() > 0 {
            results.summary.total_packages as f64 / results.summary.scan_duration.as_secs_f64()
        } else {
            results.summary.total_packages as f64 // If less than a second, just report the total
        };
        
        if packages_per_second >= 1.0 {
            println!("  - Processing speed: {:.1} packages/second", packages_per_second);
        }
    }
    
    // Add a timestamp
    println!("\nAnalysis completed at: {}", 
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
}

/// Print a summary of errors encountered during analysis
fn print_error_summary(results: &AnalysisResults) {
    if results.errors.is_empty() {
        return;
    }
    
    println!("\n=== Error Summary ===");
    
    // Group errors by severity
    let critical_errors: Vec<&AnalysisError> = results.errors.iter()
        .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
        .collect();
    
    let errors: Vec<&AnalysisError> = results.errors.iter()
        .filter(|e| matches!(e.severity, ErrorSeverity::Error))
        .collect();
    
    let warnings: Vec<&AnalysisError> = results.errors.iter()
        .filter(|e| matches!(e.severity, ErrorSeverity::Warning))
        .collect();
    
    // Print critical errors first
    if !critical_errors.is_empty() {
        println!("\nCritical Errors: {} (require immediate attention)", critical_errors.len());
        for (i, error) in critical_errors.iter().enumerate().take(5) {
            println!("  {}. {} - {}", i + 1, error.path.display(), error.error);
        }
        if critical_errors.len() > 5 {
            println!("  ... and {} more critical errors", critical_errors.len() - 5);
        }
        
        // Add suggestions for critical errors
        println!("\n  Suggestions for critical errors:");
        println!("  - Check configuration files for syntax errors");
        println!("  - Verify output directory permissions");
        println!("  - Ensure all required files exist");
    }
    
    // Print regular errors
    if !errors.is_empty() {
        println!("\nErrors: {} (analysis continued but some packages were skipped)", errors.len());
        for (i, error) in errors.iter().enumerate().take(5) {
            println!("  {}. {} - {}", i + 1, error.path.display(), error.error);
        }
        if errors.len() > 5 {
            println!("  ... and {} more errors", errors.len() - 5);
        }
        
        // Add suggestions for regular errors
        println!("\n  Suggestions for errors:");
        println!("  - Check package.json files for valid JSON syntax");
        println!("  - Verify file permissions in skipped directories");
        println!("  - Consider using --exclude to skip problematic directories");
    }
    
    // Print warnings
    if !warnings.is_empty() {
        println!("\nWarnings: {} (non-critical issues that didn't affect analysis)", warnings.len());
        for (i, warning) in warnings.iter().enumerate().take(5) {
            println!("  {}. {} - {}", i + 1, warning.path.display(), warning.error);
        }
        if warnings.len() > 5 {
            println!("  ... and {} more warnings", warnings.len() - 5);
        }
    }
    
    // Group errors by common patterns to provide targeted advice
    let permission_errors = results.errors.iter()
        .filter(|e| e.error.contains("permission denied") || e.error.contains("Permission denied"))
        .count();
    
    let json_errors = results.errors.iter()
        .filter(|e| e.error.contains("JSON") || e.error.contains("json"))
        .count();
    
    let not_found_errors = results.errors.iter()
        .filter(|e| e.error.contains("not found") || e.error.contains("Not found"))
        .count();
    
    // Print targeted advice based on error patterns
    println!("\nCommon Issues Detected:");
    
    if permission_errors > 0 {
        println!("  - Permission issues: {} occurrences", permission_errors);
        println!("    Try running with elevated permissions or check file access rights");
    }
    
    if json_errors > 0 {
        println!("  - JSON parsing issues: {} occurrences", json_errors);
        println!("    Check package.json files for syntax errors");
    }
    
    if not_found_errors > 0 {
        println!("  - Missing files/directories: {} occurrences", not_found_errors);
        println!("    Verify paths and ensure required files exist");
    }
    
    // Add a note about verbose mode for more details
    println!("\nFor detailed error information, run with --verbose flag");
}

/// Format a size in bytes to a human-readable string
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
}

/// Format a duration in a human-readable way
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if total_secs == 0 {
        return format!("{}ms", millis);
    }
    
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}.{:03}s", secs, millis)
    }
}/// Gener
ate and add performance metrics to the analysis results
fn generate_performance_metrics(results: &mut AnalysisResults, settings: &Settings, elapsed: std::time::Duration) {
    // Store performance metrics in the summary
    results.summary.performance_metrics = Some(PerformanceMetrics {
        total_duration: elapsed,
        packages_per_second: if elapsed.as_secs() > 0 {
            results.packages.len() as f64 / elapsed.as_secs_f64()
        } else {
            results.packages.len() as f64 // If less than a second, just report the total
        },
        parallel_execution: settings.parallel,
        cache_enabled: settings.cache_enabled,
        size_calculation_enabled: settings.calculate_size,
        directories_scanned: results.summary.directories_scanned,
        files_processed: results.summary.files_processed,
        memory_usage: get_memory_usage(),
        timestamp: chrono::Utc::now(),
    });
    
    // Calculate additional statistics
    if results.packages.len() > 0 {
        // Calculate package size distribution
        if settings.calculate_size {
            let mut sizes: Vec<u64> = results.packages.iter()
                .filter_map(|p| p.size)
                .collect();
            
            if !sizes.is_empty() {
                sizes.sort();
                
                let median_size = if sizes.len() % 2 == 0 {
                    (sizes[sizes.len() / 2 - 1] + sizes[sizes.len() / 2]) / 2
                } else {
                    sizes[sizes.len() / 2]
                };
                
                results.summary.median_package_size = median_size;
                
                // Calculate size percentiles
                if sizes.len() >= 10 {
                    results.summary.size_percentiles = Some(SizePercentiles {
                        p10: sizes[sizes.len() / 10],
                        p25: sizes[sizes.len() / 4],
                        p50: median_size,
                        p75: sizes[sizes.len() * 3 / 4],
                        p90: sizes[sizes.len() * 9 / 10],
                    });
                }
            }
        }
        
        // Calculate dependency distribution
        let mut dep_counts: Vec<usize> = results.packages.iter()
            .map(|p| p.dependencies.total_count)
            .collect();
        
        if !dep_counts.is_empty() {
            dep_counts.sort();
            
            let median_deps = if dep_counts.len() % 2 == 0 {
                (dep_counts[dep_counts.len() / 2 - 1] + dep_counts[dep_counts.len() / 2]) / 2
            } else {
                dep_counts[dep_counts.len() / 2]
            };
            
            results.summary.median_dependencies = median_deps;
            
            // Calculate dependency percentiles
            if dep_counts.len() >= 10 {
                results.summary.dependency_percentiles = Some(DependencyPercentiles {
                    p10: dep_counts[dep_counts.len() / 10],
                    p25: dep_counts[dep_counts.len() / 4],
                    p50: median_deps,
                    p75: dep_counts[dep_counts.len() * 3 / 4],
                    p90: dep_counts[dep_counts.len() * 9 / 10],
                });
            }
        }
    }
}

/// Get the current memory usage of the process (if available)
fn get_memory_usage() -> Option<u64> {
    // This is a simple implementation that works on some platforms
    // A more robust implementation would use platform-specific APIs
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return Some(kb * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // For other platforms or if the above failed
    None
}

// Using the structs defined in models/analysis.rs///
 Generate and add performance metrics to the analysis results
fn generate_performance_metrics(results: &mut AnalysisResults, settings: &Settings, elapsed: std::time::Duration) {
    // Store performance metrics in the summary
    // Calculate total packages (including those in offloaded files)
    let total_packages = results.packages.len() + 
        results.offloaded_files.iter().map(|_| 0).count() * settings.batch_size;
    
    results.summary.performance_metrics = Some(models::analysis::PerformanceMetrics {
        total_duration: elapsed,
        packages_per_second: if elapsed.as_secs() > 0 {
            total_packages as f64 / elapsed.as_secs_f64()
        } else {
            total_packages as f64 // If less than a second, just report the total
        },
        parallel_execution: settings.parallel,
        cache_enabled: settings.cache_enabled,
        size_calculation_enabled: settings.calculate_size,
        directories_scanned: results.summary.directories_scanned,
        files_processed: results.summary.files_processed,
        memory_usage: get_memory_usage(),
        timestamp: chrono::Utc::now(),
        streaming_enabled: settings.stream_results,
        batch_size: if settings.stream_results { Some(settings.batch_size) } else { None },
        offloaded_batches: results.offloaded_files.len(),
    });
    
    // Calculate additional statistics
    if !results.packages.is_empty() {
        // Calculate package size distribution
        if settings.calculate_size {
            let mut sizes: Vec<u64> = results.packages.iter()
                .filter_map(|p| p.size)
                .collect();
            
            if !sizes.is_empty() {
                sizes.sort();
                
                let median_size = if sizes.len() % 2 == 0 {
                    (sizes[sizes.len() / 2 - 1] + sizes[sizes.len() / 2]) / 2
                } else {
                    sizes[sizes.len() / 2]
                };
                
                results.summary.median_package_size = median_size;
                
                // Calculate size percentiles
                if sizes.len() >= 10 {
                    results.summary.size_percentiles = Some(models::analysis::SizePercentiles {
                        p10: sizes[sizes.len() / 10],
                        p25: sizes[sizes.len() / 4],
                        p50: median_size,
                        p75: sizes[sizes.len() * 3 / 4],
                        p90: sizes[sizes.len() * 9 / 10],
                    });
                }
            }
        }
        
        // Calculate dependency distribution
        let mut dep_counts: Vec<usize> = results.packages.iter()
            .map(|p| p.dependencies.total_count)
            .collect();
        
        if !dep_counts.is_empty() {
            dep_counts.sort();
            
            let median_deps = if dep_counts.len() % 2 == 0 {
                (dep_counts[dep_counts.len() / 2 - 1] + dep_counts[dep_counts.len() / 2]) / 2
            } else {
                dep_counts[dep_counts.len() / 2]
            };
            
            results.summary.median_dependencies = median_deps;
            
            // Calculate dependency percentiles
            if dep_counts.len() >= 10 {
                results.summary.dependency_percentiles = Some(models::analysis::DependencyPercentiles {
                    p10: dep_counts[dep_counts.len() / 10],
                    p25: dep_counts[dep_counts.len() / 4],
                    p50: median_deps,
                    p75: dep_counts[dep_counts.len() * 3 / 4],
                    p90: dep_counts[dep_counts.len() * 9 / 10],
                });
            }
        }
    }
}

/// Get the current memory usage of the process (if available)
fn get_memory_usage() -> Option<u64> {
    // This is a simple implementation that works on some platforms
    // A more robust implementation would use platform-specific APIs
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return Some(kb * 1024); // Convert KB to bytes
                        }
                    }
                }
            }
        }
    }
    
    // For macOS, we could use task_info from the Mach API, but that requires
    // unsafe code or external crates, so we'll skip it for this implementation
    #[cfg(target_os = "macos")]
    {
        // Placeholder for macOS-specific memory usage implementation
    }
    
    // For Windows, we could use GetProcessMemoryInfo, but that requires
    // the windows crate or winapi, so we'll skip it for this implementation
    #[cfg(target_os = "windows")]
    {
        // Placeholder for Windows-specific memory usage implementation
    }
    
    // For other platforms or if the above failed
    None
}

/// Format a size in bytes to a human-readable string
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
}

/// Format a duration in a human-readable way
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if total_secs == 0 {
        return format!("{}ms", millis);
    }
    
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    
    if hours > 0 {
        format!("{}h {}m {}s", hours, mins, secs)
    } else if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}.{:03}s", secs, millis)
    }
}
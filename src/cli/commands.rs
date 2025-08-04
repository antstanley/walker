//! Command implementations

use super::Args;
use crate::config::{cli::CliConfig, ConfigBuilder, ConfigSource};
use crate::error::{Result, WalkerError};

/// Available commands
#[derive(Debug)]
pub enum Command {
    /// Analyze packages in the specified directory
    Analyze(Args),
    /// Show version information
    Version,
    /// Show help information
    Help,
    /// Initialize a default configuration file
    Init,
}

impl Command {
    /// Create a command from parsed arguments
    pub fn from_args(args: Args) -> Self {
        // Check for special commands
        if args.init {
            // Initialize configuration file command
            return Command::Init;
        }

        // Check for version or help commands (future expansion)
        if args.output == super::args::OutputFormat::Text &&
           args.path.is_none() &&
           args.exclude.is_empty() &&
           args.config.is_none() {
            // This could be a special command, but for now we default to Analyze
            // In the future, we could add more command detection logic here
        }

        Command::Analyze(args)
    }

    /// Execute the command
    pub fn execute(&self) -> Result<()> {
        match self {
            Command::Analyze(args) => {
                // Validate arguments
                self.validate()?;

                // Convert Args to CliConfig
                let cli_config = CliConfig::from_args(args);

                // Load settings from CLI config
                let partial_settings = cli_config.load()?;

                // Build final settings
                let config_builder = ConfigBuilder::new();

                // Add config file if specified
                let config_builder = if let Some(config_path) = cli_config.config_path() {
                    config_builder.add_config_file(config_path)?
                } else {
                    // Try to load default config file
                    config_builder.try_add_default_config_file()
                };

                // Merge CLI settings (highest priority)
                let settings = config_builder
                    .merge(partial_settings)
                    .build()?;

                // Display startup information
                if !settings.quiet {
                    println!("Walker v{} - Node.js package analyzer", env!("CARGO_PKG_VERSION"));
                    println!("Scanning path: {}", settings.scan_path.display());
                    println!("Output format: {}", settings.output_format);

                    if settings.verbose {
                        println!("Settings: {:#?}", settings);
                    }
                }

                // Execute the analysis
                if settings.parallel {
                    // Use parallel walker with progress reporting
                    let walker = crate::core::ParallelWalker::new(settings.clone());

                    // Create progress reporter
                    let reporter = std::sync::Arc::new(
                        crate::output::ProgressReporter::new(settings.quiet, settings.verbose)
                    );

                    // Create progress callback
                    let progress_callback = crate::output::create_progress_callback(reporter.clone());

                    // Start the progress reporter
                    if !settings.quiet && settings.show_progress {
                        reporter.start(0, &format!("Scanning {}", settings.scan_path.display()));
                    }

                    // Run analysis with progress reporting
                    let results = walker.analyze_with_progress(progress_callback)?;

                    // Finish the progress reporter
                    if !settings.quiet && settings.show_progress {
                        reporter.finish(&format!("Found {} packages", results.packages.len()));
                    }

                    // Print summary
                    if !settings.quiet {
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
                } else {
                    // Use regular walker
                    let mut walker = crate::core::Walker::new(settings.clone());

                    // Run analysis with progress reporting function
                    let results = if !settings.quiet && settings.show_progress {
                        walker.analyze_with_progress(|current, total, message| {
                            println!("[{}/{}] {}", current, total, message);
                        })?
                    } else {
                        walker.analyze()?
                    };

                    // Print summary
                    if !settings.quiet {
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
                    }
                }

                Ok(())
            }
            Command::Version => {
                println!("Walker v{}", env!("CARGO_PKG_VERSION"));
                println!("A Node.js package analyzer for module system support detection");
                println!("Copyright (c) 2023-2025");
                println!("License: MIT");
                println!("\nFor more information, visit: https://github.com/yourusername/walker");
                Ok(())
            }
            Command::Help => {
                // We don't need to implement this as clap handles help automatically
                // But we can add additional help information here if needed
                println!("Walker v{} - Node.js package analyzer", env!("CARGO_PKG_VERSION"));
                println!("\nUsage Examples:");
                println!("  walker                           # Analyze current directory");
                println!("  walker --path ./my-project       # Analyze specific directory");
                println!("  walker --exclude node_modules    # Skip node_modules directories");
                println!("  walker --output json             # Output in JSON format");
                println!("  walker --output-file report.json # Write results to file");

                println!("\nCommon Workflows:");
                println!("  # Quick scan of a project (fastest performance)");
                println!("  walker --path ./my-project --no-size --exclude node_modules");
                println!("\n  # Detailed analysis with all information");
                println!("  walker --path ./my-project --verbose");
                println!("\n  # Generate a CSV report for spreadsheet analysis");
                println!("  walker --path ./my-project --output csv --output-file report.csv");
                println!("\n  # Analyze only production dependencies");
                println!("  walker --path ./my-project --no-dev-deps --no-peer-deps --no-optional-deps");

                println!("\nFor more options, use --help");
                Ok(())
            }
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
            }
        }
    }

    /// Validate the command arguments
    pub fn validate(&self) -> Result<()> {
        match self {
            Command::Analyze(args) => {
                // Validate path if provided
                if let Some(path) = &args.path {
                    if !path.exists() {
                        return Err(WalkerError::InvalidPath {
                            path: path.clone(),
                            #[cfg(not(tarpaulin_include))]
                            backtrace: std::backtrace::Backtrace::capture(),
                        });
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
                }

                Ok(())
            }
            // No validation needed for these commands
            Command::Version | Command::Help | Command::Init => Ok(()),
        }
    }

    /// Run the command and handle errors
    pub fn run(&self) -> i32 {
        match self.execute() {
            Ok(_) => 0,
            Err(err) => {
                // Print user-friendly error message
                eprintln!("{}: {}", err.severity(), err.user_message());

                // Return appropriate exit code based on error severity
                match err.severity() {
                    crate::error::ErrorSeverity::Warning => 0, // Warnings don't cause failure
                    crate::error::ErrorSeverity::Error => 1,   // Regular errors
                    crate::error::ErrorSeverity::Critical => 2, // Critical errors
                }
            }
        }
    }
}

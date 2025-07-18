//! Output formatting and writing functionality

mod formatters;
mod writers;
mod progress;
#[cfg(test)]
mod tests;

pub use self::writers::{FileWriter, OutputWriter, StdoutWriter, create_writer};
pub use self::progress::{ProgressReporter, create_progress_callback};

use crate::error::Result;
use crate::models::analysis::AnalysisResults;

/// Trait for different output formatters
pub trait Formatter {
    /// Format analysis results into a string
    fn format(&self, results: &AnalysisResults) -> Result<String>;
}

/// Text formatter for human-readable output
pub struct TextFormatter {
    pub use_colors: bool,
    pub verbose: bool,
    pub quiet: bool,
}

impl TextFormatter {
    /// Create a new text formatter
    pub fn new(use_colors: bool, verbose: bool, quiet: bool) -> Self {
        Self {
            use_colors,
            verbose,
            quiet,
        }
    }
}

impl Formatter for TextFormatter {
    fn format(&self, results: &AnalysisResults) -> Result<String> {
        // In quiet mode, only output critical information
        if self.quiet {
            let mut output = String::new();
            
            // Just output the basic summary statistics
            let summary = &results.summary;
            output.push_str(&format!("Total: {}, ESM: {} ({:.1}%), CJS: {} ({:.1}%)\n", 
                summary.total_packages, 
                summary.esm_supported, 
                summary.esm_percentage(),
                summary.cjs_supported, 
                summary.cjs_percentage()
            ));
            
            // Add critical errors if any
            if summary.critical_errors_count > 0 {
                output.push_str(&format!("Critical errors: {}\n", summary.critical_errors_count));
            }
            
            return Ok(output);
        }
        
        // For normal or verbose mode, use the full formatter
        let mut output = String::new();
        
        // Add the summary
        output.push_str(&formatters::format_results_text(
            results,
            self.use_colors,
            self.verbose
        ));
        
        // In verbose mode, add details for each package
        if self.verbose {
            output.push_str("\nPackage Details:\n\n");
            
            for package in &results.packages {
                output.push_str(&formatters::format_package_text(
                    package,
                    self.use_colors,
                    self.verbose
                ));
            }
        }
        
        Ok(output)
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter {
    pub pretty: bool,
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }
}

impl Formatter for JsonFormatter {
    fn format(&self, results: &AnalysisResults) -> Result<String> {
        formatters::format_results_json(results)
    }
}

/// CSV formatter for spreadsheet analysis
pub struct CsvFormatter;

impl CsvFormatter {
    /// Create a new CSV formatter
    pub fn new() -> Self {
        Self {}
    }
}

impl Formatter for CsvFormatter {
    fn format(&self, results: &AnalysisResults) -> Result<String> {
        formatters::format_results_csv(results)
    }
}

/// Create a formatter based on the output format
pub fn create_formatter(
    format: &crate::models::config::OutputFormat,
    use_colors: bool,
    verbose: bool,
    quiet: bool,
) -> Box<dyn Formatter> {
    match format {
        crate::models::config::OutputFormat::Text => {
            Box::new(TextFormatter::new(use_colors, verbose, quiet))
        }
        crate::models::config::OutputFormat::Json => {
            Box::new(JsonFormatter::new(true)) // Use pretty printing by default
        }
        crate::models::config::OutputFormat::Csv => {
            Box::new(CsvFormatter::new())
        }
    }
}
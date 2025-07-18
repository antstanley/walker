//! Progress reporting functionality
//!
//! This module provides progress reporting for long-running operations
//! with support for quiet and verbose modes.

use crate::core::parallel::ProgressUpdate;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Progress reporter for long-running operations
pub struct ProgressReporter {
    quiet: bool,
    verbose: bool,
    multi_progress: Arc<MultiProgress>,
    main_progress_bar: Option<ProgressBar>,
    message_bar: Option<ProgressBar>,
    current_operation: Arc<Mutex<String>>,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(quiet: bool, verbose: bool) -> Self {
        let multi_progress = Arc::new(MultiProgress::new());
        
        // Don't create progress bars in quiet mode
        let (main_progress_bar, message_bar) = if quiet {
            (None, None)
        } else {
            let main_bar = multi_progress.add(ProgressBar::new(100));
            main_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            
            let msg_bar = multi_progress.add(ProgressBar::new(1));
            msg_bar.set_style(
                ProgressStyle::default_bar()
                    .template("{wide_msg}")
                    .unwrap()
            );
            
            (Some(main_bar), Some(msg_bar))
        };
        
        Self {
            quiet,
            verbose,
            multi_progress,
            main_progress_bar,
            message_bar,
            current_operation: Arc::new(Mutex::new(String::new())),
        }
    }
    
    /// Start a new progress operation
    pub fn start(&self, total: usize, operation: &str) {
        if self.quiet {
            return;
        }
        
        if let Some(bar) = &self.main_progress_bar {
            bar.reset();
            bar.set_length(total as u64);
            bar.set_position(0);
            bar.set_message(operation.to_string());
        }
        
        if let Some(msg_bar) = &self.message_bar {
            msg_bar.set_message(operation.to_string());
        }
        
        // Store the current operation
        if let Ok(mut current_op) = self.current_operation.lock() {
            *current_op = operation.to_string();
        }
        
        // Print verbose message
        if self.verbose {
            println!("Starting: {}", operation);
        }
    }
    
    /// Update progress
    pub fn update(&self, current: usize, total: usize, message: &str) {
        if self.quiet {
            return;
        }
        
        if let Some(bar) = &self.main_progress_bar {
            bar.set_length(total as u64);
            bar.set_position(current as u64);
            
            // Only update the message if it's different from the current operation
            if let Ok(current_op) = self.current_operation.lock() {
                if message != *current_op {
                    if let Some(msg_bar) = &self.message_bar {
                        msg_bar.set_message(message.to_string());
                    }
                }
            }
        }
        
        // Print verbose message
        if self.verbose {
            println!("[{}/{}] {}", current, total, message);
        }
    }
    
    /// Update progress from a ProgressUpdate
    pub fn update_from(&self, progress: ProgressUpdate) {
        self.update(progress.current, progress.total, &progress.message);
    }
    
    /// Finish the progress operation
    pub fn finish(&self, message: &str) {
        if self.quiet {
            return;
        }
        
        if let Some(bar) = &self.main_progress_bar {
            bar.finish_with_message(message.to_string());
        }
        
        if let Some(msg_bar) = &self.message_bar {
            msg_bar.finish_with_message(message.to_string());
        }
        
        // Print verbose message
        if self.verbose {
            println!("Finished: {}", message);
        }
    }
    
    /// Create a new progress bar for a specific operation
    pub fn create_spinner(&self, message: &str) -> Option<ProgressBar> {
        if self.quiet {
            return None;
        }
        
        let spinner = self.multi_progress.add(ProgressBar::new_spinner());
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));
        
        Some(spinner)
    }
    
    /// Create a progress bar for file operations
    pub fn create_file_progress_bar(&self, total_size: u64, message: &str) -> Option<ProgressBar> {
        if self.quiet {
            return None;
        }
        
        let bar = self.multi_progress.add(ProgressBar::new(total_size));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        bar.set_message(message.to_string());
        
        Some(bar)
    }
    
    /// Print a message (respects quiet mode)
    pub fn print(&self, message: &str) {
        if !self.quiet {
            println!("{}", message);
        }
    }
    
    /// Print a verbose message (only in verbose mode)
    pub fn print_verbose(&self, message: &str) {
        if self.verbose {
            println!("{}", message);
        }
    }
    
    /// Print a warning message (always printed, even in quiet mode)
    pub fn print_warning(&self, message: &str) {
        eprintln!("Warning: {}", message);
    }
    
    /// Print an error message (always printed, even in quiet mode)
    pub fn print_error(&self, message: &str) {
        eprintln!("Error: {}", message);
    }
    
    /// Check if quiet mode is enabled
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
    
    /// Check if verbose mode is enabled
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }
}

/// Create a progress callback function that updates a ProgressReporter
pub fn create_progress_callback(
    reporter: Arc<ProgressReporter>
) -> impl Fn(ProgressUpdate) + Send + Sync {
    move |progress: ProgressUpdate| {
        reporter.update_from(progress);
    }
}
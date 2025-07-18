//! Output writing functionality
//!
//! This module provides writers for different output destinations.

use crate::error::{Result, WalkerError};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Trait for output writers
pub trait OutputWriter {
    /// Write content to the output destination
    fn write(&self, content: &str) -> Result<()>;
}

/// Writer for stdout output
#[derive(Debug)]
pub struct StdoutWriter;

impl OutputWriter for StdoutWriter {
    fn write(&self, content: &str) -> Result<()> {
        print!("{}", content);
        io::stdout().flush().map_err(|e| WalkerError::StdoutWrite {
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

/// Writer for file output
#[derive(Debug)]
pub struct FileWriter {
    path: std::path::PathBuf,
}

impl FileWriter {
    /// Create a new file writer
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }
}

impl OutputWriter for FileWriter {
    fn write(&self, content: &str) -> Result<()> {
        let mut file = File::create(&self.path).map_err(|e| WalkerError::OutputWrite {
            path: self.path.clone(),
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;
        
        file.write_all(content.as_bytes()).map_err(|e| WalkerError::OutputWrite {
            path: self.path.clone(),
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

/// Create an output writer based on the output file option
pub fn create_writer(output_file: Option<impl AsRef<Path>>) -> Box<dyn OutputWriter> {
    match output_file {
        Some(path) => Box::new(FileWriter::new(path)),
        None => Box::new(StdoutWriter),
    }
}
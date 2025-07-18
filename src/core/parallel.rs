//! Parallel processing utilities

use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use crate::error::{Result, WalkerError};

/// Progress update information for parallel operations
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub current: usize,
    pub total: usize,
    pub message: String,
}

impl ProgressUpdate {
    /// Create a new progress update
    pub fn new(current: usize, total: usize, message: impl Into<String>) -> Self {
        Self {
            current,
            total,
            message: message.into(),
        }
    }

    /// Calculate progress percentage
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.current as f64 / self.total as f64) * 100.0
        }
    }
}

/// Execute a function in parallel on a collection of items
pub fn parallel_process<T, F, R>(items: Vec<T>, f: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Send + Sync,
{
    items.into_par_iter().map(f).collect()
}

/// Execute a function in parallel on a collection of items with error handling
pub fn parallel_process_with_errors<T, F, R>(items: Vec<T>, f: F) -> Result<Vec<R>>
where
    T: Send,
    R: Send,
    F: Fn(T) -> Result<R> + Send + Sync,
{
    let results: Result<Vec<_>> = items
        .into_par_iter()
        .map(f)
        .collect();
    
    results
}

/// Execute a function in parallel on a collection of items with progress reporting
pub fn parallel_process_with_progress<T, F, R, P>(
    items: Vec<T>,
    f: F,
    progress_callback: P,
) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Send + Sync,
    P: Fn(ProgressUpdate) + Send + Sync,
{
    let total = items.len();
    let counter = Arc::new(Mutex::new(0));
    
    items
        .into_par_iter()
        .map(|item| {
            let result = f(item);
            
            // Replace unwrap with proper error handling
            let mut count = match counter.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    // Since we can't return a Result from this closure in this function,
                    // we'll log the error and continue with a default value
                    eprintln!("Warning: Failed to lock counter mutex: {}", e);
                    return result;
                }
            };
            
            *count += 1;
            
            progress_callback(ProgressUpdate::new(
                *count,
                total,
                format!("Processing item {}/{}", *count, total),
            ));
            
            result
        })
        .collect()
}

/// Execute a function in parallel on a collection of items with progress reporting and error handling
pub fn parallel_process_with_progress_and_errors<T, F, R, P>(
    items: Vec<T>,
    f: F,
    progress_callback: P,
) -> Result<Vec<R>>
where
    T: Send,
    R: Send,
    F: Fn(T) -> Result<R> + Send + Sync,
    P: Fn(ProgressUpdate) + Send + Sync,
{
    let total = items.len();
    let counter = Arc::new(Mutex::new(0));
    
    let results: Result<Vec<_>> = items
        .into_par_iter()
        .map(|item| {
            let result = f(item);
            
            let mut count = match counter.lock() {
                Ok(guard) => guard,
                Err(_) => return Err(WalkerError::ParallelExecution {
                    message: "Failed to lock counter mutex".to_string(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                }),
            };
            
            *count += 1;
            
            progress_callback(ProgressUpdate::new(
                *count,
                total,
                format!("Processing item {}/{}", *count, total),
            ));
            
            result
        })
        .collect();
    
    results
}
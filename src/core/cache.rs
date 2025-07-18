//! Caching functionality for analysis results
//!
//! This module provides a thread-safe cache for storing package analysis results
//! to avoid re-analyzing identical packages.

use crate::error::{Result, WalkerError};
use crate::models::analysis::PackageAnalysis;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Cache for storing analysis results
#[derive(Clone)]
pub struct Cache {
    cache: HashMap<PathBuf, CacheEntry>,
    max_size: usize,
    ttl: Option<Duration>,
    hits: usize,
    misses: usize,
}

/// Cache entry with timestamp for TTL-based expiration
#[derive(Clone)]
struct CacheEntry {
    analysis: PackageAnalysis,
    timestamp: Instant,
}

impl Cache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_size: 1000, // Default max size
            ttl: None,      // No expiration by default
            hits: 0,
            misses: 0,
        }
    }

    /// Create a new cache with specified maximum size and TTL
    pub fn with_options(max_size: usize, ttl_seconds: Option<u64>) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            ttl: ttl_seconds.map(Duration::from_secs),
            hits: 0,
            misses: 0,
        }
    }

    /// Get a cached analysis result
    pub fn get(&self, path: &PathBuf) -> Option<PackageAnalysis> {
        match self.cache.get(path) {
            Some(entry) => {
                // Check if the entry has expired
                if let Some(ttl) = self.ttl {
                    if entry.timestamp.elapsed() > ttl {
                        return None;
                    }
                }
                
                // Return a clone of the cached analysis
                Some(entry.analysis.clone())
            }
            None => None,
        }
    }

    /// Store an analysis result in the cache
    pub fn insert(&mut self, path: PathBuf, analysis: PackageAnalysis) {
        // Evict entries if we're at capacity
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }
        
        // Create a new cache entry with current timestamp
        let entry = CacheEntry {
            analysis,
            timestamp: Instant::now(),
        };
        
        // Insert the entry
        self.cache.insert(path, entry);
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.len(), self.hits, self.misses)
    }
    
    /// Evict the oldest entries when cache is full
    fn evict_oldest(&mut self) {
        // Find the oldest entry
        if let Some((oldest_key, _)) = self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp) {
            // Clone the key so we can remove it
            let key_to_remove = oldest_key.clone();
            self.cache.remove(&key_to_remove);
        }
    }
    
    /// Remove expired entries
    pub fn cleanup_expired(&mut self) -> usize {
        if let Some(ttl) = self.ttl {
            let now = Instant::now();
            let expired_keys: Vec<PathBuf> = self.cache
                .iter()
                .filter(|(_, entry)| now.duration_since(entry.timestamp) > ttl)
                .map(|(key, _)| key.clone())
                .collect();
            
            let count = expired_keys.len();
            for key in expired_keys {
                self.cache.remove(&key);
            }
            
            count
        } else {
            0
        }
    }
    
    /// Record a cache hit
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }
    
    /// Record a cache miss
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }
    
    /// Get the cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe cache wrapper using RwLock
pub struct ThreadSafeCache {
    inner: RwLock<Cache>,
}

impl ThreadSafeCache {
    /// Create a new thread-safe cache
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Cache::new()),
        }
    }
    
    /// Create a new thread-safe cache with options
    pub fn with_options(max_size: usize, ttl_seconds: Option<u64>) -> Self {
        Self {
            inner: RwLock::new(Cache::with_options(max_size, ttl_seconds)),
        }
    }
    
    /// Get a cached analysis result
    pub fn get(&self, path: &PathBuf) -> Result<Option<PackageAnalysis>> {
        match self.inner.read() {
            Ok(cache) => {
                let result = cache.get(path);
                Ok(result)
            }
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire read lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Store an analysis result in the cache
    pub fn insert(&self, path: PathBuf, analysis: PackageAnalysis) -> Result<()> {
        match self.inner.write() {
            Ok(mut cache) => {
                cache.insert(path, analysis);
                Ok(())
            }
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire write lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Clear the cache
    pub fn clear(&self) -> Result<()> {
        match self.inner.write() {
            Ok(mut cache) => {
                cache.clear();
                Ok(())
            }
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire write lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Get the number of cached entries
    pub fn len(&self) -> Result<usize> {
        match self.inner.read() {
            Ok(cache) => Ok(cache.len()),
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire read lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Check if the cache is empty
    pub fn is_empty(&self) -> Result<bool> {
        match self.inner.read() {
            Ok(cache) => Ok(cache.is_empty()),
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire read lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> Result<(usize, usize, usize)> {
        match self.inner.read() {
            Ok(cache) => Ok(cache.stats()),
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire read lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
    
    /// Remove expired entries
    pub fn cleanup_expired(&self) -> Result<usize> {
        match self.inner.write() {
            Ok(mut cache) => Ok(cache.cleanup_expired()),
            Err(_) => Err(WalkerError::Cache {
                message: "Failed to acquire write lock on cache".to_string(),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }
}
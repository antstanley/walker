//! AST parser wrapper using OXC

use crate::error::{Result, WalkerError};
use crate::models::file_metadata::{ExportedSymbol, ImportedSymbol, ModuleSystem };
use oxc_allocator::Allocator;
use oxc_diagnostics::OxcDiagnostic;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;
use parking_lot::RwLock;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// Thread-safe allocator pool for reuse across parses
pub struct AllocatorPool {
    allocators: Arc<RwLock<Vec<Allocator>>>,
}

impl AllocatorPool {
    /// Create a new allocator pool
    pub fn new(size: usize) -> Self {
        let mut allocators = Vec::with_capacity(size);
        for _ in 0..size {
            allocators.push(Allocator::default());
        }
        Self {
            allocators: Arc::new(RwLock::new(allocators)),
        }
    }

    /// Take an allocator from the pool
    pub fn take(&self) -> Option<Allocator> {
        self.allocators.write().pop()
    }

    /// Return an allocator to the pool
    pub fn return_allocator(&self, allocator: Allocator) {
        self.allocators.write().push(allocator);
    }
}

/// AST parser using OXC
pub struct ASTParser {
    parse_options: ParseOptions,
    allocator_pool: AllocatorPool,
}

impl ASTParser {
    /// Create a new AST parser
    pub fn new() -> Self {
        Self {
            parse_options: ParseOptions {
                parse_regular_expression: true,
                ..ParseOptions::default()
            },
            allocator_pool: AllocatorPool::new(num_cpus::get()),
        }
    }

    /// Parse a JavaScript/TypeScript file and extract needed data immediately
    /// This avoids lifetime issues by processing the AST while the allocator is alive
    pub fn parse_and_analyze(&self, path: &Path) -> Result<FileAnalysis> {
        // Read the source file
        let source_text = fs::read_to_string(path).map_err(|e| WalkerError::IoRead {
            path: path.to_path_buf(),
            source: e,
            #[cfg(not(tarpaulin_include))]
            backtrace: std::backtrace::Backtrace::capture(),
        })?;

        // Determine source type from file extension
        let source_type = SourceType::from_path(path).unwrap();

        // Get or create an allocator
        let allocator = self
            .allocator_pool
            .take()
            .unwrap_or_else(|| Allocator::default());

        // Parse the source code
        let ret = Parser::new(&allocator, &source_text, source_type)
            .with_options(self.parse_options.clone())
            .parse();

        // Process AST immediately while allocator is alive
        let analysis = if ret.errors.is_empty() {
            // Use the module detector to analyze the AST
            super::module_detector::ModuleDetector::analyze(&ret.program)
        } else {
            // Handle parse errors gracefully
            ModuleSystemAnalysis::with_errors(ret.errors)
        };

        // Return allocator to pool for reuse
        self.allocator_pool.return_allocator(allocator);

        Ok(FileAnalysis {
            path: path.to_path_buf(),
            module_system: analysis.module_system,
            imports: analysis.imports,
            exports: analysis.exports,
            has_errors: analysis.has_errors,
            parse_errors: analysis.parse_errors,
            source_text,
            circular_dependencies: analysis.circular_dependencies,
        })
    }
}

impl Default for ASTParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracted analysis data that doesn't depend on AST lifetimes
#[derive(Debug, Clone)]
pub struct FileAnalysis {
    pub path: std::path::PathBuf,
    pub module_system: ModuleSystem,
    pub imports: Vec<ImportedSymbol>,
    pub exports: Vec<ExportedSymbol>,
    pub has_errors: bool,
    pub parse_errors: Vec<String>,
    pub source_text: String,
    pub circular_dependencies: std::collections::HashSet<std::path::PathBuf>,
}

/// Results from module system analysis
#[derive(Debug, Clone)]
pub struct ModuleSystemAnalysis {
    pub module_system: ModuleSystem,
    pub imports: Vec<ImportedSymbol>,
    pub exports: Vec<ExportedSymbol>,
    pub circular_dependencies: std::collections::HashSet<std::path::PathBuf>,
    pub has_errors: bool,
    pub parse_errors: Vec<String>,
}

impl ModuleSystemAnalysis {
    /// Create analysis results from parse errors
    pub fn with_errors(errors: Vec<OxcDiagnostic>) -> Self {
        Self {
            module_system: ModuleSystem::Unknown,
            imports: Vec::new(),
            exports: Vec::new(),
            circular_dependencies: std::collections::HashSet::new(),
            has_errors: true,
            parse_errors: errors.iter().map(|e| format!("{}", e)).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_esm_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.js");
        fs::write(
            &file_path,
            r#"
            import { foo } from './foo.js';
            export const bar = 42;
            "#,
        )
        .unwrap();

        let parser = ASTParser::new();
        let result = parser.parse_and_analyze(&file_path).unwrap();

        assert_eq!(result.module_system, ModuleSystem::ESM);
        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.exports.len(), 1);
    }

    #[test]
    fn test_parse_cjs_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.js");
        fs::write(
            &file_path,
            r#"
            const foo = require('./foo.js');
            module.exports = { bar: 42 };
            "#,
        )
        .unwrap();

        let parser = ASTParser::new();
        let result = parser.parse_and_analyze(&file_path).unwrap();

        assert_eq!(result.module_system, ModuleSystem::CommonJS);
    }

    #[test]
    fn test_parse_mixed_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.js");
        fs::write(
            &file_path,
            r#"
            import { foo } from './foo.js';
            const bar = require('./bar.js');
            export default 42;
            "#,
        )
        .unwrap();

        let parser = ASTParser::new();
        let result = parser.parse_and_analyze(&file_path).unwrap();

        assert_eq!(result.module_system, ModuleSystem::Mixed);
    }
}

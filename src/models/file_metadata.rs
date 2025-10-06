//! File-level metadata for AST analysis

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// Metadata for a single file analyzed via AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Relative path from package root
    pub relative_path: PathBuf,

    /// Absolute path
    pub absolute_path: PathBuf,

    /// File name
    pub file_name: String,

    /// File type (.js, .mjs, .cjs, .ts, .tsx, etc.)
    pub file_type: FileType,

    /// File size in bytes
    pub size_bytes: u64,

    /// Created timestamp
    pub created: Option<SystemTime>,

    /// Modified timestamp
    pub modified: Option<SystemTime>,

    /// Module system used in this file
    pub module_system: ModuleSystem,

    /// Whether this file is part of core package or a dependency
    pub file_location: FileLocation,

    /// Exports from this file
    pub exports: Vec<ExportedSymbol>,

    /// Imports in this file
    pub imports: Vec<ImportedSymbol>,

    /// Whether this file is referenced in the dependency graph
    pub is_referenced: bool,

    /// Number of times this file is imported
    pub reference_count: usize,
}

/// Location of the file within the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileLocation {
    /// File is part of the core package
    CorePackage,
    /// File is part of a dependency
    Dependency(String), // Dependency package name
}

/// Module system detected in the file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModuleSystem {
    /// ECMAScript Modules
    ESM,
    /// CommonJS
    CommonJS,
    /// Mixed ESM and CommonJS
    Mixed,
    /// Unknown or no module system detected
    Unknown,
}

/// Type of JavaScript/TypeScript file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    JavaScript,
    JavaScriptModule,  // .mjs
    JavaScriptCommon,  // .cjs
    TypeScript,
    TypeScriptReact,   // .tsx
    JavaScriptReact,   // .jsx
    TypeScriptDeclaration, // .d.ts
    Json,
    Other(String),
}

impl FileType {
    /// Determine file type from extension
    pub fn from_path(path: &std::path::Path) -> Self {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let file_name = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        // Check for .d.ts files
        if file_name.ends_with(".d.ts") {
            return FileType::TypeScriptDeclaration;
        }

        match extension {
            "js" => FileType::JavaScript,
            "mjs" => FileType::JavaScriptModule,
            "cjs" => FileType::JavaScriptCommon,
            "ts" => FileType::TypeScript,
            "tsx" => FileType::TypeScriptReact,
            "jsx" => FileType::JavaScriptReact,
            "json" => FileType::Json,
            ext => FileType::Other(ext.to_string()),
        }
    }

    /// Check if this is a TypeScript file
    pub fn is_typescript(&self) -> bool {
        matches!(
            self,
            FileType::TypeScript | FileType::TypeScriptReact | FileType::TypeScriptDeclaration
        )
    }

    /// Check if this is a JavaScript file
    pub fn is_javascript(&self) -> bool {
        matches!(
            self,
            FileType::JavaScript | FileType::JavaScriptModule | FileType::JavaScriptCommon | FileType::JavaScriptReact
        )
    }
}

/// An exported symbol from a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSymbol {
    /// Name of the export
    pub name: String,
    /// Type of the symbol
    pub symbol_type: SymbolType,
    /// Whether this is the default export
    pub is_default: bool,
    /// Line number where the export is defined
    pub line_number: usize,
}

/// An imported symbol in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportedSymbol {
    /// Source module (e.g., "./utils" or "lodash")
    pub source: String,
    /// Names of imported symbols
    pub imported_names: Vec<String>,
    /// Whether this is a dynamic import
    pub is_dynamic: bool,
    /// Line number of the import statement
    pub line_number: usize,
}

/// Type of a symbol (export/import)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    Function,
    Class,
    Variable,
    Type,
    Interface,
    Namespace,
    Enum,
    Unknown,
}

impl FileMetadata {
    /// Calculate complexity score for the file
    pub fn complexity_score(&self) -> usize {
        let import_complexity = self.imports.len();
        let export_complexity = self.exports.len();
        let module_complexity = match self.module_system {
            ModuleSystem::Mixed => 5,
            ModuleSystem::Unknown => 3,
            _ => 1,
        };

        import_complexity + export_complexity + module_complexity
    }

    /// Check if file has side effects (heuristic)
    pub fn has_side_effects(&self) -> bool {
        // Files with no exports might have side effects
        if self.exports.is_empty() && !self.imports.is_empty() {
            return true;
        }

        // CommonJS files often have side effects
        if self.module_system == ModuleSystem::CommonJS {
            return true;
        }

        false
    }

    /// Check if file is tree-shakeable
    pub fn is_tree_shakeable(&self) -> bool {
        // ESM with named exports is tree-shakeable
        if self.module_system == ModuleSystem::ESM {
            return self.exports.iter().any(|e| !e.is_default);
        }

        false
    }
}
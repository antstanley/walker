//! Exports field parsing functionality
//!
//! This module provides comprehensive parsing for the exports field in package.json,
//! handling various formats including conditional exports.

use crate::models::package::{CjsSupport, EsmSupport, ModuleSupport};
use serde_json::Value;
use std::collections::HashSet;

/// Parser for package.json exports field
pub struct ExportsParser;

impl ExportsParser {
    /// Parse exports field to determine module support
    pub fn parse_exports(exports: &Value) -> ModuleSupport {
        let mut support = ModuleSupport::default();
        
        // Check for import/require conditions in exports
        let has_import = Self::exports_has_condition(exports, "import");
        let has_require = Self::exports_has_condition(exports, "require");
        
        // Check for file extensions in exports paths
        let extensions = Self::collect_export_extensions(exports);
        let has_mjs = extensions.contains(".mjs");
        let has_cjs = extensions.contains(".cjs");
        
        // Update ESM support
        support.esm.exports_import = has_import;
        if has_import || has_mjs {
            support.esm.overall = true;
        }
        
        // Update CJS support
        support.cjs.exports_require = has_require;
        if has_require || has_cjs {
            support.cjs.overall = true;
        }
        
        support
    }
    
    /// Check if exports field has a specific condition
    pub fn exports_has_condition(exports: &Value, condition: &str) -> bool {
        match exports {
            Value::Object(map) => {
                // Check for direct condition
                if map.contains_key(condition) {
                    return true;
                }
                
                // Check for nested conditions
                for (_, value) in map {
                    if let Value::Object(inner_map) = value {
                        if inner_map.contains_key(condition) {
                            return true;
                        }
                        
                        // Check for deeper nesting
                        for (_, inner_value) in inner_map {
                            if let Value::Object(deeper_map) = inner_value {
                                if deeper_map.contains_key(condition) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                
                false
            },
            _ => false,
        }
    }
    
    /// Collect all file extensions from export paths
    pub fn collect_export_extensions(exports: &Value) -> HashSet<String> {
        let mut extensions = HashSet::new();
        Self::extract_extensions_recursive(exports, &mut extensions);
        extensions
    }
    
    /// Recursively extract file extensions from export paths
    fn extract_extensions_recursive(value: &Value, extensions: &mut HashSet<String>) {
        match value {
            Value::String(path) => {
                if let Some(ext) = Self::extract_extension(path) {
                    extensions.insert(ext);
                }
            },
            Value::Object(map) => {
                for (_, value) in map {
                    Self::extract_extensions_recursive(value, extensions);
                }
            },
            Value::Array(arr) => {
                for value in arr {
                    Self::extract_extensions_recursive(value, extensions);
                }
            },
            _ => {}
        }
    }
    
    /// Extract file extension from a path string
    fn extract_extension(path: &str) -> Option<String> {
        let path = path.trim();
        
        // Skip URLs and special paths
        if path.starts_with("http://") || path.starts_with("https://") || path == "." || path == ".." {
            return None;
        }
        
        // Find last dot after the last slash
        let last_slash = path.rfind('/').map(|i| i + 1).unwrap_or(0);
        let suffix = &path[last_slash..];
        
        if let Some(dot_pos) = suffix.rfind('.') {
            Some(suffix[dot_pos..].to_string())
        } else {
            None
        }
    }
    
    /// Parse a package.json exports field to determine module support
    pub fn analyze_exports(exports: &Value) -> (bool, bool, bool, bool) {
        let mut has_esm = false;
        let mut has_cjs = false;
        let mut has_typescript = false;
        let mut has_browser = false;
        
        // Check for import/require conditions
        if Self::exports_has_condition(exports, "import") {
            has_esm = true;
        }
        
        if Self::exports_has_condition(exports, "require") {
            has_cjs = true;
        }
        
        // Check for browser condition
        if Self::exports_has_condition(exports, "browser") {
            has_browser = true;
        }
        
        // Check for types condition
        if Self::exports_has_condition(exports, "types") {
            has_typescript = true;
        }
        
        // Check file extensions
        let extensions = Self::collect_export_extensions(exports);
        
        if extensions.contains(".mjs") {
            has_esm = true;
        }
        
        if extensions.contains(".cjs") {
            has_cjs = true;
        }
        
        if extensions.contains(".d.ts") || extensions.contains(".d.mts") || extensions.contains(".d.cts") {
            has_typescript = true;
        }
        
        // If no specific indicators, check for default exports
        if !has_esm && !has_cjs {
            // If exports field exists but doesn't specify module type,
            // it's likely supporting both or at least ESM
            has_esm = true;
            
            // Most packages maintain CJS compatibility
            has_cjs = true;
        }
        
        (has_esm, has_cjs, has_typescript, has_browser)
    }
    
    /// Create a ModuleSupport instance from package.json exports field
    pub fn create_module_support(exports: &Value, package_type: Option<&str>) -> ModuleSupport {
        use crate::models::package::{TypeScriptSupport, BrowserSupport};
        
        let mut esm_support = EsmSupport::default();
        let mut cjs_support = CjsSupport::default();
        let mut typescript_support = TypeScriptSupport::default();
        let mut browser_support = BrowserSupport::default();
        
        // Check package type
        if let Some(pkg_type) = package_type {
            if pkg_type == "module" {
                esm_support.type_module = true;
            } else if pkg_type == "commonjs" {
                cjs_support.type_commonjs = true;
            }
        }
        
        // Analyze exports field
        let (has_esm, has_cjs, has_typescript, has_browser) = Self::analyze_exports(exports);
        
        esm_support.exports_import = has_esm;
        cjs_support.exports_require = has_cjs;
        typescript_support.exports_dts = has_typescript;
        browser_support.exports_browser = has_browser;
        
        // Set default CJS support (most packages support CJS by default unless explicitly ESM-only)
        if !esm_support.type_module {
            cjs_support.default_support = true;
        }
        
        // Calculate overall support flags
        esm_support.overall = esm_support.type_module || esm_support.exports_import;
        cjs_support.overall = cjs_support.type_commonjs || cjs_support.exports_require || cjs_support.default_support;
        typescript_support.overall = typescript_support.exports_dts;
        browser_support.overall = browser_support.exports_browser;
        
        ModuleSupport {
            esm: esm_support,
            cjs: cjs_support,
            typescript: typescript_support,
            browser: browser_support,
        }
    }
}
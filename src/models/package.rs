//! Package-related data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Comprehensive package details extracted from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDetails {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub main: Option<String>,
    pub module: Option<String>,
    pub types: Option<String>,
    pub typings: Option<String>,
    pub browser: Option<serde_json::Value>,
    pub exports: Option<serde_json::Value>,
    pub package_type: Option<String>,
    pub engines: Option<serde_json::Value>,
    pub license: Option<String>,
    pub author: Option<AuthorInfo>,
    pub repository: Option<RepositoryInfo>,
    pub homepage: Option<String>,
    pub bugs: Option<BugsInfo>,
    pub keywords: Option<Vec<String>>,
    pub bin: Option<serde_json::Value>,
    pub scripts: Option<HashMap<String, String>>,
    pub dependencies: Option<serde_json::Value>,
    pub dev_dependencies: Option<serde_json::Value>,
    pub peer_dependencies: Option<serde_json::Value>,
    pub optional_dependencies: Option<serde_json::Value>,
    pub bundled_dependencies: Option<Vec<String>>,
    pub funding: Option<serde_json::Value>,
    pub private: Option<bool>,
    pub publish_config: Option<serde_json::Value>,
    pub os: Option<Vec<String>>,
    pub cpu: Option<Vec<String>>,
    pub workspaces: Option<serde_json::Value>,
    pub files: Option<Vec<String>>,
    pub man: Option<serde_json::Value>,
    pub directories: Option<serde_json::Value>,
    pub side_effects: Option<serde_json::Value>,
}

/// Author information from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthorInfo {
    String(String),
    Object {
        name: String,
        email: Option<String>,
        url: Option<String>,
    },
}

/// Repository information from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RepositoryInfo {
    String(String),
    Object {
        #[serde(rename = "type")]
        repo_type: String,
        url: String,
        directory: Option<String>,
    },
}

/// Bugs information from package.json
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BugsInfo {
    String(String),
    Object {
        url: String,
        email: Option<String>,
    },
}

impl Default for PackageDetails {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: None,
            main: None,
            module: None,
            types: None,
            typings: None,
            browser: None,
            exports: None,
            package_type: None,
            engines: None,
            license: None,
            author: None,
            repository: None,
            homepage: None,
            bugs: None,
            keywords: None,
            bin: None,
            scripts: None,
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            bundled_dependencies: None,
            funding: None,
            private: None,
            publish_config: None,
            os: None,
            cpu: None,
            workspaces: None,
            files: None,
            man: None,
            directories: None,
            side_effects: None,
        }
    }
}

impl PackageDetails {
    /// Get the Node.js version requirement as a string
    pub fn node_version_requirement(&self) -> Option<String> {
        if let Some(engines) = &self.engines {
            if let Some(node) = engines.get("node").and_then(|v| v.as_str()) {
                return Some(node.to_string());
            }
        }
        None
    }
}

/// Module system support detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSupport {
    pub esm: EsmSupport,
    pub cjs: CjsSupport,
    pub typescript: TypeScriptSupport,
    pub browser: BrowserSupport,
}

impl Default for ModuleSupport {
    fn default() -> Self {
        Self {
            esm: EsmSupport::default(),
            cjs: CjsSupport::default(),
            typescript: TypeScriptSupport::default(),
            browser: BrowserSupport::default(),
        }
    }
}

impl ModuleSupport {
    /// Create a new ModuleSupport instance from package details
    pub fn from_package_details(details: &PackageDetails) -> Self {
        use crate::parsers::ExportsParser;

        let mut support = Self::default();

        // Check ESM support
        if let Some(pkg_type) = &details.package_type {
            if pkg_type == "module" {
                support.esm.type_module = true;
            } else if pkg_type == "commonjs" {
                support.cjs.type_commonjs = true;
            }
        }

        // Check module field for ESM
        if details.module.is_some() {
            support.esm.module_field = true;
        }

        // Check main field for .mjs extension
        if let Some(main) = &details.main {
            if main.ends_with(".mjs") {
                support.esm.main_mjs = true;
            } else if main.ends_with(".cjs") {
                support.cjs.exports_require = true;
            }
        }

        // Check exports field for ESM/CJS/TypeScript/Browser support using the ExportsParser
        if let Some(exports) = &details.exports {
            // Use the ExportsParser's comprehensive analysis
            let (has_esm, has_cjs, has_typescript, has_browser) = ExportsParser::analyze_exports(exports);

            support.esm.exports_import = has_esm;
            support.cjs.exports_require = has_cjs;
            support.typescript.exports_dts = has_typescript;
            support.browser.exports_browser = has_browser;
        }

        // Set default CJS support (most packages support CJS by default unless explicitly ESM-only)
        support.cjs.default_support = !support.esm.type_module;

        // Check TypeScript support via dedicated fields
        if details.types.is_some() {
            support.typescript.types_field = true;
        }

        if details.typings.is_some() {
            support.typescript.typings_field = true;
        }

        // Check browser support via browser field
        if details.browser.is_some() {
            support.browser.browser_field = true;

            // Analyze browser field for more detailed browser support info
            if let Some(browser) = &details.browser {
                match browser {
                    serde_json::Value::Object(map) => {
                        // If browser field maps to .mjs files, it likely supports ESM in browser
                        for (_, target) in map {
                            if let Some(target_str) = target.as_str() {
                                if target_str.ends_with(".mjs") {
                                    support.esm.exports_import = true;
                                    break;
                                }
                            }
                        }
                    },
                    serde_json::Value::String(s) => {
                        // If browser field is a string pointing to .mjs, it supports ESM in browser
                        if s.ends_with(".mjs") {
                            support.esm.exports_import = true;
                        }
                    },
                    _ => {}
                }
            }
        }

        // Calculate overall support flags
        support.esm.overall = support.esm.type_module ||
                             support.esm.exports_import ||
                             support.esm.module_field ||
                             support.esm.main_mjs;

        support.cjs.overall = support.cjs.type_commonjs ||
                             support.cjs.exports_require ||
                             support.cjs.default_support;

        support.typescript.overall = support.typescript.types_field ||
                                    support.typescript.typings_field ||
                                    support.typescript.exports_dts;

        support.browser.overall = support.browser.browser_field ||
                                 support.browser.exports_browser;

        support
    }

    /// Check if the package is dual-mode (supports both ESM and CJS)
    pub fn is_dual_mode(&self) -> bool {
        self.esm.overall && self.cjs.overall
    }

    /// Check if the package is ESM-only
    pub fn is_esm_only(&self) -> bool {
        self.esm.overall && !self.cjs.overall
    }

    /// Check if the package is CJS-only
    pub fn is_cjs_only(&self) -> bool {
        !self.esm.overall && self.cjs.overall
    }

    /// Check if the package has no module system support detected
    pub fn has_no_support(&self) -> bool {
        !self.esm.overall && !self.cjs.overall
    }

    /// Check if the package has TypeScript support
    pub fn has_typescript_support(&self) -> bool {
        self.typescript.overall
    }

    /// Check if the package has browser support
    pub fn has_browser_support(&self) -> bool {
        self.browser.overall
    }
}

/// ESM (ECMAScript Modules) support details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EsmSupport {
    pub type_module: bool,
    pub exports_import: bool,
    pub module_field: bool,
    pub main_mjs: bool,
    pub overall: bool,
}

impl Default for EsmSupport {
    fn default() -> Self {
        Self {
            type_module: false,
            exports_import: false,
            module_field: false,
            main_mjs: false,
            overall: false,
        }
    }
}

/// CommonJS support details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CjsSupport {
    pub type_commonjs: bool,
    pub exports_require: bool,
    pub default_support: bool,
    pub overall: bool,
}

impl Default for CjsSupport {
    fn default() -> Self {
        Self {
            type_commonjs: false,
            exports_require: false,
            default_support: false,
            overall: false,
        }
    }
}

/// TypeScript support details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeScriptSupport {
    pub types_field: bool,
    pub typings_field: bool,
    pub exports_dts: bool,
    pub overall: bool,
}

impl Default for TypeScriptSupport {
    fn default() -> Self {
        Self {
            types_field: false,
            typings_field: false,
            exports_dts: false,
            overall: false,
        }
    }
}

/// Browser support details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSupport {
    pub browser_field: bool,
    pub exports_browser: bool,
    pub overall: bool,
}

impl Default for BrowserSupport {
    fn default() -> Self {
        Self {
            browser_field: false,
            exports_browser: false,
            overall: false,
        }
    }
}

/// Dependency information and counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub production_count: usize,
    pub development_count: usize,
    pub peer_count: usize,
    pub optional_count: usize,
    pub total_count: usize,
    pub production_deps: Option<Vec<DependencyEntry>>,
    pub dev_deps: Option<Vec<DependencyEntry>>,
    pub peer_deps: Option<Vec<DependencyEntry>>,
    pub optional_deps: Option<Vec<DependencyEntry>>,
}

/// Individual dependency entry with name and version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEntry {
    pub name: String,
    pub version_spec: String,
}

impl Default for DependencyInfo {
    fn default() -> Self {
        Self {
            production_count: 0,
            development_count: 0,
            peer_count: 0,
            optional_count: 0,
            total_count: 0,
            production_deps: None,
            dev_deps: None,
            peer_deps: None,
            optional_deps: None,
        }
    }
}

impl DependencyInfo {
    /// Create a new DependencyInfo instance from package details
    pub fn from_package_details(details: &PackageDetails) -> Self {
        use crate::parsers::PackageJsonParser;

        // Use the PackageJsonParser to extract dependency information
        PackageJsonParser::extract_dependency_info(details)
    }

    /// Get the list of all dependency names
    pub fn all_dependency_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        if let Some(deps) = &self.production_deps {
            names.extend(deps.iter().map(|d| d.name.clone()));
        }

        if let Some(deps) = &self.dev_deps {
            names.extend(deps.iter().map(|d| d.name.clone()));
        }

        if let Some(deps) = &self.peer_deps {
            names.extend(deps.iter().map(|d| d.name.clone()));
        }

        if let Some(deps) = &self.optional_deps {
            names.extend(deps.iter().map(|d| d.name.clone()));
        }

        names
    }
}

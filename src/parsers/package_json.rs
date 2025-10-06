//! Package.json parsing functionality
//!
//! This module provides comprehensive parsing for package.json files,
//! extracting all relevant fields and handling various formats.

use crate::error::{Result, WalkerError};
use crate::models::package::{
    AuthorInfo, BugsInfo, DependencyEntry, DependencyInfo, PackageDetails, RepositoryInfo,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

/// Parser for package.json files
pub struct PackageJsonParser;

impl PackageJsonParser {
    /// Parse package.json content into PackageDetails
    pub fn parse(content: &str) -> Result<PackageDetails> {
        // Parse JSON content
        let json_value: Value = serde_json::from_str(content)
            .map_err(|e| WalkerError::json_parse_error("package.json", e))?;

        // Ensure we have an object
        let obj = match json_value {
            Value::Object(obj) => obj,
            _ => {
                return Err(WalkerError::InvalidPackageJson {
                    path: "package.json".into(),
                    message: "Root value is not an object".into(),
                    #[cfg(not(tarpaulin_include))]
                    backtrace: std::backtrace::Backtrace::capture(),
                });
            }
        };

        // Extract required fields
        // let name = Self::extract_string_field(&obj, "name")?;
        // let version = Self::extract_string_field(&obj, "version")?;

        // Create package details with required fields
        let mut details = PackageDetails {
            ..Default::default()
        };

        // Extract optional fields
        details.description = Self::extract_optional_string(&obj, "description");
        details.main = Self::extract_optional_string(&obj, "main");
        details.module = Self::extract_optional_string(&obj, "module");
        details.types = Self::extract_optional_string(&obj, "types");
        details.typings = Self::extract_optional_string(&obj, "typings");
        details.browser = Self::extract_optional_value(&obj, "browser");
        details.exports = Self::extract_optional_value(&obj, "exports");
        details.package_type = Self::extract_optional_string(&obj, "type");
        details.engines = Self::extract_optional_value(&obj, "engines");
        details.license = Self::extract_optional_string(&obj, "license");
        details.homepage = Self::extract_optional_string(&obj, "homepage");
        details.private = Self::extract_optional_bool(&obj, "private");
        details.keywords = Self::extract_optional_string_array(&obj, "keywords");
        details.bin = Self::extract_optional_value(&obj, "bin");
        details.os = Self::extract_optional_string_array(&obj, "os");
        details.cpu = Self::extract_optional_string_array(&obj, "cpu");
        details.workspaces = Self::extract_optional_value(&obj, "workspaces");
        details.files = Self::extract_optional_string_array(&obj, "files");
        details.man = Self::extract_optional_value(&obj, "man");
        details.directories = Self::extract_optional_value(&obj, "directories");
        details.side_effects = Self::extract_optional_value(&obj, "side_effects");
        details.funding = Self::extract_optional_value(&obj, "funding");
        details.publish_config = Self::extract_optional_value(&obj, "publish_config");

        // Extract complex fields
        details.author = Self::extract_author(&obj);
        details.repository = Self::extract_repository(&obj);
        details.bugs = Self::extract_bugs(&obj);

        // Extract scripts
        if let Some(scripts_value) = obj.get("scripts") {
            if let Value::Object(scripts_obj) = scripts_value {
                let mut scripts = HashMap::new();
                for (key, value) in scripts_obj {
                    if let Value::String(script) = value {
                        scripts.insert(key.clone(), script.clone());
                    }
                }
                if !scripts.is_empty() {
                    details.scripts = Some(scripts);
                }
            }
        }

        // Extract dependencies
        details.dependencies = Self::extract_optional_value(&obj, "dependencies");
        details.dev_dependencies = Self::extract_optional_value(&obj, "devDependencies");
        details.peer_dependencies = Self::extract_optional_value(&obj, "peerDependencies");
        details.optional_dependencies = Self::extract_optional_value(&obj, "optionalDependencies");
        details.bundled_dependencies = Self::extract_optional_string_array(&obj, "bundledDependencies")
            .or_else(|| Self::extract_optional_string_array(&obj, "bundleDependencies"));

        Ok(details)
    }

    /// Parse package.json file from a path
    pub fn parse_file(path: &Path) -> Result<PackageDetails> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| WalkerError::io_error(e))?;

        Self::parse(&content).map_err(|e| match e {
            WalkerError::JsonParse { source, .. } => WalkerError::JsonParse {
                file: path.to_path_buf(),
                source,
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            },
            WalkerError::InvalidPackageJson { message, .. } => WalkerError::InvalidPackageJson {
                path: path.to_path_buf(),
                message,
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            },
            _ => e,
        })
    }

    /// Extract dependency information from package details
    pub fn extract_dependency_info(details: &PackageDetails) -> DependencyInfo {
        let mut info = DependencyInfo::default();

        // Process production dependencies
        if let Some(deps) = &details.dependencies {
            let entries = Self::extract_dependencies(deps);
            info.production_count = entries.len();
            info.production_deps = Some(entries);
        }

        // Process development dependencies
        if let Some(deps) = &details.dev_dependencies {
            let entries = Self::extract_dependencies(deps);
            info.development_count = entries.len();
            info.dev_deps = Some(entries);
        }

        // Process peer dependencies
        if let Some(deps) = &details.peer_dependencies {
            let entries = Self::extract_dependencies(deps);
            info.peer_count = entries.len();
            info.peer_deps = Some(entries);
        }

        // Process optional dependencies
        if let Some(deps) = &details.optional_dependencies {
            let entries = Self::extract_dependencies(deps);
            info.optional_count = entries.len();
            info.optional_deps = Some(entries);
        }

        // Calculate total count
        info.total_count = info.production_count + info.development_count +
                          info.peer_count + info.optional_count;

        info
    }

    /// Extract dependencies from a JSON value
    fn extract_dependencies(deps_json: &Value) -> Vec<DependencyEntry> {
        let mut entries = Vec::new();

        if let Value::Object(map) = deps_json {
            for (name, version) in map {
                if let Some(version_str) = version.as_str() {
                    entries.push(DependencyEntry {
                        name: name.clone(),
                        version_spec: version_str.to_string(),
                    });
                }
            }
        }

        entries
    }

    /// Extract a required string field from a JSON object
    fn extract_string_field(obj: &Map<String, Value>, field: &str) -> Result<String> {
        match obj.get(field) {
            Some(Value::String(s)) => Ok(s.clone()),
            Some(_) => Err(WalkerError::InvalidPackageJson {
                path: "package.json".into(),
                message: format!("Field '{}' is not a string", field),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
            None => Err(WalkerError::InvalidPackageJson {
                path: "package.json".into(),
                message: format!("Required field '{}' is missing", field),
                #[cfg(not(tarpaulin_include))]
                backtrace: std::backtrace::Backtrace::capture(),
            }),
        }
    }

    /// Extract an optional string field from a JSON object
    fn extract_optional_string(obj: &Map<String, Value>, field: &str) -> Option<String> {
        match obj.get(field) {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        }
    }

    /// Extract an optional boolean field from a JSON object
    fn extract_optional_bool(obj: &Map<String, Value>, field: &str) -> Option<bool> {
        match obj.get(field) {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Extract an optional array of strings from a JSON object
    fn extract_optional_string_array(obj: &Map<String, Value>, field: &str) -> Option<Vec<String>> {
        match obj.get(field) {
            Some(Value::Array(arr)) => {
                let strings: Vec<String> = arr
                    .iter()
                    .filter_map(|v| {
                        if let Value::String(s) = v {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                if strings.is_empty() {
                    None
                } else {
                    Some(strings)
                }
            }
            _ => None,
        }
    }

    /// Extract an optional JSON value from a JSON object
    fn extract_optional_value(obj: &Map<String, Value>, field: &str) -> Option<Value> {
        obj.get(field).cloned()
    }

    /// Extract author information from a JSON object
    fn extract_author(obj: &Map<String, Value>) -> Option<AuthorInfo> {
        match obj.get("author") {
            Some(Value::String(s)) => Some(AuthorInfo::String(s.clone())),
            Some(Value::Object(author_obj)) => {
                let name = match author_obj.get("name") {
                    Some(Value::String(s)) => s.clone(),
                    _ => return None, // Name is required for author object
                };

                let email = match author_obj.get("email") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };

                let url = match author_obj.get("url") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };

                Some(AuthorInfo::Object { name, email, url })
            }
            _ => None,
        }
    }

    /// Extract repository information from a JSON object
    fn extract_repository(obj: &Map<String, Value>) -> Option<RepositoryInfo> {
        match obj.get("repository") {
            Some(Value::String(s)) => Some(RepositoryInfo::String(s.clone())),
            Some(Value::Object(repo_obj)) => {
                let repo_type = match repo_obj.get("type") {
                    Some(Value::String(s)) => s.clone(),
                    _ => return None, // Type is required for repository object
                };

                let url = match repo_obj.get("url") {
                    Some(Value::String(s)) => s.clone(),
                    _ => return None, // URL is required for repository object
                };

                let directory = match repo_obj.get("directory") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };

                Some(RepositoryInfo::Object {
                    repo_type,
                    url,
                    directory,
                })
            }
            _ => None,
        }
    }

    /// Extract bugs information from a JSON object
    fn extract_bugs(obj: &Map<String, Value>) -> Option<BugsInfo> {
        match obj.get("bugs") {
            Some(Value::String(s)) => Some(BugsInfo::String(s.clone())),
            Some(Value::Object(bugs_obj)) => {
                let url = match bugs_obj.get("url") {
                    Some(Value::String(s)) => s.clone(),
                    _ => return None, // URL is required for bugs object
                };

                let email = match bugs_obj.get("email") {
                    Some(Value::String(s)) => Some(s.clone()),
                    _ => None,
                };

                Some(BugsInfo::Object { url, email })
            }
            _ => None,
        }
    }
}

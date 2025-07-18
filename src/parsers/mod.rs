//! Parsing functionality for package files
//!
//! This module provides parsers for various file formats used in the Walker tool,
//! including package.json and exports field parsing.

pub mod exports;
pub mod package_json;

pub use exports::ExportsParser;
pub use package_json::PackageJsonParser;
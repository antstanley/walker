//! Parsing functionality for package files
//!
//! This module provides parsers for various file formats used in the Walker tool,
//! including package.json and exports field parsing, as well as AST analysis.

pub mod ast_parser;
pub mod dependency_graph_builder;
pub mod exports;
pub mod module_detector;
pub mod package_json;

pub use ast_parser::ASTParser;
pub use dependency_graph_builder::{DependencyGraphBuilder, GraphBuilderConfig};
pub use exports::ExportsParser;
pub use module_detector::ModuleDetector;
pub use package_json::PackageJsonParser;
//! Data models and structures for Walker

pub mod analysis;
pub mod ast;
pub mod config;
pub mod dependency_graph;
pub mod file_metadata;
pub mod package;

pub use analysis::{AnalysisResults, AnalysisSummary, PackageAnalysis};
pub use ast::ASTAnalysisResults;
pub use config::Settings;
pub use dependency_graph::DependencyGraph;
pub use file_metadata::{FileMetadata, ModuleSystem};
pub use package::{DependencyInfo, ModuleSupport, PackageDetails};
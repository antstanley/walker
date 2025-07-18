//! Data models and structures for Walker

pub mod analysis;
pub mod config;
pub mod package;

pub use analysis::{AnalysisResults, AnalysisSummary, PackageAnalysis};
pub use config::Settings;
pub use package::{DependencyInfo, ModuleSupport, PackageDetails};
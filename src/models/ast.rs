//! AST analysis result structures

use super::dependency_graph::DependencyGraph;
use super::file_metadata::FileMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Complete results from AST analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTAnalysisResults {
    /// Package being analyzed
    pub package_path: PathBuf,

    /// All files analyzed
    pub files: Vec<FileMetadata>,

    /// Dependency graph
    pub dependency_graph: DependencyGraph,

    /// Statistics
    pub statistics: ASTStatistics,

    /// Complexity metrics
    pub complexity_metrics: ComplexityMetrics,

    /// Bundle impact analysis
    pub bundle_impact: BundleImpactAnalysis,

    /// Analysis errors (non-fatal)
    pub analysis_errors: Vec<AnalysisError>,

    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
}

/// Statistics from AST analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASTStatistics {
    pub total_files: usize,
    pub esm_files: usize,
    pub cjs_files: usize,
    pub mixed_files: usize,
    pub typescript_files: usize,
    pub referenced_files: usize,
    pub unreferenced_files: usize,
    pub total_exports: usize,
    pub total_imports: usize,
    pub entry_points_count: usize,
    pub circular_dependency_count: usize,
    pub unresolved_imports_count: usize,
    pub average_import_depth: f64,
    pub max_import_depth: usize,
    pub files_with_side_effects: usize,
}

/// Code complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity per file
    pub file_complexity: HashMap<PathBuf, FileComplexity>,

    /// Module cohesion (files that import each other)
    pub module_cohesion: HashMap<PathBuf, f64>,

    /// Files with highest fan-out (most dependencies)
    pub high_fan_out_files: Vec<(PathBuf, usize)>,

    /// Files with highest fan-in (most dependents)
    pub high_fan_in_files: Vec<(PathBuf, usize)>,
}

/// Complexity metrics for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComplexity {
    pub cyclomatic_complexity: usize,
    pub lines_of_code: usize,
    pub function_count: usize,
    pub class_count: usize,
    pub import_count: usize,
    pub export_count: usize,
}

/// Bundle impact analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleImpactAnalysis {
    /// Dependencies that contribute most to bundle size
    pub heaviest_dependencies: Vec<DependencyImpact>,

    /// Files marked as having side effects
    pub side_effect_files: Vec<PathBuf>,

    /// Tree-shakeable exports (ESM named exports)
    pub tree_shakeable_exports: HashMap<PathBuf, Vec<String>>,

    /// Non-tree-shakeable files (CJS or default exports only)
    pub non_tree_shakeable_files: Vec<PathBuf>,

    /// Estimated bundle contribution per file
    pub bundle_contribution: HashMap<PathBuf, BundleContribution>,
}

/// Impact of a dependency on bundle size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyImpact {
    pub package_name: String,
    pub total_size: u64,
    pub file_count: usize,
    pub import_chains: Vec<Vec<PathBuf>>,
}

/// Bundle contribution for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleContribution {
    pub direct_size: u64,
    pub transitive_size: u64,
    pub is_tree_shakeable: bool,
}

/// An error that occurred during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisError {
    pub file_path: PathBuf,
    pub error_type: AnalysisErrorType,
    pub message: String,
    pub import_chain: Vec<PathBuf>,
    pub suggested_fix: Option<String>,
}

/// Type of analysis error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisErrorType {
    ParseError,
    UnresolvedImport,
    CircularDependency,
    MissingExport,
    InvalidSyntax,
    UnsupportedFeature,
}

impl ASTAnalysisResults {
    /// Create new analysis results
    pub fn new(package_path: PathBuf) -> Self {
        Self {
            package_path,
            files: Vec::new(),
            dependency_graph: DependencyGraph::new(),
            statistics: ASTStatistics::default(),
            complexity_metrics: ComplexityMetrics::default(),
            bundle_impact: BundleImpactAnalysis::default(),
            analysis_errors: Vec::new(),
            analyzed_at: Utc::now(),
        }
    }

    /// Add a file to the results
    pub fn add_file(&mut self, metadata: FileMetadata) {
        // Update statistics
        self.update_statistics(&metadata);

        // Add to files list
        self.files.push(metadata);
    }

    /// Add an analysis error
    pub fn add_error(&mut self, error: AnalysisError) {
        self.analysis_errors.push(error);
    }

    /// Update statistics based on a file
    fn update_statistics(&mut self, metadata: &FileMetadata) {
        use super::file_metadata::ModuleSystem;

        self.statistics.total_files += 1;

        match metadata.module_system {
            ModuleSystem::ESM => self.statistics.esm_files += 1,
            ModuleSystem::CommonJS => self.statistics.cjs_files += 1,
            ModuleSystem::Mixed => self.statistics.mixed_files += 1,
            _ => {}
        }

        if metadata.file_type.is_typescript() {
            self.statistics.typescript_files += 1;
        }

        if metadata.is_referenced {
            self.statistics.referenced_files += 1;
        } else {
            self.statistics.unreferenced_files += 1;
        }

        self.statistics.total_exports += metadata.exports.len();
        self.statistics.total_imports += metadata.imports.len();

        if metadata.has_side_effects() {
            self.statistics.files_with_side_effects += 1;
        }
    }

    /// Finalize the analysis results
    pub fn finalize(&mut self) {
        // Calculate reachability in dependency graph
        self.dependency_graph.calculate_reachability();

        // Update statistics from dependency graph
        let graph_stats = self.dependency_graph.statistics();
        self.statistics.circular_dependency_count = graph_stats.circular_count;
        self.statistics.unresolved_imports_count = graph_stats.unresolved_count;
        self.statistics.average_import_depth = graph_stats.avg_depth;
        self.statistics.max_import_depth = graph_stats.max_depth;
        self.statistics.entry_points_count = self.dependency_graph.entry_points.len();

        // Calculate complexity metrics
        self.calculate_complexity_metrics();

        // Calculate bundle impact
        self.calculate_bundle_impact();
    }

    /// Calculate complexity metrics
    fn calculate_complexity_metrics(&mut self) {
        let mut high_fan_out = Vec::new();
        let mut high_fan_in = Vec::new();

        for (path, node) in &self.dependency_graph.nodes {
            // Calculate file complexity
            let complexity = FileComplexity {
                cyclomatic_complexity: 0, // Would need AST visitor to calculate
                lines_of_code: 0,         // Would need source text
                function_count: 0,         // Would need AST visitor
                class_count: 0,            // Would need AST visitor
                import_count: node.metadata.imports.len(),
                export_count: node.metadata.exports.len(),
            };

            self.complexity_metrics
                .file_complexity
                .insert(path.clone(), complexity);

            // Track fan-out and fan-in
            high_fan_out.push((path.clone(), node.dependencies.len()));
            high_fan_in.push((path.clone(), node.dependents.len()));
        }

        // Sort and take top 10
        high_fan_out.sort_by(|a, b| b.1.cmp(&a.1));
        high_fan_in.sort_by(|a, b| b.1.cmp(&a.1));

        self.complexity_metrics.high_fan_out_files = high_fan_out.into_iter().take(10).collect();
        self.complexity_metrics.high_fan_in_files = high_fan_in.into_iter().take(10).collect();
    }

    /// Calculate bundle impact
    fn calculate_bundle_impact(&mut self) {
        let mut dependency_sizes: HashMap<String, DependencyImpact> = HashMap::new();

        for file in &self.files {
            // Track side effects
            if file.has_side_effects() {
                self.bundle_impact.side_effect_files.push(file.absolute_path.clone());
            }

            // Track tree-shakeable exports
            if file.is_tree_shakeable() {
                let export_names: Vec<String> = file
                    .exports
                    .iter()
                    .filter(|e| !e.is_default)
                    .map(|e| e.name.clone())
                    .collect();

                if !export_names.is_empty() {
                    self.bundle_impact
                        .tree_shakeable_exports
                        .insert(file.absolute_path.clone(), export_names);
                }
            } else if !file.exports.is_empty() {
                self.bundle_impact
                    .non_tree_shakeable_files
                    .push(file.absolute_path.clone());
            }

            // Track bundle contribution
            let contribution = BundleContribution {
                direct_size: file.size_bytes,
                transitive_size: file.size_bytes, // Would need to calculate transitively
                is_tree_shakeable: file.is_tree_shakeable(),
            };

            self.bundle_impact
                .bundle_contribution
                .insert(file.absolute_path.clone(), contribution);

            // Track dependency impacts
            if let super::file_metadata::FileLocation::Dependency(pkg_name) = &file.file_location {
                dependency_sizes
                    .entry(pkg_name.clone())
                    .and_modify(|impact| {
                        impact.total_size += file.size_bytes;
                        impact.file_count += 1;
                    })
                    .or_insert(DependencyImpact {
                        package_name: pkg_name.clone(),
                        total_size: file.size_bytes,
                        file_count: 1,
                        import_chains: Vec::new(),
                    });
            }
        }

        // Sort dependencies by size
        let mut heaviest: Vec<DependencyImpact> = dependency_sizes.into_values().collect();
        heaviest.sort_by(|a, b| b.total_size.cmp(&a.total_size));
        self.bundle_impact.heaviest_dependencies = heaviest.into_iter().take(20).collect();
    }

    /// Generate a summary report
    pub fn summary(&self) -> String {
        format!(
            r#"AST Analysis Summary
====================
Package: {}
Analyzed at: {}

Files:
  Total: {}
  ESM: {}
  CommonJS: {}
  Mixed: {}
  TypeScript: {}

Reachability:
  Entry points: {}
  Referenced: {}
  Unreferenced: {} (potential dead code)

Dependencies:
  Circular: {}
  Unresolved imports: {}
  Average depth: {:.2}
  Max depth: {}

Bundle Impact:
  Files with side effects: {}
  Tree-shakeable files: {}
  Non-tree-shakeable: {}

Errors: {}
"#,
            self.package_path.display(),
            self.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC"),
            self.statistics.total_files,
            self.statistics.esm_files,
            self.statistics.cjs_files,
            self.statistics.mixed_files,
            self.statistics.typescript_files,
            self.statistics.entry_points_count,
            self.statistics.referenced_files,
            self.statistics.unreferenced_files,
            self.statistics.circular_dependency_count,
            self.statistics.unresolved_imports_count,
            self.statistics.average_import_depth,
            self.statistics.max_import_depth,
            self.statistics.files_with_side_effects,
            self.bundle_impact.tree_shakeable_exports.len(),
            self.bundle_impact.non_tree_shakeable_files.len(),
            self.analysis_errors.len()
        )
    }
}

impl Default for ASTStatistics {
    fn default() -> Self {
        Self {
            total_files: 0,
            esm_files: 0,
            cjs_files: 0,
            mixed_files: 0,
            typescript_files: 0,
            referenced_files: 0,
            unreferenced_files: 0,
            total_exports: 0,
            total_imports: 0,
            entry_points_count: 0,
            circular_dependency_count: 0,
            unresolved_imports_count: 0,
            average_import_depth: 0.0,
            max_import_depth: 0,
            files_with_side_effects: 0,
        }
    }
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self {
            file_complexity: HashMap::new(),
            module_cohesion: HashMap::new(),
            high_fan_out_files: Vec::new(),
            high_fan_in_files: Vec::new(),
        }
    }
}

impl Default for BundleImpactAnalysis {
    fn default() -> Self {
        Self {
            heaviest_dependencies: Vec::new(),
            side_effect_files: Vec::new(),
            tree_shakeable_exports: HashMap::new(),
            non_tree_shakeable_files: Vec::new(),
            bundle_contribution: HashMap::new(),
        }
    }
}
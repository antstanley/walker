//! Dependency graph data structures for AST analysis

use super::file_metadata::FileMetadata;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Complete dependency graph for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All files in the package
    pub nodes: HashMap<PathBuf, FileNode>,

    /// Edges represent imports/requires
    pub edges: Vec<DependencyEdge>,

    /// Entry points (from package.json main/exports)
    pub entry_points: Vec<PathBuf>,

    /// Files reachable from entry points
    pub reachable_files: HashSet<PathBuf>,

    /// Files NOT reachable (potential dead code)
    pub unreachable_files: HashSet<PathBuf>,

    /// Circular dependency chains
    pub circular_dependencies: HashSet<(PathBuf, PathBuf)>,

    /// Import depth from entry points
    pub import_depths: HashMap<PathBuf, usize>,

    /// Files with unresolved imports
    pub unresolved_imports: HashMap<PathBuf, Vec<String>>,
}

/// A node in the dependency graph representing a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// File metadata
    pub metadata: FileMetadata,
    /// Files that this file imports
    pub dependencies: Vec<PathBuf>,
    /// Files that import this file
    pub dependents: Vec<PathBuf>,
}

/// An edge in the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Source file (the importer)
    pub from: PathBuf,
    /// Target file (the imported)
    pub to: PathBuf,
    /// Type of import
    pub import_type: ImportType,
}

/// Type of import relationship
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportType {
    /// Static ES6 import
    StaticImport,
    /// Dynamic import()
    DynamicImport,
    /// CommonJS require()
    Require,
    /// TypeScript type-only import
    TypeImport,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            entry_points: Vec::new(),
            reachable_files: HashSet::new(),
            unreachable_files: HashSet::new(),
            circular_dependencies: HashSet::new(),
            import_depths: HashMap::new(),
            unresolved_imports: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, path: PathBuf, metadata: FileMetadata) {
        self.nodes.insert(
            path.clone(),
            FileNode {
                metadata,
                dependencies: Vec::new(),
                dependents: Vec::new(),
            },
        );
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, from: PathBuf, to: PathBuf, import_type: ImportType) {
        // Update edge list
        self.edges.push(DependencyEdge {
            from: from.clone(),
            to: to.clone(),
            import_type,
        });

        // Update node relationships
        if let Some(from_node) = self.nodes.get_mut(&from) {
            if !from_node.dependencies.contains(&to) {
                from_node.dependencies.push(to.clone());
            }
        }

        if let Some(to_node) = self.nodes.get_mut(&to) {
            if !to_node.dependents.contains(&from) {
                to_node.dependents.push(from);
            }
        }
    }

    /// Mark a file as an entry point
    pub fn add_entry_point(&mut self, path: PathBuf) {
        if !self.entry_points.contains(&path) {
            self.entry_points.push(path);
        }
    }

    /// Add a circular dependency
    pub fn add_circular_dependency(&mut self, from: PathBuf, to: PathBuf) {
        self.circular_dependencies.insert((from, to));
    }

    /// Add an unresolved import
    pub fn add_unresolved_import(&mut self, file: PathBuf, import: String) {
        self.unresolved_imports
            .entry(file)
            .or_insert_with(Vec::new)
            .push(import);
    }

    /// Calculate which files are reachable from entry points
    pub fn calculate_reachability(&mut self) {
        let mut visited = HashSet::new();
        let mut queue = self.entry_points.clone();
        let mut depths = HashMap::new();

        // Initialize entry points with depth 0
        for entry in &self.entry_points {
            depths.insert(entry.clone(), 0);
        }

        // BFS traversal from entry points
        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());
            self.reachable_files.insert(current.clone());

            let current_depth = *depths.get(&current).unwrap_or(&0);

            // Process dependencies of current file
            if let Some(node) = self.nodes.get(&current) {
                for dep in &node.dependencies {
                    if !visited.contains(dep) {
                        queue.push(dep.clone());

                        // Update depth
                        let new_depth = current_depth + 1;
                        depths.entry(dep.clone())
                            .and_modify(|d| *d = (*d).min(new_depth))
                            .or_insert(new_depth);
                    }
                }
            }
        }

        self.import_depths = depths;

        // Identify unreachable files
        for node_path in self.nodes.keys() {
            if !self.reachable_files.contains(node_path) {
                self.unreachable_files.insert(node_path.clone());
            }
        }
    }

    /// Get statistics about the dependency graph
    pub fn statistics(&self) -> GraphStatistics {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();
        let reachable_count = self.reachable_files.len();
        let unreachable_count = self.unreachable_files.len();
        let circular_count = self.circular_dependencies.len();
        let unresolved_count = self.unresolved_imports.values().map(|v| v.len()).sum();

        let max_depth = self.import_depths.values().copied().max().unwrap_or(0);
        let avg_depth = if !self.import_depths.is_empty() {
            self.import_depths.values().sum::<usize>() as f64 / self.import_depths.len() as f64
        } else {
            0.0
        };

        // Calculate fan-in and fan-out
        let mut max_fan_in = 0;
        let mut max_fan_out = 0;
        let mut max_fan_in_file = None;
        let mut max_fan_out_file = None;

        for (path, node) in &self.nodes {
            let fan_in = node.dependents.len();
            let fan_out = node.dependencies.len();

            if fan_in > max_fan_in {
                max_fan_in = fan_in;
                max_fan_in_file = Some(path.clone());
            }

            if fan_out > max_fan_out {
                max_fan_out = fan_out;
                max_fan_out_file = Some(path.clone());
            }
        }

        GraphStatistics {
            total_nodes,
            total_edges,
            reachable_count,
            unreachable_count,
            circular_count,
            unresolved_count,
            max_depth,
            avg_depth,
            max_fan_in,
            max_fan_in_file,
            max_fan_out,
            max_fan_out_file,
        }
    }

    /// Find all paths between two files
    pub fn find_paths(&self, from: &PathBuf, to: &PathBuf) -> Vec<Vec<PathBuf>> {
        let mut paths = Vec::new();
        let mut current_path = vec![from.clone()];
        let mut visited = HashSet::new();

        self.find_paths_recursive(from, to, &mut current_path, &mut visited, &mut paths);

        paths
    }

    fn find_paths_recursive(
        &self,
        current: &PathBuf,
        target: &PathBuf,
        current_path: &mut Vec<PathBuf>,
        visited: &mut HashSet<PathBuf>,
        paths: &mut Vec<Vec<PathBuf>>,
    ) {
        if current == target {
            paths.push(current_path.clone());
            return;
        }

        if visited.contains(current) {
            return;
        }

        visited.insert(current.clone());

        if let Some(node) = self.nodes.get(current) {
            for dep in &node.dependencies {
                current_path.push(dep.clone());
                self.find_paths_recursive(dep, target, current_path, visited, paths);
                current_path.pop();
            }
        }

        visited.remove(current);
    }

    /// Export to DOT format for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph dependencies {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Add nodes with styling
        for (path, node) in &self.nodes {
            let label = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            let color = if self.entry_points.contains(path) {
                "green"
            } else if self.unreachable_files.contains(path) {
                "red"
            } else {
                "black"
            };

            let style = if node.metadata.is_tree_shakeable() {
                "solid"
            } else {
                "dashed"
            };

            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\", color={}, style={}];\n",
                path.display(),
                label,
                color,
                style
            ));
        }

        dot.push_str("\n");

        // Add edges
        for edge in &self.edges {
            let style = match edge.import_type {
                ImportType::DynamicImport => "dotted",
                ImportType::TypeImport => "dashed",
                _ => "solid",
            };

            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [style={}];\n",
                edge.from.display(),
                edge.to.display(),
                style
            ));
        }

        // Add circular dependencies in red
        for (from, to) in &self.circular_dependencies {
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [color=red, style=bold];\n",
                from.display(),
                to.display()
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

/// Statistics about the dependency graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub reachable_count: usize,
    pub unreachable_count: usize,
    pub circular_count: usize,
    pub unresolved_count: usize,
    pub max_depth: usize,
    pub avg_depth: f64,
    pub max_fan_in: usize,
    pub max_fan_in_file: Option<PathBuf>,
    pub max_fan_out: usize,
    pub max_fan_out_file: Option<PathBuf>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}
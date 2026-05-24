//! Dependency graph and initialization ordering.
//!
//! Peripherals depend on one another — a chip-select GPIO must be configured
//! before the SPI device that uses it, an I2C device behind a mux must wait for
//! the mux, and so on. This module models those relationships as a directed
//! acyclic graph and computes a valid, layered initialization order via
//! [Kahn's algorithm](https://en.wikipedia.org/wiki/Topological_sorting).

pub mod builder;
pub mod cycle;
pub mod dot;
pub mod topo;

use std::collections::{HashMap, HashSet};

pub use builder::build;

/// A directed dependency graph over peripheral labels.
///
/// An edge from `A` to `B` means "`A` depends on `B`", i.e. `B` must be
/// initialized **before** `A`.
#[derive(Debug, Clone, Default)]
pub struct DepGraph {
    /// Adjacency list: node -> the set of nodes it depends on.
    edges: HashMap<String, HashSet<String>>,
    /// Every known node label.
    nodes: HashSet<String>,
}

impl DepGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a node. Idempotent.
    pub fn add_node(&mut self, label: &str) {
        self.nodes.insert(label.to_string());
        self.edges.entry(label.to_string()).or_default();
    }

    /// Record that `dependent` depends on `dependency` (dependency inits first).
    /// Both endpoints are registered as nodes if not already present.
    pub fn add_edge(&mut self, dependent: &str, dependency: &str) {
        self.add_node(dependent);
        self.add_node(dependency);
        self.edges
            .get_mut(dependent)
            .expect("node was just inserted")
            .insert(dependency.to_string());
    }

    /// The set of nodes `label` directly depends on.
    pub fn dependencies(&self, label: &str) -> impl Iterator<Item = &String> {
        self.edges.get(label).into_iter().flatten()
    }

    /// Iterator over all node labels.
    pub fn node_labels(&self) -> impl Iterator<Item = &String> {
        self.nodes.iter()
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Detect a dependency cycle. On success returns `Ok(())`; on failure
    /// returns the cycle path (e.g. `["a", "b", "a"]`).
    pub fn detect_cycle(&self) -> Result<(), Vec<String>> {
        cycle::detect(self)
    }

    /// Topologically sort the graph (dependencies first). Returns the cycle
    /// path as `Err` if the graph is not acyclic. Nodes available at the same
    /// time are emitted in alphabetical order for deterministic output.
    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<String>> {
        topo::sort(self)
    }

    /// Group nodes into initialization layers by dependency depth. Layer 0 has
    /// no dependencies; each subsequent layer depends only on earlier layers.
    pub fn layers(&self) -> Result<Vec<Vec<String>>, Vec<String>> {
        topo::layers(self)
    }

    /// The dependency depth of a node: 0 if it has no dependencies, otherwise
    /// one more than the deepest dependency. Returns 0 for unknown nodes or
    /// graphs containing a cycle.
    pub fn depth_of(&self, label: &str) -> usize {
        topo::depths(self)
            .ok()
            .and_then(|d| d.get(label).copied())
            .unwrap_or(0)
    }
}

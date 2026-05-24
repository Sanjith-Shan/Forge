//! Kahn's algorithm: topological sort and depth-based layering.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use super::DepGraph;

/// Topologically sort `g` (dependencies first). Returns the cycle path as
/// `Err` if `g` contains a cycle.
///
/// Ties (nodes whose dependencies are all already emitted) are broken
/// alphabetically, so the output is fully deterministic.
pub fn sort(g: &DepGraph) -> Result<Vec<String>, Vec<String>> {
    // In-degree here counts each node's *dependencies*.
    let mut indegree: HashMap<String, usize> = HashMap::new();
    for n in g.node_labels() {
        indegree.insert(n.clone(), g.dependencies(n).count());
    }

    // Reverse adjacency: for each dependency, who depends on it.
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for n in g.node_labels() {
        for dep in g.dependencies(n) {
            dependents.entry(dep.clone()).or_default().push(n.clone());
        }
    }

    // Min-heap (via `Reverse`) yields alphabetical order among ready nodes.
    let mut ready: BinaryHeap<Reverse<String>> = indegree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(n, _)| Reverse(n.clone()))
        .collect();

    let mut order = Vec::with_capacity(g.node_count());
    while let Some(Reverse(node)) = ready.pop() {
        if let Some(deps) = dependents.get(&node) {
            for m in deps {
                let deg = indegree.get_mut(m).expect("dependent is a known node");
                *deg -= 1;
                if *deg == 0 {
                    ready.push(Reverse(m.clone()));
                }
            }
        }
        order.push(node);
    }

    if order.len() == g.node_count() {
        Ok(order)
    } else {
        Err(super::cycle::detect(g).err().unwrap_or_default())
    }
}

/// Compute each node's dependency depth. Returns the cycle path as `Err` if the
/// graph is not acyclic.
pub fn depths(g: &DepGraph) -> Result<HashMap<String, usize>, Vec<String>> {
    let order = sort(g)?;
    let mut depth: HashMap<String, usize> = HashMap::new();
    for node in &order {
        // Deepest dependency + 1, or 0 if there are none.
        let d = g
            .dependencies(node)
            .map(|dep| depth.get(dep).copied().unwrap_or(0) + 1)
            .max()
            .unwrap_or(0);
        depth.insert(node.clone(), d);
    }
    Ok(depth)
}

/// Group nodes into layers by dependency depth, each layer sorted
/// alphabetically. Layer index equals dependency depth.
pub fn layers(g: &DepGraph) -> Result<Vec<Vec<String>>, Vec<String>> {
    let depth = depths(g)?;
    let max_depth = depth.values().copied().max();
    let mut layers: Vec<Vec<String>> = match max_depth {
        Some(m) => vec![Vec::new(); m + 1],
        None => Vec::new(),
    };
    for (node, d) in &depth {
        layers[*d].push(node.clone());
    }
    for layer in &mut layers {
        layer.sort();
    }
    Ok(layers)
}

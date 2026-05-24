//! Cycle detection with path reconstruction.

use std::collections::HashMap;

use super::DepGraph;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    /// Not yet visited.
    White,
    /// On the current DFS path.
    Gray,
    /// Fully explored.
    Black,
}

/// Detect a dependency cycle via depth-first search.
///
/// On success returns `Ok(())`. On failure returns the cycle as a path that
/// starts and ends at the same node, e.g. `["a", "b", "a"]`. Nodes are visited
/// in alphabetical order so the reported cycle is deterministic.
pub fn detect(g: &DepGraph) -> Result<(), Vec<String>> {
    let mut color: HashMap<String, Color> =
        g.node_labels().map(|n| (n.clone(), Color::White)).collect();
    let mut path: Vec<String> = Vec::new();

    let mut roots: Vec<String> = g.node_labels().cloned().collect();
    roots.sort();

    for start in &roots {
        if color[start] == Color::White {
            if let Some(found) = visit(g, start, &mut color, &mut path) {
                return Err(found);
            }
        }
    }
    Ok(())
}

/// DFS from `node`, returning a cycle path if a back-edge to a gray node is hit.
fn visit(
    g: &DepGraph,
    node: &str,
    color: &mut HashMap<String, Color>,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    color.insert(node.to_string(), Color::Gray);
    path.push(node.to_string());

    let mut deps: Vec<String> = g.dependencies(node).cloned().collect();
    deps.sort();

    for dep in deps {
        match color[&dep] {
            Color::White => {
                if let Some(found) = visit(g, &dep, color, path) {
                    return Some(found);
                }
            }
            Color::Gray => {
                // Back-edge: slice the current path from `dep` and close the loop.
                let start = path.iter().position(|n| n == &dep).expect("gray node on path");
                let mut found: Vec<String> = path[start..].to_vec();
                found.push(dep);
                return Some(found);
            }
            Color::Black => {}
        }
    }

    path.pop();
    color.insert(node.to_string(), Color::Black);
    None
}

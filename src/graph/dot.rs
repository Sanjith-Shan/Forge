//! Export a dependency graph as Graphviz DOT.

use std::collections::BTreeMap;

use super::DepGraph;
use crate::model::Board;

/// Render the dependency graph in Graphviz DOT format.
///
/// Edges point from a dependent to its dependency, so `rankdir=BT` draws
/// dependencies at the bottom and dependents above them — the order things
/// initialize, read upward.
///
/// ```text
/// forge board.toml --graph | dot -Tpng -o deps.png
/// ```
pub fn to_dot(board: &Board, g: &DepGraph) -> String {
    let details = node_details(board);

    let mut out = String::from("digraph board_init {\n");
    out.push_str("    rankdir=BT;\n");

    // Node declarations, alphabetical for stable output.
    let mut labels: Vec<&String> = g.node_labels().collect();
    labels.sort();
    for label in &labels {
        let detail = details
            .get(label.as_str())
            .map(String::as_str)
            .unwrap_or("unknown");
        out.push_str(&format!(
            "    {label} [shape=box, label=\"{label}\\n({detail})\"];\n"
        ));
    }

    // Edges, sorted by (dependent, dependency).
    let mut edges: Vec<(&String, &String)> = Vec::new();
    for label in &labels {
        for dep in g.dependencies(label) {
            edges.push((label, dep));
        }
    }
    edges.sort();
    for (from, to) in edges {
        out.push_str(&format!("    {from} -> {to};\n"));
    }

    out.push_str("}\n");
    out
}

/// Map each peripheral label to a short descriptor used in DOT node labels.
fn node_details(board: &Board) -> BTreeMap<String, String> {
    let mut details = BTreeMap::new();
    for gpio in &board.gpios {
        details.insert(gpio.label.clone(), format!("gpio pin {}", gpio.pin));
    }
    for bus in &board.i2c_buses {
        details.insert(bus.label.clone(), format!("i2c bus {}", bus.bus));
        for d in &bus.devices {
            details.insert(d.label.clone(), format!("i2c {:#04X}", d.address));
        }
    }
    for bus in &board.spi_buses {
        details.insert(bus.label.clone(), format!("spi bus {}", bus.bus));
        for d in &bus.devices {
            details.insert(d.label.clone(), format!("spi cs {}", d.cs_pin));
        }
    }
    for uart in &board.uarts {
        details.insert(uart.label.clone(), format!("uart {}", uart.index));
    }
    details
}

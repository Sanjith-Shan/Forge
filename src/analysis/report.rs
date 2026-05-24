//! The detailed `analysis.txt` report written when `--report` is passed.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::analysis::{bus_utilization, summary};
use crate::error::ForgeError;
use crate::graph::DepGraph;
use crate::model::Board;

/// Render the full analysis report as text.
pub fn render(
    board: &Board,
    graph: &DepGraph,
    layers: &[Vec<String>],
    warnings: &[String],
) -> String {
    let kinds = node_kinds(board);

    let mut s = format!("Forge Analysis Report — {}\n", board.name);
    s.push_str(&format!("Generated: {}\n\n", utc_now()));

    // Dependency graph.
    s.push_str("DEPENDENCY GRAPH\n");
    let mut labels: Vec<&String> = graph.node_labels().collect();
    labels.sort();
    for label in &labels {
        let kind = kinds.get(label.as_str()).map(String::as_str).unwrap_or("?");
        let mut deps: Vec<&String> = graph.dependencies(label).collect();
        deps.sort();
        if deps.is_empty() {
            s.push_str(&format!("  {label} ({kind}) — no dependencies\n"));
        } else {
            let joined = deps
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            s.push_str(&format!("  {label} ({kind}) — depends on: {joined}\n"));
        }
    }

    // Init order.
    s.push_str(&format!("\nINIT ORDER ({} layers)\n", layers.len()));
    for (i, layer) in layers.iter().enumerate() {
        s.push_str(&format!("  Layer {i}: {}\n", layer.join(", ")));
    }

    // Warnings.
    s.push_str("\nWARNINGS\n");
    if warnings.is_empty() {
        s.push_str("  [none]\n");
    } else {
        for w in warnings {
            s.push_str(&format!("  - {w}\n"));
        }
    }

    // Resource utilization.
    s.push_str("\nRESOURCE UTILIZATION\n");
    s.push_str(&format!(
        "  GPIO:  {}\n",
        count_noun(board.gpios.len(), "pin", "pins")
    ));
    s.push_str(&format!(
        "  I2C:   {}\n",
        count_noun(board.i2c_buses.len(), "bus", "buses")
    ));
    for bus in &board.i2c_buses {
        let util = match bus_utilization::utilization(bus) {
            Some(u) => format!("~{}%", (u * 100.0).round() as u32),
            None => "n/a (no poll rates)".to_string(),
        };
        s.push_str(&format!(
            "    {} @ {} kHz, {}, est. utilization {}\n",
            bus.label,
            bus.speed_khz,
            count_noun(bus.devices.len(), "device", "devices"),
            util,
        ));
    }
    s.push_str(&format!(
        "  SPI:   {}\n",
        count_noun(board.spi_buses.len(), "bus", "buses")
    ));
    s.push_str(&format!(
        "  UART:  {}\n",
        count_noun(board.uarts.len(), "port", "ports")
    ));

    // Echo the one-line summary for convenience.
    s.push_str("\nSUMMARY\n");
    for line in summary::render(board, layers.len()).lines() {
        s.push_str(&format!("  {line}\n"));
    }

    s
}

/// Write `analysis.txt` into `output_dir`.
pub async fn write(
    board: &Board,
    graph: &DepGraph,
    layers: &[Vec<String>],
    warnings: &[String],
    output_dir: &Path,
) -> Result<(), ForgeError> {
    let path = output_dir.join("analysis.txt");
    let contents = render(board, graph, layers, warnings);
    tokio::fs::write(&path, contents)
        .await
        .map_err(|source| ForgeError::Write { path, source })
}

/// `count` followed by the singular or plural form of a noun.
fn count_noun(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Map each peripheral label to a one-word kind, for the report's graph view.
fn node_kinds(board: &Board) -> BTreeMap<String, String> {
    let mut kinds = BTreeMap::new();
    for g in &board.gpios {
        kinds.insert(g.label.clone(), "gpio".to_string());
    }
    for b in &board.i2c_buses {
        kinds.insert(b.label.clone(), "i2c".to_string());
        for d in &b.devices {
            kinds.insert(d.label.clone(), "i2c".to_string());
        }
    }
    for b in &board.spi_buses {
        kinds.insert(b.label.clone(), "spi".to_string());
        for d in &b.devices {
            kinds.insert(d.label.clone(), "spi".to_string());
        }
    }
    for u in &board.uarts {
        kinds.insert(u.label.clone(), "uart".to_string());
    }
    kinds
}

/// Current UTC time formatted as an RFC 3339 timestamp, with no external crates.
fn utc_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (h, m, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert days-since-Unix-epoch to a `(year, month, day)` civil date.
///
/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

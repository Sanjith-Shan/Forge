//! Build a [`DepGraph`] from a validated [`Board`], inferring implicit
//! dependencies and validating explicit ones.

use std::collections::HashSet;

use super::DepGraph;
use crate::error::ValidationError;
use crate::model::Board;

/// Build the dependency graph for `board`.
///
/// Adds every peripheral as a node, then edges for:
/// - each I2C/SPI device → its parent bus,
/// - each SPI device `cs_pin` → a GPIO declared on that pin (if any),
/// - each device `power_pin` → the GPIO declared on that pin,
/// - each device `depends_on` → the named peripheral.
///
/// Returns every validation error found: unknown `depends_on` targets,
/// `power_pin`s that do not name a GPIO output, and dependency cycles.
pub fn build(board: &Board) -> Result<DepGraph, Vec<ValidationError>> {
    let mut g = DepGraph::new();
    let mut errors: Vec<ValidationError> = Vec::new();

    // 1. Register every peripheral as a node.
    for gpio in &board.gpios {
        g.add_node(&gpio.label);
    }
    for bus in &board.i2c_buses {
        g.add_node(&bus.label);
        for d in &bus.devices {
            g.add_node(&d.label);
        }
    }
    for bus in &board.spi_buses {
        g.add_node(&bus.label);
        for d in &bus.devices {
            g.add_node(&d.label);
        }
    }
    for uart in &board.uarts {
        g.add_node(&uart.label);
    }

    let known: HashSet<String> = g.node_labels().cloned().collect();

    // 2. I2C devices depend on their bus, plus power/explicit dependencies.
    for bus in &board.i2c_buses {
        for d in &bus.devices {
            g.add_edge(&d.label, &bus.label);
            add_power_edge(
                board,
                &d.label,
                d.power_pin,
                "i2c device",
                &mut g,
                &mut errors,
            );
            add_depends_edge(
                &known,
                &d.label,
                &d.depends_on,
                "i2c device",
                &mut g,
                &mut errors,
            );
        }
    }

    // 3. SPI devices depend on their bus, their chip-select GPIO (if declared),
    //    plus power/explicit dependencies.
    for bus in &board.spi_buses {
        for d in &bus.devices {
            g.add_edge(&d.label, &bus.label);
            if let Some(gpio) = board.gpio_by_pin(d.cs_pin) {
                g.add_edge(&d.label, &gpio.label);
            }
            add_power_edge(
                board,
                &d.label,
                d.power_pin,
                "spi device",
                &mut g,
                &mut errors,
            );
            add_depends_edge(
                &known,
                &d.label,
                &d.depends_on,
                "spi device",
                &mut g,
                &mut errors,
            );
        }
    }

    // 4. No cycles. Only meaningful once edges are well-formed.
    if errors.is_empty() {
        if let Err(path) = g.detect_cycle() {
            errors.push(ValidationError::new(format!(
                "Circular dependency detected in initialization order\n  {}\n\n  \
                 These peripherals form a cycle and cannot be initialized in any valid order.\n  \
                 Review the `depends_on` fields in your config.",
                path.join(" -> ")
            )));
        }
    }

    if errors.is_empty() {
        Ok(g)
    } else {
        Err(errors)
    }
}

/// Add an edge for a `power_pin`, validating it names a GPIO output.
fn add_power_edge(
    board: &Board,
    dependent: &str,
    power_pin: Option<u32>,
    kind: &str,
    g: &mut DepGraph,
    errors: &mut Vec<ValidationError>,
) {
    let Some(pin) = power_pin else { return };
    match board.gpio_by_pin(pin) {
        Some(gpio) if gpio.is_output() => g.add_edge(dependent, &gpio.label),
        Some(gpio) => errors.push(ValidationError::new(format!(
            "{kind} '{dependent}' has power_pin {pin}, but GPIO '{}' on that pin is not \
             configured as an output. A power pin must be a GPIO output.",
            gpio.label
        ))),
        None => errors.push(ValidationError::new(format!(
            "{kind} '{dependent}' has power_pin {pin}, but no GPIO output is declared on \
             that pin. Declare a `[[gpio]]` with mode \"output\" on pin {pin}."
        ))),
    }
}

/// Add an edge for an explicit `depends_on`, validating the target exists.
fn add_depends_edge(
    known: &HashSet<String>,
    dependent: &str,
    depends_on: &Option<String>,
    kind: &str,
    g: &mut DepGraph,
    errors: &mut Vec<ValidationError>,
) {
    let Some(target) = depends_on else { return };
    if known.contains(target) {
        g.add_edge(dependent, target);
    } else {
        errors.push(ValidationError::new(format!(
            "{kind} '{dependent}' depends_on '{target}', which is not a declared peripheral label."
        )));
    }
}

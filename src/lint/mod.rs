//! The design-review (lint) engine.
//!
//! Where `config::validate` and `graph::build` enforce *hard* correctness (a
//! config that fails them cannot be built), the lint engine produces *advisory*
//! diagnostics about a board that is already valid: bus saturation, unreachable
//! SPI clocks, interrupt overload, and so on. Each diagnostic carries a
//! severity, a stable machine code, a message, and an optional fix suggestion,
//! and can be rendered for humans or as JSON.

use serde::Serialize;

use crate::analysis::{bus_utilization, clock, interrupts};
use crate::model::Board;

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Must be fixed; the design is wrong.
    Error,
    /// Likely a problem; worth reviewing.
    Warning,
    /// Informational; no action required.
    Info,
}

impl Severity {
    /// Uppercase label for human output.
    fn label(self) -> &'static str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        }
    }
}

/// A single design-review finding.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    /// Severity of the finding.
    pub severity: Severity,
    /// Stable machine-readable code, e.g. `"i2c-bus-saturation"`.
    pub code: String,
    /// Human-readable description.
    pub message: String,
    /// Optional suggested fix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl Diagnostic {
    /// A warning-severity diagnostic.
    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// An info-severity diagnostic.
    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            code: code.to_string(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// An error-severity diagnostic.
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.into(),
            suggestion: None,
        }
    }

    /// Attach a suggested fix.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

/// Review an already-valid board, returning advisory diagnostics.
pub fn review(board: &Board) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for message in bus_utilization::check(&board.i2c_buses) {
        out.push(
            Diagnostic::warning("i2c-bus-saturation", message)
                .with_suggestion("Split devices across two buses or lower their poll rates."),
        );
    }
    for message in clock::check(board) {
        out.push(
            Diagnostic::warning("spi-clock-mismatch", message).with_suggestion(
                "Request a speed reachable by a power-of-two prescaler of the system clock.",
            ),
        );
    }
    for message in interrupts::check(board) {
        out.push(
            Diagnostic::warning("interrupt-density", message)
                .with_suggestion("Poll lower-priority signals instead of interrupting on them."),
        );
    }

    // Informational: buses we couldn't estimate, and idle buses.
    for bus in &board.i2c_buses {
        if !bus.devices.is_empty() && bus.devices.iter().all(|d| d.poll_rate_hz.is_none()) {
            out.push(Diagnostic::info(
                "i2c-no-poll-rate",
                format!(
                    "I2C bus '{}' has devices but no poll_rate_hz, so utilization was not estimated.",
                    bus.label
                ),
            ));
        }
    }

    out
}

/// True if any diagnostic is an error.
pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// Render diagnostics for a terminal.
pub fn render_human(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No issues found.\n".to_string();
    }
    let mut s = String::new();
    for d in diagnostics {
        s.push_str(&format!(
            "[{}] {}: {}\n",
            d.severity.label(),
            d.code,
            d.message
        ));
        if let Some(sug) = &d.suggestion {
            s.push_str(&format!("       ↳ {sug}\n"));
        }
    }
    s
}

/// Render diagnostics as a JSON array.
pub fn render_json(diagnostics: &[Diagnostic]) -> String {
    serde_json::to_string_pretty(diagnostics).unwrap_or_else(|_| "[]".to_string())
}

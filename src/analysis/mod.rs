//! Static analysis: non-fatal checks over the validated board.
//!
//! Analysis never blocks code generation. The checks return warnings (surfaced
//! to stderr by the CLI), and [`summary::print`] always emits a one-screen
//! resource overview.

pub mod bus_utilization;
pub mod clock;
pub mod interrupts;
pub mod report;
pub mod summary;

use crate::model::Board;

pub use report::write as write_report;
pub use summary::print as print_summary;

/// Run every analysis check, returning all warnings in a stable order.
pub fn analyze(board: &Board) -> Vec<String> {
    let mut warnings = Vec::new();
    warnings.extend(bus_utilization::check(&board.i2c_buses));
    warnings.extend(clock::check(board));
    warnings.extend(interrupts::check(board));
    warnings
}

//! Interrupt-density analysis.

use crate::model::Board;

/// Warn when more than this many GPIO interrupts are configured.
const MAX_INTERRUPTS: usize = 3;

/// Number of GPIO pins configured with an interrupt.
pub fn interrupt_count(board: &Board) -> usize {
    board.gpios.iter().filter(|g| g.interrupt.is_some()).count()
}

/// Produce a warning if interrupt density is high.
pub fn check(board: &Board) -> Vec<String> {
    let count = interrupt_count(board);
    if count > MAX_INTERRUPTS {
        vec![format!(
            "{count} GPIO interrupts configured. High interrupt density may cause \
             missed interrupts or excessive ISR overhead. Consider polling \
             for lower-priority signals."
        )]
    } else {
        Vec::new()
    }
}

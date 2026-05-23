//! The `main.c` skeleton.

use crate::model::Board;

/// Render `main.c`. The skeleton is the same for every board; it calls
/// `board_init()` once and then spins in an empty main loop.
///
/// `_board` is accepted for symmetry with the other generators (and to allow
/// future board-specific scaffolding) but is currently unused.
pub fn render(_board: &Board) -> String {
    "#include \"init.h\"\n\
     #include \"handlers.h\"\n\
     \n\
     int main(void) {\n    \
     board_init();\n\
     \n    \
     while (1) {\n        \
     /* TODO: Main loop */\n    \
     }\n\
     \n    \
     return 0;\n\
     }\n"
    .to_string()
}

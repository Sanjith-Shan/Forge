//! Snapshot tests: each valid fixture must produce exactly the expected C
//! output. Run with `UPDATE_SNAPSHOTS` in mind — these are intentionally strict.

use forge::codegen::{
    render_handlers_header, render_handlers_source, render_init_header, render_init_source,
    render_main,
};
use forge::config::{parse_str, validate};
use forge::model::Board;
use pretty_assertions::assert_eq;

/// Run the full parse + validate pipeline on a fixture, returning the board.
fn board_from(toml: &str) -> Board {
    let raw = parse_str(toml).expect("valid TOML");
    validate(raw).expect("valid config").board
}

#[test]
fn basic_snapshot_matches() {
    let board = board_from(include_str!("fixtures/valid_basic.toml"));
    assert_eq!(
        render_init_header(&board),
        include_str!("fixtures/expected/basic_init.h")
    );
    assert_eq!(
        render_init_source(&board),
        include_str!("fixtures/expected/basic_init.c")
    );
    assert_eq!(
        render_handlers_header(&board),
        include_str!("fixtures/expected/basic_handlers.h")
    );
    assert_eq!(
        render_handlers_source(&board),
        include_str!("fixtures/expected/basic_handlers.c")
    );
    assert_eq!(
        render_main(&board),
        include_str!("fixtures/expected/basic_main.c")
    );
}

#[test]
fn full_snapshot_matches() {
    let board = board_from(include_str!("fixtures/valid_full.toml"));
    assert_eq!(
        render_init_header(&board),
        include_str!("fixtures/expected/full_init.h")
    );
    assert_eq!(
        render_init_source(&board),
        include_str!("fixtures/expected/full_init.c")
    );
    assert_eq!(
        render_handlers_header(&board),
        include_str!("fixtures/expected/full_handlers.h")
    );
    assert_eq!(
        render_handlers_source(&board),
        include_str!("fixtures/expected/full_handlers.c")
    );
    assert_eq!(
        render_main(&board),
        include_str!("fixtures/expected/full_main.c")
    );
}

#[test]
fn generated_headers_have_include_guards() {
    let board = board_from(include_str!("fixtures/valid_full.toml"));
    let init_h = render_init_header(&board);
    assert!(init_h.contains("#ifndef FORGE_INIT_H"));
    assert!(init_h.contains("#endif /* FORGE_INIT_H */"));
}

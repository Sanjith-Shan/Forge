//! Snapshot tests: each valid fixture must produce exactly the expected C
//! output. These are intentionally strict.

use forge::codegen::{
    render_handlers_header, render_handlers_source, render_init_header, render_init_source,
    render_main,
};
use forge::config::{parse_str, validate};
use forge::graph;
use forge::model::Board;
use pretty_assertions::assert_eq;

/// Run the full pipeline on a fixture, returning the board and its init layers.
fn board_and_layers(toml: &str) -> (Board, Vec<Vec<String>>) {
    let raw = parse_str(toml).expect("valid TOML");
    let board = validate(raw).expect("valid config").board;
    let layers = graph::build(&board)
        .expect("valid dependency graph")
        .layers()
        .expect("acyclic graph");
    (board, layers)
}

#[test]
fn basic_snapshot_matches() {
    let (board, layers) = board_and_layers(include_str!("fixtures/valid_basic.toml"));
    assert_eq!(
        render_init_header(&board),
        include_str!("fixtures/expected/basic_init.h")
    );
    assert_eq!(
        render_init_source(&board, &layers),
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
    let (board, layers) = board_and_layers(include_str!("fixtures/valid_full.toml"));
    assert_eq!(
        render_init_header(&board),
        include_str!("fixtures/expected/full_init.h")
    );
    assert_eq!(
        render_init_source(&board, &layers),
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
fn dependencies_snapshot_matches() {
    let (board, layers) = board_and_layers(include_str!("fixtures/valid_dependencies.toml"));
    assert_eq!(
        render_init_source(&board, &layers),
        include_str!("fixtures/expected/deps_init.c")
    );
    assert_eq!(
        render_init_header(&board),
        include_str!("fixtures/expected/deps_init.h")
    );
}

#[test]
fn board_init_orders_dependencies_into_layers() {
    let (board, layers) = board_and_layers(include_str!("fixtures/valid_dependencies.toml"));
    let init_c = render_init_source(&board, &layers);

    // The mux's bus must be configured before the mux, which must come before
    // the sensors behind it. Check the emitted order reflects that.
    let pos = |needle: &str| init_c.find(needle).expect("call present");
    assert!(pos("i2c_sensor_bus_init();") < pos("i2c_i2c_mux_init();"));
    assert!(pos("i2c_i2c_mux_init();") < pos("i2c_imu_init();"));
    assert!(pos("i2c_i2c_mux_init();") < pos("i2c_barometer_init();"));
    // The power pin is driven HIGH before the device it powers.
    assert!(pos("GPIO_WRITE(12, HIGH);") < pos("i2c_imu_init();"));
    // The chip-select GPIO is configured before the SPI device that uses it.
    assert!(pos("gpio_init_cs_flash();") < pos("spi_flash_init();"));
}

#[test]
fn generated_headers_have_include_guards() {
    let (board, _) = board_and_layers(include_str!("fixtures/valid_full.toml"));
    let init_h = render_init_header(&board);
    assert!(init_h.contains("#ifndef FORGE_INIT_H"));
    assert!(init_h.contains("#endif /* FORGE_INIT_H */"));
}

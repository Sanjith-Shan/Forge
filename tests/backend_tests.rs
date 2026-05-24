//! Tests for the backend abstraction and the C / Zephyr backends.

use forge::backend::{self, Backend, InitPlan};
use forge::config::{parse_str, validate};
use forge::graph;
use forge::model::Board;

fn board_and_plan(toml: &str) -> (Board, InitPlan) {
    let board = validate(parse_str(toml).unwrap()).unwrap().board;
    let layers = graph::build(&board).unwrap().layers().unwrap();
    (board, InitPlan { layers })
}

#[test]
fn by_id_resolves_known_backends_only() {
    assert!(backend::by_id("c").is_some());
    assert!(backend::by_id("zephyr").is_some());
    assert!(backend::by_id("nonexistent").is_none());
}

#[test]
fn c_backend_emits_the_five_files() {
    let (board, plan) = board_and_plan(include_str!("fixtures/valid_dependencies.toml"));
    let files = backend::c::CBackend.render(&board, &plan);
    let names: Vec<String> = files
        .iter()
        .map(|f| f.path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        vec!["init.h", "init.c", "handlers.h", "handlers.c", "main.c"]
    );
}

#[test]
fn zephyr_backend_emits_overlay_and_prjconf() {
    let (board, plan) = board_and_plan(include_str!("fixtures/valid_dependencies.toml"));
    let files = backend::zephyr::ZephyrBackend.render(&board, &plan);

    let overlay = files
        .iter()
        .find(|f| f.path.to_string_lossy().ends_with(".overlay"))
        .expect("an overlay file");
    // Bus node, device address, chip-select, and clock all show up.
    assert!(overlay.contents.contains("&i2c0"));
    assert!(overlay.contents.contains("reg = <0x70>"), "mux address");
    assert!(overlay.contents.contains("clock-frequency = <400000>"));
    assert!(overlay.contents.contains("cs-gpios"));

    let prj = files
        .iter()
        .find(|f| f.path.to_string_lossy() == "prj.conf")
        .expect("a prj.conf");
    assert!(prj.contents.contains("CONFIG_I2C=y"));
    assert!(prj.contents.contains("CONFIG_SPI=y"));
}

#[test]
fn zephyr_prjconf_only_enables_used_subsystems() {
    // A GPIO-only board should not pull in I2C/SPI/UART.
    let toml = r#"
        [board]
        name = "blink"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "output"
        label = "led"
    "#;
    let (board, plan) = board_and_plan(toml);
    let files = backend::zephyr::ZephyrBackend.render(&board, &plan);
    let prj = &files
        .iter()
        .find(|f| f.path.to_string_lossy() == "prj.conf")
        .unwrap()
        .contents;
    assert!(prj.contains("CONFIG_GPIO=y"));
    assert!(!prj.contains("CONFIG_I2C"));
    assert!(!prj.contains("CONFIG_SPI"));
    assert!(!prj.contains("CONFIG_SERIAL"));
}

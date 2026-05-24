//! Tests for the lint / design-review engine's diagnostic layer.

use forge::config::{parse_str, validate};
use forge::lint::{self, Diagnostic, Severity};
use forge::model::Board;

fn board_from(toml: &str) -> Board {
    validate(parse_str(toml).unwrap()).unwrap().board
}

#[test]
fn review_flags_spi_clock_mismatch_as_warning() {
    let toml = r#"
        [board]
        name = "c"
        mcu = "x"
        clock_mhz = 16
        [[spi]]
        bus = 0
        mosi_pin = 23
        miso_pin = 19
        sck_pin = 18
        label = "bus0"
        [[spi.device]]
        cs_pin = 5
        label = "oled"
        mode = 0
        speed_mhz = 10
    "#;
    let diags = lint::review(&board_from(toml));
    let d = diags
        .iter()
        .find(|d| d.code == "spi-clock-mismatch")
        .expect("clock diagnostic");
    assert_eq!(d.severity, Severity::Warning);
    assert!(d.suggestion.is_some());
}

#[test]
fn review_is_quiet_on_a_clean_board() {
    let toml = r#"
        [board]
        name = "ok"
        mcu = "x"
        clock_mhz = 16
        [[gpio]]
        pin = 1
        mode = "output"
        label = "led"
    "#;
    // No warnings or errors; at most informational.
    let diags = lint::review(&board_from(toml));
    assert!(!lint::has_errors(&diags));
    assert!(diags.iter().all(|d| d.severity == Severity::Info));
}

#[test]
fn has_errors_detects_error_severity() {
    let diags = vec![
        Diagnostic::warning("w", "a warning"),
        Diagnostic::error("e", "an error"),
    ];
    assert!(lint::has_errors(&diags));
}

#[test]
fn json_output_is_a_parsable_array_with_codes() {
    let diags = vec![Diagnostic::warning("test-code", "hello").with_suggestion("do x")];
    let json = lint::render_json(&diags);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed[0]["code"], "test-code");
    assert_eq!(parsed[0]["severity"], "warning");
    assert_eq!(parsed[0]["suggestion"], "do x");
}

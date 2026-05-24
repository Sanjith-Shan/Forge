//! Tests for the AI frontend's verification gate.
//!
//! These never touch the network: they exercise `verify` directly and drive
//! `synthesize` through the deterministic `MockProvider`. The point is to prove
//! the gate catches bad model output — the model proposes, the core disposes.

use forge::frontend::ai::{synthesize, verify, AiRequest, MockProvider};

const VALID: &str = r#"
[board]
name = "blink"
mcu = "esp32c6"
clock_mhz = 160

[[gpio]]
pin = 13
mode = "output"
label = "led"
"#;

#[test]
fn gate_accepts_a_valid_candidate() {
    let outcome = verify(VALID.to_string());
    assert!(outcome.is_valid());
    assert!(outcome.board.is_some());
    assert!(outcome.errors.is_empty());
}

#[test]
fn gate_rejects_a_pin_conflict_from_the_model() {
    // The "model" put two peripherals on pin 21 — the gate must catch it.
    let bad = r#"
        [board]
        name = "b"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 21
        mode = "output"
        label = "led"
        [[i2c]]
        bus = 0
        sda_pin = 21
        scl_pin = 22
        speed_khz = 400
        label = "bus0"
    "#;
    let outcome = verify(bad.to_string());
    assert!(!outcome.is_valid());
    assert!(outcome.errors.iter().any(|e| e.message.contains("Pin 21")));
}

#[test]
fn gate_rejects_non_toml() {
    let outcome = verify("I'm a language model, not TOML!".to_string());
    assert!(!outcome.is_valid());
    assert!(outcome
        .errors
        .iter()
        .any(|e| e.message.contains("valid TOML")));
}

#[tokio::test]
async fn synthesize_strips_markdown_fences() {
    let fenced = format!("Here you go:\n```toml\n{VALID}\n```");
    let provider = MockProvider::new(fenced);
    let req = AiRequest {
        intent: "a blinky board".to_string(),
        datasheet: None,
    };
    let outcome = synthesize(&provider, &req).await.unwrap();
    assert!(outcome.is_valid(), "errors: {:?}", outcome.errors);
    assert!(!outcome.candidate_toml.contains("```"));
    assert!(outcome.candidate_toml.contains("[board]"));
}

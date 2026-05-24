//! Tests for credential / settings resolution.
//!
//! The `resolve_*` functions are pure (env is passed in), so these never touch
//! real environment variables or the real config file.

use forge::settings::{self, Settings};

fn settings_with_key(key: &str) -> Settings {
    Settings {
        openai_api_key: Some(key.to_string()),
        ..Default::default()
    }
}

#[test]
fn api_key_precedence_is_flag_then_env_then_config() {
    let cfg = settings_with_key("from-config");

    // Flag wins over everything.
    assert_eq!(
        settings::resolve_api_key(Some("from-flag"), Some("from-env"), &cfg).as_deref(),
        Some("from-flag")
    );
    // Env wins over config.
    assert_eq!(
        settings::resolve_api_key(None, Some("from-env"), &cfg).as_deref(),
        Some("from-env")
    );
    // Config is the last resort.
    assert_eq!(
        settings::resolve_api_key(None, None, &cfg).as_deref(),
        Some("from-config")
    );
    // Nothing anywhere → None.
    assert_eq!(
        settings::resolve_api_key(None, None, &Settings::default()),
        None
    );
}

#[test]
fn empty_values_are_ignored_in_resolution() {
    let cfg = settings_with_key("real-key");
    // An empty flag / env must not shadow the real config value.
    assert_eq!(
        settings::resolve_api_key(Some(""), Some(""), &cfg).as_deref(),
        Some("real-key")
    );
}

#[test]
fn model_falls_back_to_default() {
    let empty = Settings::default();
    assert_eq!(
        settings::resolve_model(None, None, &empty),
        settings::DEFAULT_MODEL
    );
    assert_eq!(
        settings::resolve_model(Some("gpt-4o"), None, &empty),
        "gpt-4o"
    );
    let cfg = Settings {
        openai_model: Some("o-cfg".to_string()),
        ..Default::default()
    };
    assert_eq!(settings::resolve_model(None, Some("o-env"), &cfg), "o-env");
    assert_eq!(settings::resolve_model(None, None, &cfg), "o-cfg");
}

#[test]
fn backend_and_output_fall_back_to_defaults() {
    let empty = Settings::default();
    assert_eq!(
        settings::resolve_backend(None, &empty),
        settings::DEFAULT_BACKEND
    );
    assert_eq!(
        settings::resolve_output(None, &empty),
        settings::DEFAULT_OUTPUT
    );
    assert_eq!(settings::resolve_backend(Some("zephyr"), &empty), "zephyr");
}

#[test]
fn redaction_hides_the_secret() {
    // Deliberately not an OpenAI-shaped token, so secret scanners stay quiet.
    let long = "EXAMPLEKEY_abcdef1234567890_TAIL";
    let shown = settings::redact(long);
    assert!(
        !shown.contains("abcdef1234567890"),
        "must not reveal the middle"
    );
    assert!(shown.starts_with("EXAMPL"));
    // Short secrets are fully masked.
    assert_eq!(settings::redact("short"), "********");
}

//! Tests that invalid configs produce the right errors and valid configs pass.

use forge::config::{parse_str, validate};

/// Parse + validate a TOML string, returning the collected error messages.
fn errors_for(toml: &str) -> Vec<String> {
    let raw = parse_str(toml).expect("fixture should be syntactically valid TOML");
    match validate(raw) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn pin_conflict_names_both_peripherals() {
    let errors = errors_for(include_str!("fixtures/invalid_pin_conflict.toml"));
    assert_eq!(errors.len(), 1, "expected exactly one error: {errors:?}");
    let msg = &errors[0];
    assert!(msg.contains("Pin 21"), "should name the pin: {msg}");
    assert!(msg.contains("status_led"), "should name the gpio: {msg}");
    assert!(msg.contains("sensor_bus"), "should name the i2c bus: {msg}");
    assert!(
        msg.contains("SDA"),
        "should name the conflicting role: {msg}"
    );
}

#[test]
fn i2c_address_out_of_range_is_rejected() {
    let errors = errors_for(include_str!("fixtures/invalid_i2c_addr.toml"));
    assert_eq!(errors.len(), 1, "expected exactly one error: {errors:?}");
    assert!(
        errors[0].contains("0x80"),
        "should name the address: {:?}",
        errors[0]
    );
    assert!(
        errors[0].to_lowercase().contains("out of range"),
        "should explain the range: {:?}",
        errors[0]
    );
}

#[test]
fn duplicate_label_is_rejected() {
    let errors = errors_for(include_str!("fixtures/invalid_duplicate_label.toml"));
    assert_eq!(errors.len(), 1, "expected exactly one error: {errors:?}");
    assert!(
        errors[0].contains("'led'"),
        "should name the label: {:?}",
        errors[0]
    );
}

#[test]
fn valid_configs_pass() {
    assert!(errors_for(include_str!("fixtures/valid_basic.toml")).is_empty());
    assert!(errors_for(include_str!("fixtures/valid_full.toml")).is_empty());
}

#[test]
fn reserved_i2c_address_warns_but_passes() {
    let toml = r#"
        [board]
        name = "w"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 100
        label = "b"
        [[i2c.device]]
        address = 0x03
        label = "d"
    "#;
    let raw = parse_str(toml).unwrap();
    let validated = validate(raw).expect("reserved address should still validate");
    assert_eq!(validated.warnings.len(), 1, "expected one warning");
    assert!(validated.warnings[0].contains("reserved"));
}

#[test]
fn missing_required_field_is_reported() {
    // I2C bus without sda_pin.
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        scl_pin = 2
        speed_khz = 100
        label = "b"
    "#;
    let errors = errors_for(toml);
    assert!(
        errors.iter().any(|e| e.contains("sda_pin")),
        "should report missing sda_pin: {errors:?}"
    );
}

#[test]
fn invalid_spi_mode_is_rejected() {
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[spi]]
        bus = 0
        mosi_pin = 1
        miso_pin = 2
        sck_pin = 3
        label = "disp"
        [[spi.device]]
        cs_pin = 4
        label = "oled"
        mode = 5
        speed_mhz = 1
    "#;
    let errors = errors_for(toml);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("mode") && e.contains("0, 1, 2, or 3")),
        "should reject mode 5: {errors:?}"
    );
}

#[test]
fn invalid_gpio_interrupt_is_rejected() {
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "input"
        label = "btn"
        interrupt = "sideways"
    "#;
    let errors = errors_for(toml);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("interrupt") && e.contains("sideways")),
        "should reject invalid interrupt edge: {errors:?}"
    );
}

#[test]
fn label_must_be_valid_c_identifier() {
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "output"
        label = "1bad"
    "#;
    let errors = errors_for(toml);
    assert!(
        errors.iter().any(|e| e.contains("valid C identifier")),
        "should reject label starting with a digit: {errors:?}"
    );
}

#[test]
fn all_errors_collected_not_just_the_first() {
    // Two independent problems: a bad SPI mode and a duplicate label.
    let toml = r#"
        [board]
        name = "m"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "output"
        label = "dup"
        [[gpio]]
        pin = 2
        mode = "nonsense"
        label = "dup"
    "#;
    let errors = errors_for(toml);
    assert!(
        errors.len() >= 2,
        "expected multiple errors, got: {errors:?}"
    );
    assert!(errors.iter().any(|e| e.contains("invalid mode")));
    assert!(errors.iter().any(|e| e.contains("'dup'")));
}

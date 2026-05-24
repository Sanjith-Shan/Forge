//! Tests for the static-analysis checks: I2C bus utilization, SPI clock
//! accuracy, and interrupt density.
//!
//! Note on utilization rates: the model charges `(1+1+2)*9 = 36` bit-times per
//! transaction against the bus clock, so crossing the 70% threshold on a
//! 400 kHz bus requires several kHz of aggregate polling. The fixtures below
//! use rates chosen to exercise the threshold under that formula.

use forge::analysis::{bus_utilization, clock, interrupts};
use forge::config::{parse_str, validate};
use forge::model::Board;

fn board_from(toml: &str) -> Board {
    let raw = parse_str(toml).expect("valid TOML");
    validate(raw).expect("valid config").board
}

#[test]
fn bus_utilization_fires_when_saturated() {
    // 3 devices polling fast on a 400 kHz bus: 3 * 3100 * 36 / 400000 ≈ 0.84.
    let toml = r#"
        [board]
        name = "u"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 400
        label = "sensor_bus"
        [[i2c.device]]
        address = 0x10
        label = "a"
        poll_rate_hz = 3100
        [[i2c.device]]
        address = 0x11
        label = "b"
        poll_rate_hz = 3100
        [[i2c.device]]
        address = 0x12
        label = "c"
        poll_rate_hz = 3100
    "#;
    let warnings = bus_utilization::check(&board_from(toml).i2c_buses);
    assert_eq!(warnings.len(), 1, "expected one warning: {warnings:?}");
    assert!(warnings[0].contains("sensor_bus"));
    assert!(warnings[0].contains("utilization"));
}

#[test]
fn bus_utilization_ok_when_idle() {
    let toml = r#"
        [board]
        name = "u"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 400
        label = "sensor_bus"
        [[i2c.device]]
        address = 0x10
        label = "a"
        poll_rate_hz = 10
    "#;
    assert!(bus_utilization::check(&board_from(toml).i2c_buses).is_empty());
}

#[test]
fn bus_utilization_skips_devices_without_poll_rate() {
    let toml = r#"
        [board]
        name = "u"
        mcu = "x"
        clock_mhz = 8
        [[i2c]]
        bus = 0
        sda_pin = 1
        scl_pin = 2
        speed_khz = 400
        label = "sensor_bus"
        [[i2c.device]]
        address = 0x10
        label = "a"
        [[i2c.device]]
        address = 0x11
        label = "b"
        [[i2c.device]]
        address = 0x12
        label = "c"
    "#;
    assert!(
        bus_utilization::check(&board_from(toml).i2c_buses).is_empty(),
        "no poll rates means nothing to estimate"
    );
}

#[test]
fn spi_clock_mismatch_warns() {
    // 10 MHz requested on a 16 MHz clock => prescaler 2 => 8 MHz actual (20% off).
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
    let warnings = clock::check(&board_from(toml));
    assert_eq!(warnings.len(), 1, "expected one warning: {warnings:?}");
    assert!(warnings[0].contains("oled"));
    assert!(warnings[0].contains("8 MHz"), "msg: {}", warnings[0]);
}

#[test]
fn spi_clock_exact_match_is_quiet() {
    // 8 MHz on a 16 MHz clock => prescaler 2 => exactly 8 MHz.
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
        speed_mhz = 8
    "#;
    assert!(clock::check(&board_from(toml)).is_empty());
}

#[test]
fn interrupt_density_fires_above_three() {
    let toml = r#"
        [board]
        name = "i"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "input"
        label = "a"
        interrupt = "rising"
        [[gpio]]
        pin = 2
        mode = "input"
        label = "b"
        interrupt = "rising"
        [[gpio]]
        pin = 3
        mode = "input"
        label = "c"
        interrupt = "rising"
        [[gpio]]
        pin = 4
        mode = "input"
        label = "d"
        interrupt = "rising"
    "#;
    let warnings = interrupts::check(&board_from(toml));
    assert_eq!(warnings.len(), 1, "expected one warning: {warnings:?}");
    assert!(warnings[0].contains("4 GPIO interrupts"));
}

#[test]
fn interrupt_density_ok_at_two() {
    let toml = r#"
        [board]
        name = "i"
        mcu = "x"
        clock_mhz = 8
        [[gpio]]
        pin = 1
        mode = "input"
        label = "a"
        interrupt = "rising"
        [[gpio]]
        pin = 2
        mode = "input"
        label = "b"
        interrupt = "falling"
    "#;
    assert!(interrupts::check(&board_from(toml)).is_empty());
}

//! Prompt construction for the AI frontend.
//!
//! The system prompt teaches the model Forge's TOML schema and the rules it
//! must follow; the user prompt carries the engineer's intent and any datasheet
//! text. The model is asked to emit a config only — never prose — because its
//! output is fed straight into the deterministic verification gate.

use super::AiRequest;

/// The system prompt: schema, rules, and output contract.
pub fn system_prompt() -> String {
    r#"You are Forge's hardware-config synthesizer. Convert the user's request into a
`board.toml` describing an embedded board. Output ONLY the TOML — no prose, no
explanation, no markdown fences.

Schema:

[board]
name = "snake_case_name"      # required
mcu  = "part_number"          # required, e.g. "esp32c6", "stm32f4"
clock_mhz = 160               # required, integer

[[gpio]]                      # zero or more
pin = 13                      # required, integer
mode = "output"               # required: input | output | input_pullup | input_pulldown
label = "status_led"          # required, valid C identifier, unique across the whole file
interrupt = "falling"         # optional: rising | falling | both | none

[[i2c]]                       # zero or more buses
bus = 0
sda_pin = 21
scl_pin = 22
speed_khz = 400               # 100 or 400
label = "sensor_bus"

[[i2c.device]]                # devices on the most recent [[i2c]]
address = 0x68                # 7-bit, 0x08..0x77 (avoid reserved 0x00-0x07, 0x78-0x7F)
label = "imu"
depends_on = "other_label"    # optional: must initialize first
power_pin = 12                # optional: a GPIO output driven HIGH before this device
poll_rate_hz = 100            # optional: how often the device is polled

[[spi]]                       # zero or more buses
bus = 0
mosi_pin = 23
miso_pin = 19
sck_pin = 18
label = "display_bus"

[[spi.device]]
cs_pin = 5
label = "oled"
mode = 0                      # 0..3
speed_mhz = 10
depends_on = "..."            # optional
power_pin = 12                # optional

[[uart]]                      # zero or more
index = 1
tx_pin = 17
rx_pin = 16
baud = 115200
label = "debug_serial"

Rules:
- Every pin number may be used by exactly one peripheral.
- Every label must be unique and a valid C identifier.
- Two I2C devices on the same bus must have different addresses.
- Use `depends_on` when one device must be initialized after another (e.g. a
  sensor behind an I2C mux depends_on the mux).
- Use `power_pin` (referencing a declared GPIO output) when a device must be
  powered on before init.
- Prefer realistic pins and addresses for the named MCU when you know them.
"#
    .to_string()
}

/// The user prompt: the request plus any datasheet excerpt.
pub fn user_prompt(request: &AiRequest) -> String {
    let mut s = format!("Request: {}\n", request.intent);
    if let Some(datasheet) = &request.datasheet {
        s.push_str("\nRelevant datasheet excerpt:\n");
        s.push_str(datasheet);
        s.push('\n');
    }
    s.push_str("\nOutput the board.toml now.");
    s
}

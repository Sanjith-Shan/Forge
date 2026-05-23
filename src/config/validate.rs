//! Semantic validation: lower a [`RawConfig`] into a [`Board`], collecting every
//! problem along the way.
//!
//! Validation never stops at the first error. Hard errors are accumulated into a
//! `Vec<ValidationError>`; softer issues (e.g. reserved I2C addresses) are
//! collected as warnings and returned alongside a successfully-built board.

use std::collections::HashMap;

use crate::config::parse::{
    RawBoard, RawConfig, RawGpio, RawI2c, RawI2cDevice, RawSpi, RawSpiDevice, RawUart,
};
use crate::error::ValidationError;
use crate::model::{
    Board, Gpio, GpioMode, I2cBus, I2cDevice, InterruptEdge, SpiBus, SpiDevice, Uart,
};

/// The successful result of validation: a board plus any non-fatal warnings.
#[derive(Debug, Clone)]
pub struct Validated {
    /// The validated board model.
    pub board: Board,
    /// Non-fatal warnings to surface to the user.
    pub warnings: Vec<String>,
}

/// Validate a parsed config, producing a [`Board`] or a list of every error.
///
/// On success the returned [`Validated`] may still carry warnings. On failure
/// the `Err` holds *all* discovered errors, ordered roughly by where they
/// appear in the file.
pub fn validate(raw: RawConfig) -> Result<Validated, Vec<ValidationError>> {
    let mut cx = Collector::default();

    let (name, mcu, clock_mhz) = validate_board(&raw.board, &mut cx);

    let gpios = validate_gpios(&raw.gpio, &mut cx);
    let i2c_buses = validate_i2c(&raw.i2c, &mut cx);
    let spi_buses = validate_spi(&raw.spi, &mut cx);
    let uarts = validate_uart(&raw.uart, &mut cx);

    if !cx.errors.is_empty() {
        return Err(cx.errors);
    }

    let board = Board {
        name,
        mcu,
        clock_mhz,
        gpios,
        i2c_buses,
        spi_buses,
        uarts,
    };

    if board.is_empty() {
        cx.warnings
            .push("board defines no peripherals; generated files will be empty stubs".to_string());
    }

    Ok(Validated {
        board,
        warnings: cx.warnings,
    })
}

/// Accumulates errors, warnings, and the pin/label registries used for
/// cross-cutting uniqueness checks.
#[derive(Default)]
struct Collector {
    errors: Vec<ValidationError>,
    warnings: Vec<String>,
    /// Maps a claimed pin number to a description of its first claimant.
    pins: HashMap<u32, String>,
    /// Maps a claimed label to a description of its first claimant.
    labels: HashMap<String, String>,
}

impl Collector {
    fn error(&mut self, msg: impl Into<String>) {
        self.errors.push(ValidationError::new(msg));
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    /// Claim a pin for `owner`. If the pin is already claimed, record a
    /// conflict error naming both claimants.
    fn claim_pin(&mut self, pin: u32, owner: &str) {
        if let Some(existing) = self.pins.get(&pin) {
            self.errors.push(ValidationError::new(format!(
                "Pin {pin} is claimed by both {existing} and {owner}. \
                 Each pin can only be assigned to one peripheral."
            )));
        } else {
            self.pins.insert(pin, owner.to_string());
        }
    }

    /// Claim a label for `owner`. If the label is already claimed, record a
    /// duplicate-label error naming both claimants.
    fn claim_label(&mut self, label: &str, owner: &str) {
        if let Some(existing) = self.labels.get(label) {
            self.errors.push(ValidationError::new(format!(
                "Label '{label}' is used by both {existing} and {owner}. \
                 Labels must be unique because they become C identifiers."
            )));
        } else {
            self.labels.insert(label.to_string(), owner.to_string());
        }
    }
}

/// A label must be a valid C identifier: starts with a letter or underscore,
/// followed by letters, digits, or underscores.
fn is_c_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate a label field: present, and a legal C identifier. Returns the label
/// if usable, registering it for duplicate detection.
fn check_label(
    label: &Option<String>,
    peripheral: &str,
    owner_desc: &str,
    cx: &mut Collector,
) -> Option<String> {
    match label {
        None => {
            cx.error(format!(
                "{peripheral} {owner_desc} is missing required field 'label'"
            ));
            None
        }
        Some(l) if l.is_empty() => {
            cx.error(format!("{peripheral} {owner_desc} has an empty label"));
            None
        }
        Some(l) if !is_c_identifier(l) => {
            cx.error(format!(
                "{peripheral} label '{l}' is not a valid C identifier \
                 (use letters, digits, and underscores; must not start with a digit)"
            ));
            None
        }
        Some(l) => Some(l.clone()),
    }
}

/// Require a numeric field, recording an error if it is absent.
fn require<T: Copy>(
    value: Option<T>,
    field: &str,
    peripheral: &str,
    owner_desc: &str,
    cx: &mut Collector,
) -> Option<T> {
    if value.is_none() {
        cx.error(format!(
            "{peripheral} {owner_desc} is missing required field '{field}'"
        ));
    }
    value
}

fn validate_board(raw: &RawBoard, cx: &mut Collector) -> (String, String, u32) {
    let name = raw.name.clone().unwrap_or_else(|| {
        cx.error("[board] is missing required field 'name'");
        String::new()
    });
    let mcu = raw.mcu.clone().unwrap_or_else(|| {
        cx.error("[board] is missing required field 'mcu'");
        String::new()
    });
    let clock_mhz = raw.clock_mhz.unwrap_or_else(|| {
        cx.error("[board] is missing required field 'clock_mhz'");
        0
    });
    if clock_mhz == 0 && raw.clock_mhz.is_some() {
        cx.warn("[board].clock_mhz is 0; baud-rate timing in generated code may be wrong");
    }
    (name, mcu, clock_mhz)
}

fn validate_gpios(raw: &[RawGpio], cx: &mut Collector) -> Vec<Gpio> {
    let mut out = Vec::new();
    for (idx, g) in raw.iter().enumerate() {
        let owner_desc = match (&g.label, g.pin) {
            (Some(l), _) => format!("'{l}'"),
            (None, Some(p)) => format!("#{} (pin {p})", idx + 1),
            (None, None) => format!("#{}", idx + 1),
        };
        let label = check_label(&g.label, "gpio", &owner_desc, cx);
        let pin = require(g.pin, "pin", "gpio", &owner_desc, cx);

        let mode = match &g.mode {
            None => {
                cx.error(format!(
                    "gpio {owner_desc} is missing required field 'mode'"
                ));
                None
            }
            Some(m) => match GpioMode::from_toml(m) {
                Some(mode) => Some(mode),
                None => {
                    cx.error(format!(
                        "gpio {owner_desc} has invalid mode '{m}' \
                         (expected: input, output, input_pullup, input_pulldown)"
                    ));
                    None
                }
            },
        };

        // Interrupt: optional. "none" means no interrupt; anything else must be
        // a valid edge.
        let interrupt = match &g.interrupt {
            None => None,
            Some(s) if s == "none" => None,
            Some(s) => match InterruptEdge::from_toml(s) {
                Some(edge) => Some(edge),
                None => {
                    cx.error(format!(
                        "gpio {owner_desc} has invalid interrupt '{s}' \
                         (expected: rising, falling, both, none)"
                    ));
                    None
                }
            },
        };

        if let Some(l) = &label {
            cx.claim_label(
                l,
                &format!("gpio (pin {})", pin.map(|p| p as i64).unwrap_or(-1)),
            );
        }
        if let Some(p) = pin {
            let who = label
                .as_deref()
                .map(|l| format!("gpio '{l}'"))
                .unwrap_or_else(|| format!("gpio #{}", idx + 1));
            cx.claim_pin(p, &who);
        }

        if let (Some(pin), Some(mode), Some(label)) = (pin, mode, label) {
            out.push(Gpio {
                pin,
                mode,
                label,
                interrupt,
            });
        }
    }
    out
}

fn validate_i2c(raw: &[RawI2c], cx: &mut Collector) -> Vec<I2cBus> {
    let mut out = Vec::new();
    for (idx, b) in raw.iter().enumerate() {
        let owner_desc = match (&b.label, b.bus) {
            (Some(l), _) => format!("'{l}'"),
            (None, Some(n)) => format!("#{} (bus {n})", idx + 1),
            (None, None) => format!("#{}", idx + 1),
        };
        let label = check_label(&b.label, "i2c bus", &owner_desc, cx);
        let bus = require(b.bus, "bus", "i2c bus", &owner_desc, cx);
        let sda_pin = require(b.sda_pin, "sda_pin", "i2c bus", &owner_desc, cx);
        let scl_pin = require(b.scl_pin, "scl_pin", "i2c bus", &owner_desc, cx);
        let speed_khz = require(b.speed_khz, "speed_khz", "i2c bus", &owner_desc, cx);

        let bus_name = label
            .as_deref()
            .map(|l| format!("i2c bus '{l}'"))
            .unwrap_or_else(|| format!("i2c bus #{}", idx + 1));

        if let Some(l) = &label {
            cx.claim_label(l, &bus_name);
        }
        if let Some(p) = sda_pin {
            cx.claim_pin(p, &format!("{bus_name} (SDA)"));
        }
        if let Some(p) = scl_pin {
            cx.claim_pin(p, &format!("{bus_name} (SCL)"));
        }

        let devices = validate_i2c_devices(&b.devices, &bus_name, cx);

        if let (Some(bus), Some(sda_pin), Some(scl_pin), Some(speed_khz), Some(label)) =
            (bus, sda_pin, scl_pin, speed_khz, label)
        {
            out.push(I2cBus {
                bus,
                sda_pin,
                scl_pin,
                speed_khz,
                label,
                devices,
            });
        }
    }
    out
}

fn validate_i2c_devices(
    raw: &[RawI2cDevice],
    bus_name: &str,
    cx: &mut Collector,
) -> Vec<I2cDevice> {
    let mut out = Vec::new();
    for (idx, d) in raw.iter().enumerate() {
        let owner_desc = match &d.label {
            Some(l) => format!("'{l}'"),
            None => format!("#{} on {bus_name}", idx + 1),
        };
        let label = check_label(&d.label, "i2c device", &owner_desc, cx);
        let address = require(d.address, "address", "i2c device", &owner_desc, cx);

        if let Some(l) = &label {
            let who = match address {
                Some(a) => format!("i2c device ({:#04X} on {bus_name})", a),
                None => format!("i2c device (on {bus_name})"),
            };
            cx.claim_label(l, &who);
        }

        let address = address.and_then(|a| validate_i2c_address(a, &owner_desc, cx));

        if let (Some(address), Some(label)) = (address, label) {
            out.push(I2cDevice { address, label });
        }
    }
    out
}

/// Check a 7-bit I2C address: in range, and not in a reserved block.
fn validate_i2c_address(addr: u32, owner_desc: &str, cx: &mut Collector) -> Option<u8> {
    if addr > 0x7F {
        cx.error(format!(
            "i2c device {owner_desc} has address {addr:#04X} out of range \
             (must be a 7-bit address, 0x00-0x7F)"
        ));
        return None;
    }
    // Reserved ranges per the I2C spec: 0x00-0x07 and 0x78-0x7F.
    if (0x00..=0x07).contains(&addr) || (0x78..=0x7F).contains(&addr) {
        cx.warn(format!(
            "i2c device {owner_desc} uses reserved address {addr:#04X} \
             (0x00-0x07 and 0x78-0x7F are reserved by the I2C spec)"
        ));
    }
    Some(addr as u8)
}

fn validate_spi(raw: &[RawSpi], cx: &mut Collector) -> Vec<SpiBus> {
    let mut out = Vec::new();
    for (idx, b) in raw.iter().enumerate() {
        let owner_desc = match (&b.label, b.bus) {
            (Some(l), _) => format!("'{l}'"),
            (None, Some(n)) => format!("#{} (bus {n})", idx + 1),
            (None, None) => format!("#{}", idx + 1),
        };
        let label = check_label(&b.label, "spi bus", &owner_desc, cx);
        let bus = require(b.bus, "bus", "spi bus", &owner_desc, cx);
        let mosi_pin = require(b.mosi_pin, "mosi_pin", "spi bus", &owner_desc, cx);
        let miso_pin = require(b.miso_pin, "miso_pin", "spi bus", &owner_desc, cx);
        let sck_pin = require(b.sck_pin, "sck_pin", "spi bus", &owner_desc, cx);

        let bus_name = label
            .as_deref()
            .map(|l| format!("spi bus '{l}'"))
            .unwrap_or_else(|| format!("spi bus #{}", idx + 1));

        if let Some(l) = &label {
            cx.claim_label(l, &bus_name);
        }
        if let Some(p) = mosi_pin {
            cx.claim_pin(p, &format!("{bus_name} (MOSI)"));
        }
        if let Some(p) = miso_pin {
            cx.claim_pin(p, &format!("{bus_name} (MISO)"));
        }
        if let Some(p) = sck_pin {
            cx.claim_pin(p, &format!("{bus_name} (SCK)"));
        }

        let devices = validate_spi_devices(&b.devices, &bus_name, cx);

        if let (Some(bus), Some(mosi_pin), Some(miso_pin), Some(sck_pin), Some(label)) =
            (bus, mosi_pin, miso_pin, sck_pin, label)
        {
            out.push(SpiBus {
                bus,
                mosi_pin,
                miso_pin,
                sck_pin,
                label,
                devices,
            });
        }
    }
    out
}

fn validate_spi_devices(
    raw: &[RawSpiDevice],
    bus_name: &str,
    cx: &mut Collector,
) -> Vec<SpiDevice> {
    let mut out = Vec::new();
    for (idx, d) in raw.iter().enumerate() {
        let owner_desc = match &d.label {
            Some(l) => format!("'{l}'"),
            None => format!("#{} on {bus_name}", idx + 1),
        };
        let label = check_label(&d.label, "spi device", &owner_desc, cx);
        let cs_pin = require(d.cs_pin, "cs_pin", "spi device", &owner_desc, cx);
        let speed_mhz = require(d.speed_mhz, "speed_mhz", "spi device", &owner_desc, cx);

        let dev_name = label
            .as_deref()
            .map(|l| format!("spi device '{l}'"))
            .unwrap_or_else(|| format!("spi device #{} on {bus_name}", idx + 1));

        if let Some(l) = &label {
            cx.claim_label(l, &dev_name);
        }
        if let Some(p) = cs_pin {
            cx.claim_pin(p, &format!("{dev_name} (CS)"));
        }

        let mode = match d.mode {
            None => {
                cx.error(format!(
                    "spi device {owner_desc} is missing required field 'mode'"
                ));
                None
            }
            Some(m) if m <= 3 => Some(m as u8),
            Some(m) => {
                cx.error(format!(
                    "spi device {owner_desc} has invalid mode {m} (must be 0, 1, 2, or 3)"
                ));
                None
            }
        };

        if let (Some(cs_pin), Some(mode), Some(speed_mhz), Some(label)) =
            (cs_pin, mode, speed_mhz, label)
        {
            out.push(SpiDevice {
                cs_pin,
                label,
                mode,
                speed_mhz,
            });
        }
    }
    out
}

fn validate_uart(raw: &[RawUart], cx: &mut Collector) -> Vec<Uart> {
    let mut out = Vec::new();
    for (idx, u) in raw.iter().enumerate() {
        let owner_desc = match (&u.label, u.index) {
            (Some(l), _) => format!("'{l}'"),
            (None, Some(n)) => format!("#{} (index {n})", idx + 1),
            (None, None) => format!("#{}", idx + 1),
        };
        let label = check_label(&u.label, "uart", &owner_desc, cx);
        let index = require(u.index, "index", "uart", &owner_desc, cx);
        let tx_pin = require(u.tx_pin, "tx_pin", "uart", &owner_desc, cx);
        let rx_pin = require(u.rx_pin, "rx_pin", "uart", &owner_desc, cx);
        let baud = require(u.baud, "baud", "uart", &owner_desc, cx);

        let uart_name = label
            .as_deref()
            .map(|l| format!("uart '{l}'"))
            .unwrap_or_else(|| format!("uart #{}", idx + 1));

        if let Some(l) = &label {
            cx.claim_label(l, &uart_name);
        }
        if let Some(p) = tx_pin {
            cx.claim_pin(p, &format!("{uart_name} (TX)"));
        }
        if let Some(p) = rx_pin {
            cx.claim_pin(p, &format!("{uart_name} (RX)"));
        }

        if let (Some(index), Some(tx_pin), Some(rx_pin), Some(baud), Some(label)) =
            (index, tx_pin, rx_pin, baud, label)
        {
            out.push(Uart {
                index,
                tx_pin,
                rx_pin,
                baud,
                label,
            });
        }
    }
    out
}

//! Raw, lenient deserialization of `board.toml`.
//!
//! Every semantically-required field is modelled as an [`Option`] so that a
//! missing field does *not* abort deserialization. Presence and correctness are
//! enforced later in [`crate::config::validate`], which lets us collect and
//! report every problem in a single pass.

use serde::Deserialize;

/// The whole `board.toml` document, exactly as written by the user.
#[derive(Debug, Clone, Deserialize)]
pub struct RawConfig {
    /// The `[board]` table.
    pub board: RawBoard,
    /// Zero or more `[[gpio]]` entries.
    #[serde(default)]
    pub gpio: Vec<RawGpio>,
    /// Zero or more `[[i2c]]` buses.
    #[serde(default)]
    pub i2c: Vec<RawI2c>,
    /// Zero or more `[[spi]]` buses.
    #[serde(default)]
    pub spi: Vec<RawSpi>,
    /// Zero or more `[[uart]]` peripherals.
    #[serde(default)]
    pub uart: Vec<RawUart>,
}

/// The `[board]` table: global, mostly-informational metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct RawBoard {
    /// Board name, used in generated comments.
    pub name: Option<String>,
    /// MCU identifier, used in generated comments.
    pub mcu: Option<String>,
    /// System clock in MHz, used in generated comments / baud math.
    pub clock_mhz: Option<u32>,
}

/// A single `[[gpio]]` pin definition.
#[derive(Debug, Clone, Deserialize)]
pub struct RawGpio {
    /// Physical pin number.
    pub pin: Option<u32>,
    /// Pin mode: `input`, `output`, `input_pullup`, `input_pulldown`.
    pub mode: Option<String>,
    /// Human-readable label; becomes a C identifier.
    pub label: Option<String>,
    /// Optional interrupt edge: `rising`, `falling`, `both`, `none`.
    pub interrupt: Option<String>,
}

/// A single `[[i2c]]` bus, with any nested `[[i2c.device]]` entries.
#[derive(Debug, Clone, Deserialize)]
pub struct RawI2c {
    /// Hardware bus index.
    pub bus: Option<u32>,
    /// SDA (data) pin.
    pub sda_pin: Option<u32>,
    /// SCL (clock) pin.
    pub scl_pin: Option<u32>,
    /// Bus speed in kHz (typically 100 or 400).
    pub speed_khz: Option<u32>,
    /// Human-readable label; becomes part of C function names.
    pub label: Option<String>,
    /// Devices attached to this bus (`[[i2c.device]]`).
    #[serde(default, rename = "device")]
    pub devices: Vec<RawI2cDevice>,
}

/// A device on an I2C bus.
#[derive(Debug, Clone, Deserialize)]
pub struct RawI2cDevice {
    /// 7-bit I2C address.
    pub address: Option<u32>,
    /// Human-readable label; becomes part of C function names.
    pub label: Option<String>,
    /// Label of another peripheral that must initialize before this device.
    pub depends_on: Option<String>,
    /// GPIO pin that must be driven HIGH before this device initializes.
    pub power_pin: Option<u32>,
    /// How often this device is polled per second (used by analysis).
    pub poll_rate_hz: Option<u32>,
}

/// A single `[[spi]]` bus, with any nested `[[spi.device]]` entries.
#[derive(Debug, Clone, Deserialize)]
pub struct RawSpi {
    /// Hardware bus index.
    pub bus: Option<u32>,
    /// MOSI (controller-out) pin.
    pub mosi_pin: Option<u32>,
    /// MISO (controller-in) pin.
    pub miso_pin: Option<u32>,
    /// SCK (clock) pin.
    pub sck_pin: Option<u32>,
    /// Human-readable label; becomes part of C function names.
    pub label: Option<String>,
    /// Devices attached to this bus (`[[spi.device]]`).
    #[serde(default, rename = "device")]
    pub devices: Vec<RawSpiDevice>,
}

/// A device on an SPI bus.
#[derive(Debug, Clone, Deserialize)]
pub struct RawSpiDevice {
    /// Chip-select pin for this device.
    pub cs_pin: Option<u32>,
    /// Human-readable label; becomes part of C function names.
    pub label: Option<String>,
    /// SPI mode 0-3.
    pub mode: Option<u32>,
    /// Clock speed in MHz.
    pub speed_mhz: Option<u32>,
    /// Label of another peripheral that must initialize before this device.
    pub depends_on: Option<String>,
    /// GPIO pin that must be driven HIGH before this device initializes.
    pub power_pin: Option<u32>,
}

/// A single `[[uart]]` peripheral.
#[derive(Debug, Clone, Deserialize)]
pub struct RawUart {
    /// UART peripheral index (0, 1, 2, ...).
    pub index: Option<u32>,
    /// Transmit pin.
    pub tx_pin: Option<u32>,
    /// Receive pin.
    pub rx_pin: Option<u32>,
    /// Baud rate.
    pub baud: Option<u32>,
    /// Human-readable label; becomes part of C function names.
    pub label: Option<String>,
}

/// Deserialize a `board.toml` from an in-memory string.
///
/// Returns a [`crate::ForgeError::Parse`] if the document is not valid TOML.
pub fn parse_str(contents: &str) -> Result<RawConfig, crate::ForgeError> {
    let config = toml::from_str(contents)?;
    Ok(config)
}

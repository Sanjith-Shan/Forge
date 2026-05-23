//! The validated board model.
//!
//! These types are produced by [`crate::config::validate`] and consumed by
//! [`crate::codegen`]. Unlike the raw parse structs, every field here is
//! present and well-typed: by the time you hold a [`Board`], the configuration
//! is known to be valid.

/// A fully-validated description of an embedded board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    /// Board name (from `[board].name`).
    pub name: String,
    /// MCU identifier (from `[board].mcu`).
    pub mcu: String,
    /// System clock in MHz (from `[board].clock_mhz`).
    pub clock_mhz: u32,
    /// GPIO pins, in declaration order.
    pub gpios: Vec<Gpio>,
    /// I2C buses, in declaration order.
    pub i2c_buses: Vec<I2cBus>,
    /// SPI buses, in declaration order.
    pub spi_buses: Vec<SpiBus>,
    /// UART peripherals, in declaration order.
    pub uarts: Vec<Uart>,
}

impl Board {
    /// True if the board defines no peripherals at all.
    pub fn is_empty(&self) -> bool {
        self.gpios.is_empty()
            && self.i2c_buses.is_empty()
            && self.spi_buses.is_empty()
            && self.uarts.is_empty()
    }
}

/// A single GPIO pin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gpio {
    /// Physical pin number.
    pub pin: u32,
    /// Pin direction / pull configuration.
    pub mode: GpioMode,
    /// C-identifier label.
    pub label: String,
    /// Interrupt edge, if this pin raises interrupts.
    pub interrupt: Option<InterruptEdge>,
}

/// GPIO direction and pull configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioMode {
    /// High-impedance input.
    Input,
    /// Push-pull output.
    Output,
    /// Input with internal pull-up resistor.
    InputPullup,
    /// Input with internal pull-down resistor.
    InputPulldown,
}

impl GpioMode {
    /// Parse a mode from its TOML spelling.
    pub fn from_toml(s: &str) -> Option<Self> {
        match s {
            "input" => Some(Self::Input),
            "output" => Some(Self::Output),
            "input_pullup" => Some(Self::InputPullup),
            "input_pulldown" => Some(Self::InputPulldown),
            _ => None,
        }
    }

    /// The C macro used to configure this mode in generated code.
    pub fn as_c_macro(self) -> &'static str {
        match self {
            Self::Input => "GPIO_MODE_INPUT",
            Self::Output => "GPIO_MODE_OUTPUT",
            Self::InputPullup => "GPIO_MODE_INPUT_PULLUP",
            Self::InputPulldown => "GPIO_MODE_INPUT_PULLDOWN",
        }
    }

    /// A human-readable phrase for generated comments, e.g. `input, pull-up`.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::InputPullup => "input, pull-up",
            Self::InputPulldown => "input, pull-down",
        }
    }
}

/// The edge(s) on which a GPIO interrupt fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptEdge {
    /// Low-to-high transition.
    Rising,
    /// High-to-low transition.
    Falling,
    /// Both transitions.
    Both,
}

impl InterruptEdge {
    /// Parse an edge from its TOML spelling. `"none"` maps to `None` (no
    /// interrupt) and is handled by the caller.
    pub fn from_toml(s: &str) -> Option<Self> {
        match s {
            "rising" => Some(Self::Rising),
            "falling" => Some(Self::Falling),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// The C macro naming this edge in generated code.
    pub fn as_c_macro(self) -> &'static str {
        match self {
            Self::Rising => "GPIO_INTR_RISING",
            Self::Falling => "GPIO_INTR_FALLING",
            Self::Both => "GPIO_INTR_BOTH",
        }
    }

    /// A human-readable phrase for generated comments, e.g. `falling edge`.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Rising => "rising edge",
            Self::Falling => "falling edge",
            Self::Both => "both edges",
        }
    }
}

/// An I2C bus and the devices attached to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I2cBus {
    /// Hardware bus index.
    pub bus: u32,
    /// SDA (data) pin.
    pub sda_pin: u32,
    /// SCL (clock) pin.
    pub scl_pin: u32,
    /// Bus speed in kHz.
    pub speed_khz: u32,
    /// C-identifier label.
    pub label: String,
    /// Devices attached to this bus.
    pub devices: Vec<I2cDevice>,
}

/// A device on an I2C bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I2cDevice {
    /// 7-bit address (0x00-0x7F).
    pub address: u8,
    /// C-identifier label.
    pub label: String,
}

/// An SPI bus and the devices attached to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiBus {
    /// Hardware bus index.
    pub bus: u32,
    /// MOSI (controller-out) pin.
    pub mosi_pin: u32,
    /// MISO (controller-in) pin.
    pub miso_pin: u32,
    /// SCK (clock) pin.
    pub sck_pin: u32,
    /// C-identifier label.
    pub label: String,
    /// Devices attached to this bus.
    pub devices: Vec<SpiDevice>,
}

/// A device on an SPI bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiDevice {
    /// Chip-select pin.
    pub cs_pin: u32,
    /// C-identifier label.
    pub label: String,
    /// SPI mode (0-3).
    pub mode: u8,
    /// Clock speed in MHz.
    pub speed_mhz: u32,
}

/// A UART peripheral.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uart {
    /// Peripheral index.
    pub index: u32,
    /// Transmit pin.
    pub tx_pin: u32,
    /// Receive pin.
    pub rx_pin: u32,
    /// Baud rate.
    pub baud: u32,
    /// C-identifier label.
    pub label: String,
}

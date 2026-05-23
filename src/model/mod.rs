//! The validated board model consumed by code generation.

pub mod board;

pub use board::{Board, Gpio, GpioMode, I2cBus, I2cDevice, InterruptEdge, SpiBus, SpiDevice, Uart};

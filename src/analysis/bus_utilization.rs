//! I2C bus utilization estimation.
//!
//! Each polled device contributes traffic proportional to its poll rate and the
//! size of a transaction. We model a transaction as an address byte, a register
//! byte, and an assumed payload, each taking 9 bit-times (8 data + 1 ACK) on the
//! bus clock.

use crate::model::I2cBus;

/// Assumed data-payload bytes per transaction.
const PAYLOAD_BYTES: u32 = 2;
/// Bit-times per byte on the wire: 8 data bits + 1 ACK.
const BITS_PER_BYTE: u32 = 9;
/// Warn above this fraction of bus capacity.
const WARN_THRESHOLD: f64 = 0.70;

/// Estimate a bus's utilization as a fraction of capacity, considering only
/// devices that declare a `poll_rate_hz`. Returns `None` if no device on the
/// bus is polled (nothing to estimate).
pub fn utilization(bus: &I2cBus) -> Option<f64> {
    let bus_hz = bus.speed_khz as f64 * 1000.0;
    if bus_hz <= 0.0 {
        return None;
    }
    let bits_per_txn = f64::from(1 + 1 + PAYLOAD_BYTES) * f64::from(BITS_PER_BYTE);

    let mut total = 0.0;
    let mut polled = 0;
    for d in &bus.devices {
        if let Some(rate) = d.poll_rate_hz {
            total += f64::from(rate) * bits_per_txn / bus_hz;
            polled += 1;
        }
    }
    (polled > 0).then_some(total)
}

/// Number of devices on the bus that declare a poll rate.
pub fn polled_device_count(bus: &I2cBus) -> usize {
    bus.devices
        .iter()
        .filter(|d| d.poll_rate_hz.is_some())
        .count()
}

/// Produce a warning for every I2C bus whose estimated utilization exceeds the
/// threshold.
pub fn check(buses: &[I2cBus]) -> Vec<String> {
    let mut warnings = Vec::new();
    for bus in buses {
        if let Some(util) = utilization(bus) {
            if util > WARN_THRESHOLD {
                warnings.push(format!(
                    "I2C bus '{}' estimated at ~{}% utilization with {} polled device(s) \
                     on a {} kHz bus. Consider splitting across two buses.",
                    bus.label,
                    (util * 100.0).round() as u32,
                    polled_device_count(bus),
                    bus.speed_khz,
                ));
            }
        }
    }
    warnings
}

//! SPI clock-speed validation.
//!
//! SPI peripherals derive their clock from the system clock through power-of-two
//! prescalers, so the achievable speed is rarely exactly what was requested.
//! This check flags devices whose closest achievable speed is more than 10% off.

use crate::model::Board;

/// Warn when the achievable speed differs from the request by more than this.
const TOLERANCE: f64 = 0.10;

/// The closest achievable SPI speed (MHz) for `requested_mhz` on a given system
/// clock, using a power-of-two prescaler, plus the prescaler used.
pub fn closest_achievable(system_mhz: u32, requested_mhz: u32) -> (f64, u32) {
    let sys = f64::from(system_mhz);
    let req = f64::from(requested_mhz);
    // ceil(log2(sys/req)), clamped to >= 0 so the prescaler is at least 1.
    let exponent = (sys / req).log2().ceil().max(0.0);
    let prescaler = 2f64.powf(exponent);
    (sys / prescaler, prescaler as u32)
}

/// Produce a warning for every SPI device whose achievable clock is out of
/// tolerance.
pub fn check(board: &Board) -> Vec<String> {
    let mut warnings = Vec::new();
    for bus in &board.spi_buses {
        for d in &bus.devices {
            if d.speed_mhz == 0 {
                continue;
            }
            let (actual, prescaler) = closest_achievable(board.clock_mhz, d.speed_mhz);
            let req = f64::from(d.speed_mhz);
            let error = (actual - req).abs() / req;
            if error > TOLERANCE {
                let direction = if actual < req { "lower" } else { "higher" };
                warnings.push(format!(
                    "SPI device '{}' requested {} MHz but closest achievable is {} MHz \
                     (system clock {} MHz / prescaler {}). Actual speed is {}% {} than requested.",
                    d.label,
                    d.speed_mhz,
                    format_mhz(actual),
                    board.clock_mhz,
                    prescaler,
                    (error * 100.0).round() as u32,
                    direction,
                ));
            }
        }
    }
    warnings
}

/// Format a MHz value: whole numbers without a decimal, otherwise two places.
fn format_mhz(value: f64) -> String {
    if (value.fract()).abs() < 1e-9 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.2}")
    }
}

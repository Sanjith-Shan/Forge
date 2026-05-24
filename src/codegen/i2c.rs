//! I2C code generation: per bus, an init plus blocking read/write helpers; per
//! device, an init stub the developer fills in with device-specific setup.

use crate::model::I2cBus;

/// Declaration block for `init.h`. Empty when there are no I2C buses.
pub fn decl_section(buses: &[I2cBus]) -> Option<String> {
    if buses.is_empty() {
        return None;
    }
    let mut lines = vec!["/* I2C */".to_string()];
    for b in buses {
        lines.push(format!("void i2c_{}_init(void);", b.label));
        lines.push(format!(
            "void i2c_{}_write(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len);",
            b.label
        ));
        lines.push(format!(
            "void i2c_{}_read(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len);",
            b.label
        ));
        for d in &b.devices {
            lines.push(format!("void i2c_{}_init(void);", d.label));
        }
    }
    Some(lines.join("\n"))
}

/// Implementation functions for `init.c`: per bus init/write/read, then a
/// per-device init stub.
pub fn impl_fns(buses: &[I2cBus]) -> Vec<String> {
    let mut fns = Vec::new();
    for b in buses {
        let mut init = format!("void i2c_{}_init(void) {{\n", b.label);
        init.push_str(&format!(
            "    /* I2C bus {}: SDA={}, SCL={}, {} kHz */\n",
            b.bus, b.sda_pin, b.scl_pin, b.speed_khz
        ));
        if !b.devices.is_empty() {
            let devices = b
                .devices
                .iter()
                .map(|d| format!("{} @ {:#04X}", d.label, d.address))
                .collect::<Vec<_>>()
                .join(", ");
            init.push_str(&format!("    /* Devices: {devices} */\n"));
        }
        init.push_str(&format!(
            "    I2C_INIT({}, {}, {}, {});\n}}",
            b.bus,
            b.sda_pin,
            b.scl_pin,
            b.speed_khz * 1000
        ));
        fns.push(init);

        fns.push(format!(
            "void i2c_{label}_write(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len) {{\n    \
             /* Write `len` bytes from `data` to register `reg` on device `addr` over I2C bus {bus} */\n    \
             I2C_WRITE({bus}, addr, reg, data, len);\n}}",
            label = b.label,
            bus = b.bus,
        ));

        fns.push(format!(
            "void i2c_{label}_read(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len) {{\n    \
             /* Read `len` bytes into `data` from register `reg` on device `addr` over I2C bus {bus} */\n    \
             I2C_READ({bus}, addr, reg, data, len);\n}}",
            label = b.label,
            bus = b.bus,
        ));

        for d in &b.devices {
            fns.push(format!(
                "void i2c_{label}_init(void) {{\n    \
                 /* TODO: Initialize {label} (I2C device {addr:#04X} on bus {bus}) */\n}}",
                label = d.label,
                addr = d.address,
                bus = b.label,
            ));
        }
    }
    fns
}

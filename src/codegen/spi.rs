//! SPI code generation: per bus, an init and full-duplex transfer; per device,
//! a chip-select-configuring init plus assert/deassert helpers.

use crate::model::{Gpio, SpiBus};

/// Declaration block for `init.h`. Empty when there are no SPI buses.
pub fn decl_section(buses: &[SpiBus]) -> Option<String> {
    if buses.is_empty() {
        return None;
    }
    let mut lines = vec!["/* SPI */".to_string()];
    for b in buses {
        lines.push(format!("void spi_{}_init(void);", b.label));
        for d in &b.devices {
            lines.push(format!("void spi_{}_init(void);", d.label));
            lines.push(format!("void spi_{}_select(void);", d.label));
            lines.push(format!("void spi_{}_deselect(void);", d.label));
        }
        lines.push(format!(
            "void spi_{}_transfer(uint8_t *tx, uint8_t *rx, uint16_t len);",
            b.label
        ));
    }
    Some(lines.join("\n"))
}

/// Implementation functions for `init.c`.
///
/// `gpios` is consulted so a chip-select pin that is *also* a declared GPIO
/// output is configured once (by its `gpio_init_*` function), not twice.
pub fn impl_fns(buses: &[SpiBus], gpios: &[Gpio]) -> Vec<String> {
    let mut fns = Vec::new();
    for b in buses {
        fns.push(format!(
            "void spi_{label}_init(void) {{\n    \
             /* SPI bus {bus}: MOSI={mosi}, MISO={miso}, SCK={sck} */\n    \
             SPI_INIT({bus}, {mosi}, {miso}, {sck});\n}}",
            label = b.label,
            bus = b.bus,
            mosi = b.mosi_pin,
            miso = b.miso_pin,
            sck = b.sck_pin,
        ));

        for d in &b.devices {
            let mut init = format!("void spi_{}_init(void) {{\n", d.label);
            init.push_str(&format!(
                "    /* SPI device {}: CS={}, mode {}, {} MHz on bus {} */\n",
                d.label, d.cs_pin, d.mode, d.speed_mhz, b.label
            ));
            match gpios.iter().find(|g| g.pin == d.cs_pin && g.is_output()) {
                Some(g) => init.push_str(&format!(
                    "    /* CS pin {} configured by gpio_init_{} */\n",
                    d.cs_pin, g.label
                )),
                None => init.push_str(&format!(
                    "    GPIO_SET_MODE({}, GPIO_MODE_OUTPUT);\n",
                    d.cs_pin
                )),
            }
            init.push_str(&format!(
                "    SPI_CONFIG_CS({}, SPI_MODE_{}, {});\n}}",
                d.cs_pin,
                d.mode,
                d.speed_mhz * 1_000_000
            ));
            fns.push(init);

            fns.push(format!(
                "void spi_{label}_select(void) {{\n    \
                 /* Pull CS low to select {label} */\n    \
                 GPIO_WRITE({cs}, 0);\n}}",
                label = d.label,
                cs = d.cs_pin,
            ));
            fns.push(format!(
                "void spi_{label}_deselect(void) {{\n    \
                 /* Pull CS high to deselect {label} */\n    \
                 GPIO_WRITE({cs}, 1);\n}}",
                label = d.label,
                cs = d.cs_pin,
            ));
        }

        fns.push(format!(
            "void spi_{label}_transfer(uint8_t *tx, uint8_t *rx, uint16_t len) {{\n    \
             /* Full-duplex transfer of `len` bytes on SPI bus {bus} */\n    \
             SPI_TRANSFER({bus}, tx, rx, len);\n}}",
            label = b.label,
            bus = b.bus,
        ));
    }
    fns
}

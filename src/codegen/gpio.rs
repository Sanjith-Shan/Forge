//! GPIO code generation: one `gpio_init_<label>` per pin (so each can sit at
//! its own dependency layer) plus one ISR per interrupt-enabled pin.

use crate::model::Gpio;

/// The C name of the per-pin init function.
pub fn init_name(g: &Gpio) -> String {
    format!("gpio_init_{}", g.label)
}

/// The C name of the ISR generated for an interrupt-enabled pin.
fn isr_name(g: &Gpio) -> String {
    format!("{}_isr", g.label)
}

/// Declaration block for `init.h`. Empty when there are no GPIO pins.
pub fn decl_section(gpios: &[Gpio]) -> Option<String> {
    if gpios.is_empty() {
        return None;
    }
    let mut lines = vec!["/* GPIO */".to_string()];
    for g in gpios {
        lines.push(format!("void {}(void);", init_name(g)));
    }
    Some(lines.join("\n"))
}

/// One init function per pin for `init.c`.
pub fn impl_fns(gpios: &[Gpio]) -> Vec<String> {
    gpios
        .iter()
        .map(|g| {
            let intr_note = match g.interrupt {
                Some(edge) => format!(", interrupt on {}", edge.describe()),
                None => String::new(),
            };
            let mut body = format!("void {}(void) {{\n", init_name(g));
            body.push_str(&format!(
                "    /* Pin {}: {} ({}{}) */\n",
                g.pin,
                g.label,
                g.mode.describe(),
                intr_note
            ));
            body.push_str(&format!(
                "    GPIO_SET_MODE({}, {});\n",
                g.pin,
                g.mode.as_c_macro()
            ));
            if let Some(edge) = g.interrupt {
                body.push_str(&format!(
                    "    GPIO_ATTACH_INTERRUPT({}, {}, {});\n",
                    g.pin,
                    edge.as_c_macro(),
                    isr_name(g)
                ));
            }
            body.push('}');
            body
        })
        .collect()
}

/// Declaration block of GPIO ISRs for `handlers.h`. Only interrupt-enabled pins
/// contribute. Empty when none do.
pub fn handler_decl_section(gpios: &[Gpio]) -> Option<String> {
    let mut lines = vec!["/* GPIO interrupt handlers */".to_string()];
    for g in gpios.iter().filter(|g| g.interrupt.is_some()) {
        lines.push(format!("void {}(void);", isr_name(g)));
    }
    if lines.len() == 1 {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// ISR stub definitions for `handlers.c`, one per interrupt-enabled pin.
pub fn handler_impl_fns(gpios: &[Gpio]) -> Vec<String> {
    gpios
        .iter()
        .filter_map(|g| {
            let edge = g.interrupt?;
            Some(format!(
                "void {name}(void) {{\n    /* TODO: Handle {edge} on pin {pin} ({label}) */\n}}",
                name = isr_name(g),
                edge = edge.describe(),
                pin = g.pin,
                label = g.label,
            ))
        })
        .collect()
}

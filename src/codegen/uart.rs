//! UART code generation: per peripheral, an init that wires up an RX interrupt,
//! a blocking send, and an RX ISR stub.

use crate::model::Uart;

/// The C name of the RX ISR generated for a UART.
fn rx_isr_name(u: &Uart) -> String {
    format!("uart_{}_rx_isr", u.label)
}

/// Declaration block for `init.h`. Empty when there are no UARTs.
pub fn decl_section(uarts: &[Uart]) -> Option<String> {
    if uarts.is_empty() {
        return None;
    }
    let mut lines = vec!["/* UART */".to_string()];
    for u in uarts {
        lines.push(format!("void uart_{}_init(void);", u.label));
        lines.push(format!(
            "void uart_{}_send(const uint8_t *data, uint16_t len);",
            u.label
        ));
    }
    Some(lines.join("\n"))
}

/// Implementation functions for `init.c`: init and send per UART.
pub fn impl_fns(uarts: &[Uart]) -> Vec<String> {
    let mut fns = Vec::new();
    for u in uarts {
        fns.push(format!(
            "void uart_{label}_init(void) {{\n    \
             /* UART {index}: TX={tx}, RX={rx}, {baud} baud */\n    \
             UART_INIT({index}, {tx}, {rx}, {baud});\n    \
             UART_ATTACH_RX_INTERRUPT({index}, {isr});\n}}",
            label = u.label,
            index = u.index,
            tx = u.tx_pin,
            rx = u.rx_pin,
            baud = u.baud,
            isr = rx_isr_name(u),
        ));
        fns.push(format!(
            "void uart_{label}_send(const uint8_t *data, uint16_t len) {{\n    \
             /* Send `len` bytes over UART {index} */\n    \
             UART_SEND({index}, data, len);\n}}",
            label = u.label,
            index = u.index,
        ));
    }
    fns
}

/// RX ISR declaration block for `handlers.h`. Empty when there are no UARTs.
pub fn handler_decl_section(uarts: &[Uart]) -> Option<String> {
    if uarts.is_empty() {
        return None;
    }
    let mut lines = vec!["/* UART RX handlers */".to_string()];
    for u in uarts {
        lines.push(format!("void {}(void);", rx_isr_name(u)));
    }
    Some(lines.join("\n"))
}

/// RX ISR stub definitions for `handlers.c`, one per UART.
pub fn handler_impl_fns(uarts: &[Uart]) -> Vec<String> {
    uarts
        .iter()
        .map(|u| {
            format!(
                "void {isr}(void) {{\n    /* TODO: Handle incoming data on UART {index} ({label}) */\n}}",
                isr = rx_isr_name(u),
                index = u.index,
                label = u.label,
            )
        })
        .collect()
}

//! The resource-utilization summary printed after every successful run.

use crate::analysis::interrupts::interrupt_count;
use crate::model::Board;

/// `count` followed by the singular or plural form of a noun.
fn plural(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Render the resource summary as a multi-line string.
pub fn render(board: &Board, layer_count: usize) -> String {
    let i2c_devices: usize = board.i2c_buses.iter().map(|b| b.devices.len()).sum();
    let spi_devices: usize = board.spi_buses.iter().map(|b| b.devices.len()).sum();

    let mut s = format!(
        "Board: {} ({} @ {} MHz)\n",
        board.name, board.mcu, board.clock_mhz
    );
    s.push_str(&format!(
        "  GPIO:  {} ({} with interrupts)\n",
        plural(board.gpios.len(), "pin", "pins"),
        interrupt_count(board),
    ));
    s.push_str(&format!(
        "  I2C:   {}, {}\n",
        plural(board.i2c_buses.len(), "bus", "buses"),
        plural(i2c_devices, "device", "devices"),
    ));
    s.push_str(&format!(
        "  SPI:   {}, {}\n",
        plural(board.spi_buses.len(), "bus", "buses"),
        plural(spi_devices, "device", "devices"),
    ));
    s.push_str(&format!(
        "  UART:  {}\n",
        plural(board.uarts.len(), "port", "ports")
    ));
    s.push_str(&format!(
        "  Init order: {}, 0 cycles\n",
        plural(layer_count, "layer", "layers"),
    ));
    s
}

/// Print the summary plus a warning count to stdout.
pub fn print(board: &Board, layer_count: usize, warning_count: usize) {
    print!("{}", render(board, layer_count));
    println!("  Warnings: {warning_count}");
}

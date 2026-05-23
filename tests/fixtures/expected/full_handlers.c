#include "handlers.h"

void user_button_isr(void) {
    /* TODO: Handle falling edge on pin 4 (user_button) */
}

void uart_debug_serial_rx_isr(void) {
    /* TODO: Handle incoming data on UART 1 (debug_serial) */
}

void uart_gps_rx_isr(void) {
    /* TODO: Handle incoming data on UART 2 (gps) */
}

#include "init.h"
#include "handlers.h"

void gpio_init_led(void) {
    /* Pin 13: led (output) */
    GPIO_SET_MODE(13, GPIO_MODE_OUTPUT);
}

void gpio_init_button(void) {
    /* Pin 0: button (input, pull-up, interrupt on rising edge) */
    GPIO_SET_MODE(0, GPIO_MODE_INPUT_PULLUP);
    GPIO_ATTACH_INTERRUPT(0, GPIO_INTR_RISING, button_isr);
}

void uart_console_init(void) {
    /* UART 0: TX=1, RX=3, 9600 baud */
    UART_INIT(0, 1, 3, 9600);
    UART_ATTACH_RX_INTERRUPT(0, uart_console_rx_isr);
}

void uart_console_send(const uint8_t *data, uint16_t len) {
    /* Send `len` bytes over UART 0 */
    UART_SEND(0, data, len);
}

void board_init(void) {
    /* === Initialization order resolved by dependency analysis === */

    /* Layer 0: no dependencies */
    gpio_init_button();  /* pin 0, input, pull-up */
    uart_console_init();  /* uart 0 */
    gpio_init_led();  /* pin 13, output */
}

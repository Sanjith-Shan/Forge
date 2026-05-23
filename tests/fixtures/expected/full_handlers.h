#ifndef FORGE_HANDLERS_H
#define FORGE_HANDLERS_H

/* GPIO interrupt handlers */
void user_button_isr(void);

/* UART RX handlers */
void uart_debug_serial_rx_isr(void);
void uart_gps_rx_isr(void);

#endif /* FORGE_HANDLERS_H */

#ifndef FORGE_HANDLERS_H
#define FORGE_HANDLERS_H

/* GPIO interrupt handlers */
void button_isr(void);

/* UART RX handlers */
void uart_console_rx_isr(void);

#endif /* FORGE_HANDLERS_H */

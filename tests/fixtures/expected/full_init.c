#include "init.h"
#include "handlers.h"

void gpio_init_all(void) {
    /* Pin 13: status_led (output) */
    GPIO_SET_MODE(13, GPIO_MODE_OUTPUT);

    /* Pin 4: user_button (input, pull-up, interrupt on falling edge) */
    GPIO_SET_MODE(4, GPIO_MODE_INPUT_PULLUP);
    GPIO_ATTACH_INTERRUPT(4, GPIO_INTR_FALLING, user_button_isr);
}

void i2c_sensor_bus_init(void) {
    /* I2C bus 0: SDA=21, SCL=22, 400 kHz */
    /* Devices: potentiometer @ 0x2C, imu @ 0x68 */
    I2C_INIT(0, 21, 22, 400000);
}

void i2c_sensor_bus_write(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len) {
    /* Write `len` bytes from `data` to register `reg` on device `addr` over I2C bus 0 */
    I2C_WRITE(0, addr, reg, data, len);
}

void i2c_sensor_bus_read(uint8_t addr, uint8_t reg, uint8_t *data, uint16_t len) {
    /* Read `len` bytes into `data` from register `reg` on device `addr` over I2C bus 0 */
    I2C_READ(0, addr, reg, data, len);
}

void spi_display_bus_init(void) {
    /* SPI bus 0: MOSI=23, MISO=19, SCK=18 */
    SPI_INIT(0, 23, 19, 18);
    /* Chip-select: oled on pin 5 (mode 0, 10 MHz) */
    GPIO_SET_MODE(5, GPIO_MODE_OUTPUT);
    SPI_CONFIG_CS(5, SPI_MODE_0, 10000000);
}

void spi_oled_select(void) {
    /* Pull CS low to select oled */
    GPIO_WRITE(5, 0);
}

void spi_oled_deselect(void) {
    /* Pull CS high to deselect oled */
    GPIO_WRITE(5, 1);
}

void spi_display_bus_transfer(uint8_t *tx, uint8_t *rx, uint16_t len) {
    /* Full-duplex transfer of `len` bytes on SPI bus 0 */
    SPI_TRANSFER(0, tx, rx, len);
}

void uart_debug_serial_init(void) {
    /* UART 1: TX=17, RX=16, 115200 baud */
    UART_INIT(1, 17, 16, 115200);
    UART_ATTACH_RX_INTERRUPT(1, uart_debug_serial_rx_isr);
}

void uart_debug_serial_send(const uint8_t *data, uint16_t len) {
    /* Send `len` bytes over UART 1 */
    UART_SEND(1, data, len);
}

void uart_gps_init(void) {
    /* UART 2: TX=25, RX=26, 9600 baud */
    UART_INIT(2, 25, 26, 9600);
    UART_ATTACH_RX_INTERRUPT(2, uart_gps_rx_isr);
}

void uart_gps_send(const uint8_t *data, uint16_t len) {
    /* Send `len` bytes over UART 2 */
    UART_SEND(2, data, len);
}

void board_init(void) {
    gpio_init_all();
    i2c_sensor_bus_init();
    spi_display_bus_init();
    uart_debug_serial_init();
    uart_gps_init();
}

#include "init.h"
#include "handlers.h"

void gpio_init_sensor_power(void) {
    /* Pin 12: sensor_power (output) */
    GPIO_SET_MODE(12, GPIO_MODE_OUTPUT);
}

void gpio_init_cs_flash(void) {
    /* Pin 5: cs_flash (output) */
    GPIO_SET_MODE(5, GPIO_MODE_OUTPUT);
}

void i2c_sensor_bus_init(void) {
    /* I2C bus 0: SDA=21, SCL=22, 400 kHz */
    /* Devices: i2c_mux @ 0x70, imu @ 0x68, barometer @ 0x76 */
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

void i2c_i2c_mux_init(void) {
    /* TODO: Initialize i2c_mux (I2C device 0x70 on bus sensor_bus) */
}

void i2c_imu_init(void) {
    /* TODO: Initialize imu (I2C device 0x68 on bus sensor_bus) */
}

void i2c_barometer_init(void) {
    /* TODO: Initialize barometer (I2C device 0x76 on bus sensor_bus) */
}

void spi_storage_bus_init(void) {
    /* SPI bus 0: MOSI=23, MISO=19, SCK=18 */
    SPI_INIT(0, 23, 19, 18);
}

void spi_flash_init(void) {
    /* SPI device flash: CS=5, mode 0, 20 MHz on bus storage_bus */
    /* CS pin 5 configured by gpio_init_cs_flash */
    SPI_CONFIG_CS(5, SPI_MODE_0, 20000000);
}

void spi_flash_select(void) {
    /* Pull CS low to select flash */
    GPIO_WRITE(5, 0);
}

void spi_flash_deselect(void) {
    /* Pull CS high to deselect flash */
    GPIO_WRITE(5, 1);
}

void spi_storage_bus_transfer(uint8_t *tx, uint8_t *rx, uint16_t len) {
    /* Full-duplex transfer of `len` bytes on SPI bus 0 */
    SPI_TRANSFER(0, tx, rx, len);
}

void board_init(void) {
    /* === Initialization order resolved by dependency analysis === */

    /* Layer 0: no dependencies */
    gpio_init_cs_flash();  /* pin 5, output */
    i2c_sensor_bus_init();  /* i2c bus 0 */
    gpio_init_sensor_power();  /* pin 12, output */
    spi_storage_bus_init();  /* spi bus 0 */

    /* Layer 1: depends on Layer 0 */
    spi_flash_init();  /* spi cs 5 */
    i2c_i2c_mux_init();  /* i2c 0x70 */

    /* Layer 2: depends on Layer 1 */
    i2c_barometer_init();  /* i2c 0x76, depends_on "i2c_mux" */
    GPIO_WRITE(12, HIGH);  /* Power on imu before init */
    i2c_imu_init();  /* i2c 0x68, depends_on "i2c_mux" */
}

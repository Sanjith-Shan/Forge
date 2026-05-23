# Forge

> Generate embedded C boilerplate from TOML hardware descriptions.

Forge is a Rust CLI that reads a TOML file describing your board — which
peripherals are wired to which pins, at what settings — and generates
ready-to-compile C: initialization code, interrupt-handler stubs, and a main
loop skeleton.

Think of it like Prisma for embedded hardware. You describe your board, it
writes your boilerplate.

## Quick example

Given `board.toml`:

```toml
[board]
name = "blinky"
mcu = "atmega328p"
clock_mhz = 16

[[gpio]]
pin = 13
mode = "output"
label = "led"
```

Running `forge board.toml` generates `init.h`, `init.c`, `handlers.h`,
`handlers.c`, and `main.c`. The heart of it:

```c
/* init.c */
void gpio_init_all(void) {
    /* Pin 13: led (output) */
    GPIO_SET_MODE(13, GPIO_MODE_OUTPUT);
}

void board_init(void) {
    gpio_init_all();
}
```

```c
/* main.c */
#include "init.h"
#include "handlers.h"

int main(void) {
    board_init();

    while (1) {
        /* TODO: Main loop */
    }

    return 0;
}
```

The generated code uses readable placeholder macros (`GPIO_SET_MODE`,
`I2C_INIT`, `SPI_TRANSFER`, ...) that you map to your MCU's HAL. The structure
is done; you fill in the register-level details.

## Why

When you start an embedded project you write the same C initialization patterns
over and over — I2C bus setup, SPI configuration, UART serial init, GPIO pin
modes, interrupt handlers. The structure is always the same; only the addresses,
pins, baud rates, and bus numbers change. Forge eliminates that repetitive work
and keeps your wiring description in one declarative place.

## Installation

```bash
cargo install --path .
```

This installs the `forge` binary into `~/.cargo/bin`.

## Usage

```
USAGE:
    forge [OPTIONS] <CONFIG>

ARGS:
    <CONFIG>    Path to the board TOML config file

OPTIONS:
    -o, --output <DIR>     Output directory for generated files [default: ./output]
    -v, --verbose          Print detailed generation info
        --dry-run          Parse and validate config without generating files
        --check            Validate config only, exit 0 if valid
    -h, --help             Print help
    -V, --version          Print version
```

Examples:

```bash
forge board.toml                    # Generate into ./output/
forge board.toml -o src/generated/  # Generate into a custom directory
forge board.toml --check            # Validate only
forge board.toml --dry-run -v       # Parse, validate, show what would be generated
```

See the [`examples/`](examples/) directory for complete configs:

- [`minimal.toml`](examples/minimal.toml) — the simplest board (one GPIO)
- [`jupiter.toml`](examples/jupiter.toml) — a real board: AD5252 digital
  potentiometers on I2C, relay-control GPIOs, and a serial tracking link
- [`full.toml`](examples/full.toml) — every supported peripheral exercised

## Supported peripherals

| Peripheral | Generates |
|------------|-----------|
| **GPIO**   | `gpio_init_all`, optional interrupt attach + ISR stub per pin |
| **I2C**    | per-bus `init`, `write`, `read` helpers; device addresses validated |
| **SPI**    | per-bus `init` + `transfer`; per-device `select`/`deselect` |
| **UART**   | per-UART `init` + `send`; RX interrupt + ISR stub |

GPIO modes: `input`, `output`, `input_pullup`, `input_pulldown`. GPIO interrupt
edges: `rising`, `falling`, `both`, `none`. SPI modes: `0`–`3`. I2C addresses are
7-bit (`0x00`–`0x7F`).

## Validation

Forge validates the whole config and reports **all** problems at once (not just
the first), so you fix everything in one pass:

- **Pin conflicts** — the same pin claimed by two peripherals, naming both
- **I2C addresses** — must be 7-bit; reserved ranges (`0x00`–`0x07`,
  `0x78`–`0x7F`) raise a warning
- **Duplicate labels** — labels become C identifiers, so they must be unique
- **SPI mode** — must be `0`, `1`, `2`, or `3`
- **Missing required fields** — every peripheral's required fields are checked
- **GPIO interrupt mode** — must be a valid edge
- **Label format** — labels must be valid C identifiers

```
$ forge board.toml --check
error: configuration is invalid (1 error):
  1. Pin 21 is claimed by both gpio 'status_led' and i2c bus 'sensor_bus' (SDA). Each pin can only be assigned to one peripheral.
```

## How it works

```
board.toml ─▶ parse ─▶ validate ─▶ Board model ─▶ codegen ─▶ *.c / *.h
```

1. **Parse** (`src/config/parse.rs`) — lenient TOML deserialization
2. **Validate** (`src/config/validate.rs`) — collect every error/warning, lower
   into the typed [`Board`](src/model/board.rs) model
3. **Generate** (`src/codegen/`) — one generator per peripheral type; the five
   output files are rendered concurrently with Tokio

## Development

```bash
cargo build           # build
cargo test            # run unit, snapshot, and integration tests
cargo clippy          # lint
cargo fmt             # format
cargo run -- examples/jupiter.toml -o /tmp/out
```

## License

MIT — see [LICENSE](LICENSE).

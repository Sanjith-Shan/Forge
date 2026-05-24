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
void gpio_init_led(void) {
    /* Pin 13: led (output) */
    GPIO_SET_MODE(13, GPIO_MODE_OUTPUT);
}

void board_init(void) {
    /* === Initialization order resolved by dependency analysis === */

    /* Layer 0: no dependencies */
    gpio_init_led();  /* pin 13, output */
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
cargo install forge-embedded   # from crates.io
# or, from a local checkout:
cargo install --path .
```

Either way the installed binary is called `forge` (the crate is published as
`forge-embedded` because `forge` is taken on crates.io).

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
        --graph            Print the dependency graph as Graphviz DOT and exit
        --report           Write a detailed analysis.txt alongside the output
        --no-warnings      Suppress analysis warnings (the summary still prints)
    -h, --help             Print help
    -V, --version          Print version
```

Examples:

```bash
forge board.toml                       # Generate into ./output/
forge board.toml -o src/generated/     # Generate into a custom directory
forge board.toml --check               # Validate only
forge board.toml --report              # Generate + write analysis.txt
forge board.toml --graph | dot -Tpng -o deps.png   # Visualize dependencies
```

See the [`examples/`](examples/) directory for complete configs:

- [`minimal.toml`](examples/minimal.toml) — the simplest board (one GPIO)
- [`jupiter.toml`](examples/jupiter.toml) — a real board: AD5252 digital
  potentiometers on I2C, relay-control GPIOs, and a serial tracking link
- [`full.toml`](examples/full.toml) — every supported peripheral exercised
- [`sensor_hub.toml`](examples/sensor_hub.toml) — dependency ordering: an I2C
  mux, a power-gated IMU, and a flash chip on a dedicated chip-select GPIO

## Supported peripherals

| Peripheral | Generates |
|------------|-----------|
| **GPIO**   | one `gpio_init_<label>` per pin, optional interrupt attach + ISR stub |
| **I2C**    | per-bus `init`/`write`/`read` helpers, per-device `init` stub; addresses validated |
| **SPI**    | per-bus `init` + `transfer`; per-device `init`/`select`/`deselect` |
| **UART**   | per-UART `init` + `send`; RX interrupt + ISR stub |

GPIO modes: `input`, `output`, `input_pullup`, `input_pulldown`. GPIO interrupt
edges: `rising`, `falling`, `both`, `none`. SPI modes: `0`–`3`. I2C addresses are
7-bit (`0x00`–`0x7F`).

## Dependency ordering

Peripherals depend on each other: a chip-select GPIO must be configured before
the SPI device that uses it, an I2C mux must come up before the sensors behind
it, a power rail must be switched on before the chip it feeds. Get the order
wrong and you get silent runtime failures.

Forge builds a dependency graph, topologically sorts it (Kahn's algorithm), and
emits `board_init()` in dependency-resolved layers. Declare dependencies with two
optional fields on any device:

```toml
[[i2c.device]]
address = 0x68
label = "imu"
depends_on = "i2c_mux"   # initialize i2c_mux first
power_pin = 12           # drive GPIO 12 HIGH before init
```

It also infers dependencies automatically: devices on a bus depend on the bus,
and an SPI `cs_pin` (or any `power_pin`) that matches a declared GPIO depends on
that GPIO. Cycles are reported with the offending path. Use `--graph` to render
the graph with Graphviz.

## Analysis

After validation, Forge runs non-fatal checks and prints a resource summary:

- **I2C bus utilization** — estimates load from per-device `poll_rate_hz` and
  warns above ~70% of bus capacity.
- **SPI clock accuracy** — computes the closest power-of-two prescaler speed and
  warns when it is more than 10% off the requested rate.
- **Interrupt density** — warns when more than three GPIO interrupts are configured.

`--report` writes a full `analysis.txt` (dependency graph, init layers, warnings,
resource utilization); `--no-warnings` silences the warnings but keeps the summary.

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
board.toml ─▶ parse ─▶ validate ─▶ Board ─▶ graph ─▶ analysis ─▶ codegen ─▶ *.c / *.h
```

1. **Parse** (`src/config/parse.rs`) — lenient TOML deserialization
2. **Validate** (`src/config/validate.rs`) — collect every error/warning, lower
   into the typed [`Board`](src/model/board.rs) model
3. **Graph** (`src/graph/`) — build the dependency DAG, validate `depends_on` /
   `power_pin`, detect cycles, and compute init layers (Kahn's algorithm)
4. **Analysis** (`src/analysis/`) — non-fatal hardware checks + resource summary
5. **Generate** (`src/codegen/`) — one generator per peripheral type; the five
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

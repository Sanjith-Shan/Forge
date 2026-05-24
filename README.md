# Forge

> A vendor-neutral hardware bring-up compiler: describe a board once, verify it, and generate initialization code for any target.

Forge reads a simple description of an embedded board — which peripherals are
wired to which pins, at what settings, and how they depend on each other —
**verifies it deterministically**, and **generates target code** (portable C
today, Zephyr devicetree today, more later) from a single source of truth.

It is built like a compiler: **many frontends → one verified IR → many
backends.**

```
   FRONTENDS                  CORE (deterministic, trusted)            BACKENDS
 ┌──────────────┐
 │ board.toml   │──┐                                            ┌──> C scaffolding
 │ (hand-written)  │    parse → VALIDATE → Board IR → LINT  ───>┤
 │ natural lang │──┘         (graph + analysis)                 └──> Zephyr devicetree
 │ /datasheet(AI)│                  ▲
 └──────────────┘   every frontend's output runs the same gate
```

The point: whether a config is hand-written or proposed by a language model, it
goes through the **same deterministic verification gate** before anything is
generated. The AI proposes; the trusted core disposes.

## Why this, when STM32CubeMX and Zephyr exist?

The incumbents are vendor-locked (CubeMX = ST, NXP tools = NXP) and chip-local,
and raw LLM code generators are unreliable on the exact details (timing,
ordering, register correctness). Forge sits in the gap:

- **Vendor-neutral IR → multiple backends.** One description, many targets.
- **System-level reasoning the incumbents under-serve.** Dependency-ordered
  init, bus-utilization estimates, clock-achievability, interrupt budget,
  cross-peripheral conflicts — all deterministic.
- **An AI frontend that's *safe*** because every model proposal is run through
  the same verifier as a hand-written file.

## Install

```bash
cargo install --path .                 # core tool (binary: `forge`)
cargo install --path . --features ai   # also enable the OpenAI-backed `ai` command
```

The crate publishes as `forge-embedded`; the binary is `forge`.

## Commands

```
forge build <config> [--backend c|zephyr|all] [-o dir] [--report] [-v]
forge lint  <config> [--json]        # design review; nonzero exit on errors
forge graph <config>                 # dependency graph as Graphviz DOT
forge check <config>                 # validate only
forge ai    "<intent>" [--from-datasheet f] [-o board.toml] [--build]
```

### `build` — generate code

```bash
forge build board.toml                       # portable C into ./output/
forge build board.toml --backend zephyr      # a .overlay + prj.conf
forge build board.toml --backend all -o gen/ # both, in gen/c and gen/zephyr
```

The C backend emits `init.{c,h}`, `handlers.{c,h}`, and `main.c`, with
`board_init()` ordered by dependency analysis. The Zephyr backend emits a
devicetree `.overlay` and a matching `prj.conf` — targeting the most-complained-
about part of Zephyr (hand-writing devicetree).

### `lint` — deterministic design review

```bash
$ forge lint board.toml
[WARN] spi-clock-mismatch: SPI device 'oled' requested 20 MHz but closest achievable is 10.50 MHz ...
       ↳ Request a speed reachable by a power-of-two prescaler of the system clock.
[INFO] i2c-no-poll-rate: I2C bus 'sensor_bus' has devices but no poll_rate_hz ...
```

`--json` emits structured diagnostics (`severity`, `code`, `message`,
`suggestion`) for CI. Hard errors (pin conflicts, cycles, bad addresses) make it
exit nonzero.

### `ai` — synthesize a config from intent, then verify it

```bash
export OPENAI_API_KEY=...    # never commit this; .env is gitignored
forge ai "drone controller: an IMU on I2C and a GPS on UART" --build
```

The model returns a candidate `board.toml`; Forge runs it through the full gate
(validation + dependency graph + lint) and **refuses to write an invalid
config**. Requires `--features ai`. Honors `OPENAI_MODEL` (default
`gpt-4o-mini`). Without the feature, the rest of Forge works unchanged.

## Describing a board

```toml
[board]
name = "sensor_hub"
mcu = "stm32f4"
clock_mhz = 168

[[gpio]]
pin = 12
mode = "output"
label = "sensor_power"

[[i2c]]
bus = 0
sda_pin = 21
scl_pin = 22
speed_khz = 400
label = "sensor_bus"

[[i2c.device]]
address = 0x68
label = "imu"
depends_on = "i2c_mux"   # initialize the mux first
power_pin = 12           # drive GPIO 12 HIGH before init
poll_rate_hz = 100       # used for bus-utilization analysis
```

Supported peripherals: **GPIO, I2C, SPI, UART.** Dependencies (`depends_on`,
`power_pin`, and implicit bus / chip-select edges) drive a topological sort that
orders `board_init()` into layers and detects cycles. See [`examples/`](examples/)
for `minimal`, `jupiter`, `full`, and `sensor_hub` boards.

## Architecture

```
src/
  model/      IR — the validated Board (single source of truth)
  config/     TOML frontend: parse + validate (the hard gate)
  frontend/ai AI frontend: intent/datasheet → candidate TOML → same gate
  graph/      dependency DAG: topological sort, layering, cycle detection
  lint/       design-review engine: structured diagnostics
  analysis/   the underlying checks + resource summary + report
  backend/    Backend trait + c (scaffolding) + zephyr (devicetree)
  codegen/    pure C renderers used by the C backend
```

Adding a target is one `Backend` impl; adding an input source is one frontend —
the IR, the validation, and the analysis are shared by all of them.

## Development

```bash
cargo test                       # full suite (53 tests)
cargo test --features ai         # include the AI-feature build
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- build examples/sensor_hub.toml --backend all -o /tmp/out -v
```

## License

MIT — see [LICENSE](LICENSE).

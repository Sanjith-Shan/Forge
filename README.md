# Forge

I built Forge after rewriting the same hardware setup code at hackathon after hackathon. Every project started the same way, hand writing I2C, SPI, UART, and GPIO initialization, accidentally assigning a pin twice, and then losing an hour tracking down the mistake.

> Describe an embedded board once. Forge verifies it and generates initialization code for multiple targets.

Forge reads a simple description of a board, which peripherals are wired to which pins, at what settings, and how they depend on each other. It verifies that description deterministically and generates target code (portable C today, Zephyr devicetree today, more later) from a single source of truth.

It works like a compiler. Many frontends feed one verified intermediate representation, and that representation feeds many backends.

```
  Frontends                     Core (deterministic, trusted)        Backends

  board.toml ------\
                    \
  natural language ----> parse  validate  graph  lint  ----> C scaffolding
  or datasheet (AI) /                                  ----> Zephyr devicetree
                   /
  every frontend runs the very same gate before any code is produced
```

The important rule is simple. Whether a config is hand written or proposed by a language model, it runs through the same deterministic verification gate before any code is generated. The AI proposes and the trusted core disposes.

## Why I built it this way

The big vendor tools are locked to one chip family (CubeMX to ST, the NXP tools to NXP), and raw language model code generators are unreliable on the exact details like timing, ordering, and register correctness. Forge sits in the gap between them.

- **Vendor neutral IR to multiple backends.** One description, many targets.
- **System level reasoning that the big tools underserve.** Dependency ordered init, bus utilization estimates, clock achievability, interrupt budget, and cross peripheral conflicts, all computed deterministically.
- **An AI frontend that stays safe** because every model proposal runs through the same verifier as a hand written file.

## Install

```bash
cargo install forge-embedded
cargo install forge-embedded --features ai

# or from a checkout
cargo install --path .
cargo install --path . --features ai
```

The crate publishes as forge-embedded and the installed binary is named forge.

New here? Run `forge init` to write a starter board.toml, then `forge build board.toml`.

## Commands

```
forge init   [path] [--force]                 scaffold a starter board.toml
forge build  <config> [--backend c|zephyr|all] [-o dir] [--report] [-v]
forge lint   <config> [--json]                design review, nonzero exit on errors
forge graph  <config>                         dependency graph as Graphviz DOT
forge check  <config>                         validate only
forge ai     "<intent>" [--from-datasheet f] [-o board.toml] [--build]
forge config <path|show|set|set-key>          manage settings and the API key
forge backends                                list code generation targets
forge completions <bash|zsh|fish>             shell completion script
```

Defaults for the backend and the output directory, plus the AI key and model, can be saved in a config file so you never repeat them. See [Configuration and keys](#configuration-and-keys).

### build

```bash
forge build board.toml
forge build board.toml --backend zephyr
forge build board.toml --backend all -o gen/
```

The C backend emits init.c, init.h, handlers.c, handlers.h, and main.c, with board_init ordered by dependency analysis. The Zephyr backend emits a devicetree overlay and a matching prj.conf, which targets the part of Zephyr that people complain about most, hand writing devicetree.

### lint

Running `forge lint board.toml` performs a deterministic design review and prints findings. Each finding has a severity, a stable code, a message, and a suggested fix. Pass `--json` for structured output suited to CI, with the fields severity, code, message, and suggestion. Hard errors such as pin conflicts, dependency cycles, and bad addresses make it exit nonzero.

### ai

```bash
forge ai "drone controller with an IMU on I2C and a GPS on UART" --build
```

The model returns a candidate board.toml. Forge runs it through the full gate, which is validation, the dependency graph, and lint, and it refuses to write an invalid config. This command requires building with `--features ai`. Without the feature the rest of Forge works unchanged.

## Configuration and keys

Forge needs an OpenAI API key only for `forge ai`. It is resolved in the following order, and the first one found wins, so you can keep secrets out of source control.

1. the `--api-key <key>` flag, handy for one offs
2. the `OPENAI_API_KEY` environment variable
3. a .env file in the working directory containing `OPENAI_API_KEY=...`, which is gitignored
4. the config file, written by `forge config set-key`, which reads stdin so the key never lands in your shell history

```bash
export OPENAI_API_KEY=sk-your-key
echo 'OPENAI_API_KEY=sk-your-key' > .env

forge config set-key < my_key.txt
forge config set model gpt-4o
forge config set backend all
forge config show
forge config path
```

The config file lives at $XDG_CONFIG_HOME/forge/config.toml, or ~/.config/forge/config.toml. It can hold openai_api_key, openai_model, default_backend, and default_output. The OPENAI_MODEL environment variable overrides the model, and the default is gpt-4o-mini.

Never commit an API key. The .env and *.env patterns are gitignored, the config file is written with 0600 permissions, and `forge config show` redacts the key.

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
depends_on = "i2c_mux"
power_pin = 12
poll_rate_hz = 100
```

Supported peripherals are GPIO, I2C, SPI, and UART. Dependencies, which are depends_on, power_pin, and the implicit bus and chip select edges, drive a topological sort that orders board_init into layers and detects cycles. See the examples folder for the minimal, jupiter, full, and sensor_hub boards.

## Architecture

```
src/
  model/      the validated Board IR, the single source of truth
  config/     TOML frontend, parse and validate (the hard gate)
  frontend/ai AI frontend, intent or datasheet to candidate TOML, same gate
  graph/      dependency DAG, topological sort, layering, cycle detection
  lint/       design review engine, structured diagnostics
  analysis/   the underlying checks, resource summary, and report
  backend/    Backend trait, c (scaffolding) and zephyr (devicetree)
  codegen/    pure C renderers used by the C backend
  settings/   user config and API key resolution
```

Adding a target is one Backend impl, and adding an input source is one frontend. The IR, the validation, and the analysis are shared by all of them.

## Development

```bash
cargo test
cargo test --features ai
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo run -- build examples/sensor_hub.toml --backend all -o /tmp/out -v
```

## Releasing

Pushing a version tag triggers the release workflow in .github/workflows, which runs the checks and publishes to crates.io. It needs a repository secret named CARGO_REGISTRY_TOKEN, which is a crates.io API token added under the repository Settings, then Secrets and variables, then Actions.

```bash
git tag v0.1.0
git push origin v0.1.0
```

To publish by hand instead, run `cargo login` and then `cargo publish`.

## License

MIT. See [LICENSE](LICENSE).

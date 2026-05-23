//! Code generation: turn a validated [`Board`] into C source files.
//!
//! Each output file has a pure `render_*` function (easy to snapshot-test) and a
//! matching async `generate_*` function that renders and writes the file. The
//! five files are mutually independent, so the CLI generates them concurrently.

pub mod gpio;
pub mod header;
pub mod i2c;
pub mod main_gen;
pub mod spi;
pub mod uart;

use std::path::{Path, PathBuf};

use crate::error::ForgeError;
use crate::model::Board;

/// The five files Forge emits, in a stable order.
pub const OUTPUT_FILES: [&str; 5] = ["init.h", "init.c", "handlers.h", "handlers.c", "main.c"];

/// Render `init.h`: function declarations for every peripheral plus the master
/// `board_init`.
pub fn render_init_header(board: &Board) -> String {
    let mut s = String::new();
    s.push_str(&header::file_banner(board));
    s.push('\n');
    s.push_str("#ifndef FORGE_INIT_H\n");
    s.push_str("#define FORGE_INIT_H\n");
    s.push('\n');
    s.push_str("#include <stdint.h>\n");

    let sections: Vec<String> = [
        gpio::decl_section(&board.gpios),
        i2c::decl_section(&board.i2c_buses),
        spi::decl_section(&board.spi_buses),
        uart::decl_section(&board.uarts),
    ]
    .into_iter()
    .flatten()
    .collect();

    for section in &sections {
        s.push('\n');
        s.push_str(section);
        s.push('\n');
    }

    s.push('\n');
    s.push_str("/* Master init */\n");
    s.push_str("void board_init(void);\n");
    s.push('\n');
    s.push_str("#endif /* FORGE_INIT_H */\n");
    s
}

/// Render `init.c`: implementations for every peripheral plus `board_init`.
pub fn render_init_source(board: &Board) -> String {
    let mut s = String::new();
    s.push_str("#include \"init.h\"\n");
    s.push_str("#include \"handlers.h\"\n");

    let mut fns: Vec<String> = Vec::new();
    fns.extend(gpio::impl_fns(&board.gpios));
    fns.extend(i2c::impl_fns(&board.i2c_buses));
    fns.extend(spi::impl_fns(&board.spi_buses));
    fns.extend(uart::impl_fns(&board.uarts));
    fns.push(board_init_fn(board));

    for f in &fns {
        s.push('\n');
        s.push_str(f);
        s.push('\n');
    }
    s
}

/// The `board_init` function that calls every peripheral initializer in order.
fn board_init_fn(board: &Board) -> String {
    let mut s = String::from("void board_init(void) {\n");
    if !board.gpios.is_empty() {
        s.push_str("    gpio_init_all();\n");
    }
    for b in &board.i2c_buses {
        s.push_str(&format!("    i2c_{}_init();\n", b.label));
    }
    for b in &board.spi_buses {
        s.push_str(&format!("    spi_{}_init();\n", b.label));
    }
    for u in &board.uarts {
        s.push_str(&format!("    uart_{}_init();\n", u.label));
    }
    s.push('}');
    s
}

/// Render `handlers.h`: declarations for GPIO and UART interrupt handlers.
pub fn render_handlers_header(board: &Board) -> String {
    let mut s = String::new();
    s.push_str("#ifndef FORGE_HANDLERS_H\n");
    s.push_str("#define FORGE_HANDLERS_H\n");

    let sections: Vec<String> = [
        gpio::handler_decl_section(&board.gpios),
        uart::handler_decl_section(&board.uarts),
    ]
    .into_iter()
    .flatten()
    .collect();

    for section in &sections {
        s.push('\n');
        s.push_str(section);
        s.push('\n');
    }

    s.push('\n');
    s.push_str("#endif /* FORGE_HANDLERS_H */\n");
    s
}

/// Render `handlers.c`: TODO-stub implementations for every interrupt handler.
pub fn render_handlers_source(board: &Board) -> String {
    let mut s = String::from("#include \"handlers.h\"\n");

    let mut fns: Vec<String> = Vec::new();
    fns.extend(gpio::handler_impl_fns(&board.gpios));
    fns.extend(uart::handler_impl_fns(&board.uarts));

    for f in &fns {
        s.push('\n');
        s.push_str(f);
        s.push('\n');
    }
    s
}

/// Render `main.c`: the main-loop skeleton.
pub fn render_main(board: &Board) -> String {
    main_gen::render(board)
}

/// Write `contents` to `path`, mapping I/O failures to [`ForgeError::Write`].
async fn write_file(path: &Path, contents: &str) -> Result<(), ForgeError> {
    tokio::fs::write(path, contents)
        .await
        .map_err(|source| ForgeError::Write {
            path: path.to_path_buf(),
            source,
        })
}

/// Render and write `init.h` into `output_dir`.
pub async fn generate_init_header(board: Board, output_dir: PathBuf) -> Result<(), ForgeError> {
    write_file(&output_dir.join("init.h"), &render_init_header(&board)).await
}

/// Render and write `init.c` into `output_dir`.
pub async fn generate_init_source(board: Board, output_dir: PathBuf) -> Result<(), ForgeError> {
    write_file(&output_dir.join("init.c"), &render_init_source(&board)).await
}

/// Render and write `handlers.h` into `output_dir`.
pub async fn generate_handlers_header(board: Board, output_dir: PathBuf) -> Result<(), ForgeError> {
    write_file(
        &output_dir.join("handlers.h"),
        &render_handlers_header(&board),
    )
    .await
}

/// Render and write `handlers.c` into `output_dir`.
pub async fn generate_handlers_source(board: Board, output_dir: PathBuf) -> Result<(), ForgeError> {
    write_file(
        &output_dir.join("handlers.c"),
        &render_handlers_source(&board),
    )
    .await
}

/// Render and write `main.c` into `output_dir`.
pub async fn generate_main(board: Board, output_dir: PathBuf) -> Result<(), ForgeError> {
    write_file(&output_dir.join("main.c"), &render_main(&board)).await
}

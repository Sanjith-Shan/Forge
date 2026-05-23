//! `forge` — generate embedded C boilerplate from a TOML hardware description.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use forge::config;
use forge::error::ForgeError;
use forge::model::Board;

/// Generate embedded C boilerplate from TOML hardware descriptions.
#[derive(Debug, Parser)]
#[command(name = "forge", version, about, long_about = None)]
struct Args {
    /// Path to the board TOML config file.
    config: PathBuf,

    /// Output directory for generated files.
    #[arg(short, long, default_value = "./output")]
    output: PathBuf,

    /// Print detailed generation info.
    #[arg(short, long)]
    verbose: bool,

    /// Parse and validate config without generating files.
    #[arg(long)]
    dry_run: bool,

    /// Validate config only; exit 0 if valid, non-zero otherwise.
    #[arg(long)]
    check: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The full pipeline: parse, validate, then (unless `--check`/`--dry-run`)
/// generate all output files concurrently.
async fn run(args: Args) -> Result<(), ForgeError> {
    let raw = config::parse(&args.config)?;
    let validated = config::validate(raw).map_err(ForgeError::Validation)?;
    let board = validated.board;

    for warning in &validated.warnings {
        eprintln!("warning: {warning}");
    }

    if args.check {
        println!("{}: config is valid.", args.config.display());
        return Ok(());
    }

    if args.verbose || args.dry_run {
        print_summary(&board, &args.output, args.dry_run);
    }

    if args.dry_run {
        return Ok(());
    }

    tokio::fs::create_dir_all(&args.output)
        .await
        .map_err(|source| ForgeError::Write {
            path: args.output.clone(),
            source,
        })?;

    // The five files are independent — generate them concurrently. Each task
    // owns a clone of the board so it can run for `'static`.
    let dir = &args.output;
    let handles = vec![
        tokio::spawn(forge::codegen::generate_init_header(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(forge::codegen::generate_init_source(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(forge::codegen::generate_handlers_header(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(forge::codegen::generate_handlers_source(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(forge::codegen::generate_main(board.clone(), dir.clone())),
    ];

    for handle in handles {
        // Outer `?` unwraps the JoinError; inner `?` the generation error.
        handle.await??;
    }

    println!(
        "Generated {} files for board '{}' in {}",
        forge::codegen::OUTPUT_FILES.len(),
        board.name,
        args.output.display()
    );
    if args.verbose {
        for file in forge::codegen::OUTPUT_FILES {
            println!("  {}", args.output.join(file).display());
        }
    }

    Ok(())
}

/// Print a human-readable summary of what was (or would be) generated.
fn print_summary(board: &Board, output: &std::path::Path, dry_run: bool) {
    let verb = if dry_run {
        "Would generate"
    } else {
        "Generating"
    };
    println!(
        "{verb} code for board '{}' (MCU: {} @ {} MHz)",
        board.name, board.mcu, board.clock_mhz
    );
    println!("  GPIO pins:  {}", board.gpios.len());
    println!(
        "  I2C buses:  {} ({} device(s))",
        board.i2c_buses.len(),
        board
            .i2c_buses
            .iter()
            .map(|b| b.devices.len())
            .sum::<usize>()
    );
    println!(
        "  SPI buses:  {} ({} device(s))",
        board.spi_buses.len(),
        board
            .spi_buses
            .iter()
            .map(|b| b.devices.len())
            .sum::<usize>()
    );
    println!("  UARTs:      {}", board.uarts.len());
    println!("  Output dir: {}", output.display());
}

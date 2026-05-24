//! `forge` — generate embedded C boilerplate from a TOML hardware description.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use forge::error::ForgeError;
use forge::{analysis, codegen, config, graph};

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

    /// Print the dependency graph in Graphviz DOT format and exit.
    #[arg(long)]
    graph: bool,

    /// Write a detailed analysis.txt alongside the generated files.
    #[arg(long)]
    report: bool,

    /// Suppress analysis warnings (the summary still prints).
    #[arg(long)]
    no_warnings: bool,
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

/// The full pipeline: parse, validate, build the dependency graph, analyze,
/// then (unless `--graph`/`--check`/`--dry-run`) generate all output files
/// concurrently in dependency-resolved order.
async fn run(args: Args) -> Result<(), ForgeError> {
    let raw = config::parse(&args.config)?;
    let validated = config::validate(raw).map_err(ForgeError::Validation)?;
    let board = validated.board;

    // Dependency graph construction is itself a validation stage: it catches
    // unknown `depends_on` targets, bad `power_pin`s, and cycles.
    let dep_graph = graph::build(&board).map_err(ForgeError::Validation)?;

    // `--graph` short-circuits: emit DOT and stop.
    if args.graph {
        print!("{}", graph::dot::to_dot(&board, &dep_graph));
        return Ok(());
    }

    let layers = dep_graph
        .layers()
        .map_err(|cycle| ForgeError::Validation(vec![cycle_error(&cycle)]))?;

    // Validation warnings always surface; analysis warnings are opt-out.
    for warning in &validated.warnings {
        eprintln!("warning: {warning}");
    }
    let analysis_warnings = analysis::analyze(&board);
    if !args.no_warnings {
        for warning in &analysis_warnings {
            eprintln!("warning: {warning}");
        }
    }

    let total_warnings = validated.warnings.len() + analysis_warnings.len();
    analysis::print_summary(&board, layers.len(), total_warnings);

    if args.check {
        println!("{}: config is valid.", args.config.display());
        return Ok(());
    }

    if args.dry_run {
        println!("(dry run — no files written)");
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
        tokio::spawn(codegen::generate_init_header(board.clone(), dir.clone())),
        tokio::spawn(codegen::generate_init_source(
            board.clone(),
            layers.clone(),
            dir.clone(),
        )),
        tokio::spawn(codegen::generate_handlers_header(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(codegen::generate_handlers_source(
            board.clone(),
            dir.clone(),
        )),
        tokio::spawn(codegen::generate_main(board.clone(), dir.clone())),
    ];

    for handle in handles {
        // Outer `?` unwraps the JoinError; inner `?` the generation error.
        handle.await??;
    }

    if args.report {
        analysis::write_report(
            &board,
            &dep_graph,
            &layers,
            &analysis_warnings,
            &args.output,
        )
        .await?;
    }

    println!(
        "Generated {} files for board '{}' in {}",
        codegen::OUTPUT_FILES.len(),
        board.name,
        args.output.display()
    );
    if args.verbose {
        for file in codegen::OUTPUT_FILES {
            println!("  {}", args.output.join(file).display());
        }
        if args.report {
            println!("  {}", args.output.join("analysis.txt").display());
        }
    }

    Ok(())
}

/// Build a validation error describing a dependency cycle (defensive: the graph
/// builder normally catches cycles first).
fn cycle_error(cycle: &[String]) -> forge::error::ValidationError {
    forge::error::ValidationError::new(format!(
        "Circular dependency detected in initialization order: {}",
        cycle.join(" -> ")
    ))
}

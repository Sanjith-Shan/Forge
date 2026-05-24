//! `forge` — a vendor-neutral hardware bring-up compiler.
//!
//! Subcommands:
//! - `build` — generate target code (C, Zephyr devicetree, …) from a config
//! - `lint`  — run the deterministic design review
//! - `graph` — print the dependency graph as Graphviz DOT
//! - `check` — validate a config only
//! - `ai`    — synthesize a config from natural language, then verify it

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use forge::backend::{self, InitPlan};
use forge::error::{ForgeError, ValidationError};
use forge::graph::{self, DepGraph};
use forge::lint;
use forge::model::Board;
use forge::{analysis, config};

#[derive(Debug, Parser)]
#[command(name = "forge", version, about = "Vendor-neutral hardware bring-up compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate target code from a board config.
    Build(BuildArgs),
    /// Run the design-review lint over a board config.
    Lint(LintArgs),
    /// Print the dependency graph in Graphviz DOT format.
    Graph(GraphArgs),
    /// Validate a board config only; exit 0 if valid.
    Check(CheckArgs),
    /// Synthesize a board config from natural language, then verify it.
    Ai(AiArgs),
}

#[derive(Debug, Parser)]
struct BuildArgs {
    /// Path to the board TOML config file.
    config: PathBuf,
    /// Output directory for generated files.
    #[arg(short, long, default_value = "./output")]
    output: PathBuf,
    /// Target backend(s): `c`, `zephyr`, or `all`.
    #[arg(long, default_value = "c")]
    backend: String,
    /// Print the paths of every generated file.
    #[arg(short, long)]
    verbose: bool,
    /// Write a detailed analysis.txt alongside the output.
    #[arg(long)]
    report: bool,
    /// Suppress analysis warnings (the summary still prints).
    #[arg(long)]
    no_warnings: bool,
}

#[derive(Debug, Parser)]
struct LintArgs {
    /// Path to the board TOML config file.
    config: PathBuf,
    /// Emit diagnostics as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct GraphArgs {
    /// Path to the board TOML config file.
    config: PathBuf,
}

#[derive(Debug, Parser)]
struct CheckArgs {
    /// Path to the board TOML config file.
    config: PathBuf,
}

#[derive(Debug, Parser)]
struct AiArgs {
    /// Natural-language description of the board to build.
    #[arg(required = true, num_args = 1..)]
    intent: Vec<String>,
    /// Ground the model with a datasheet text file.
    #[arg(long)]
    from_datasheet: Option<PathBuf>,
    /// Where to write the synthesized config.
    #[arg(short, long, default_value = "board.toml")]
    output: PathBuf,
    /// After a valid synthesis, also generate code.
    #[arg(long)]
    build: bool,
    /// Target backend(s) when --build is set.
    #[arg(long, default_value = "c")]
    backend: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Build(args) => cmd_build(args).await,
        Command::Lint(args) => cmd_lint(args).await,
        Command::Graph(args) => cmd_graph(args).await,
        Command::Check(args) => cmd_check(args).await,
        Command::Ai(args) => cmd_ai(args).await,
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Parse + validate + build the dependency graph, or fail with all errors.
fn load_board(path: &Path) -> Result<(Board, DepGraph, Vec<String>), ForgeError> {
    let raw = config::parse(path)?;
    let validated = config::validate(raw).map_err(ForgeError::Validation)?;
    let graph = graph::build(&validated.board).map_err(ForgeError::Validation)?;
    Ok((validated.board, graph, validated.warnings))
}

/// Expand a `--backend` argument into validated backend ids.
fn resolve_backends(arg: &str) -> Result<Vec<String>, ForgeError> {
    if arg == "all" {
        return Ok(backend::BACKEND_IDS.iter().map(|s| s.to_string()).collect());
    }
    let ids: Vec<String> = arg.split(',').map(|s| s.trim().to_string()).collect();
    for id in &ids {
        if backend::by_id(id).is_none() {
            return Err(ForgeError::Usage(format!(
                "unknown backend '{id}' (choose from: {}, or 'all')",
                backend::BACKEND_IDS.join(", ")
            )));
        }
    }
    Ok(ids)
}

/// Generate every requested backend's files into `output`, returning the paths
/// written. With more than one backend, each gets its own subdirectory.
async fn generate(
    board: &Board,
    graph: &DepGraph,
    output: &Path,
    backend_ids: &[String],
    report: bool,
) -> Result<Vec<PathBuf>, ForgeError> {
    let layers = graph
        .layers()
        .map_err(|cycle| ForgeError::Validation(vec![cycle_error(&cycle)]))?;
    let plan = InitPlan {
        layers: layers.clone(),
    };

    let multi = backend_ids.len() > 1;
    let mut written = Vec::new();
    for id in backend_ids {
        let backend = backend::by_id(id).expect("backend id was validated");
        let files = backend.render(board, &plan);
        let dir = if multi {
            output.join(id)
        } else {
            output.to_path_buf()
        };
        let mut paths: Vec<PathBuf> = files.iter().map(|f| dir.join(&f.path)).collect();
        backend::write_files(files, &dir).await?;
        written.append(&mut paths);
    }

    if report {
        let warnings = analysis::analyze(board);
        analysis::write_report(board, graph, &layers, &warnings, output).await?;
        written.push(output.join("analysis.txt"));
    }
    Ok(written)
}

async fn cmd_build(args: BuildArgs) -> Result<ExitCode, ForgeError> {
    let backend_ids = resolve_backends(&args.backend)?;
    let (board, graph, validation_warnings) = load_board(&args.config)?;
    let layer_count = graph
        .layers()
        .map_err(|cycle| ForgeError::Validation(vec![cycle_error(&cycle)]))?
        .len();

    for warning in &validation_warnings {
        eprintln!("warning: {warning}");
    }
    let analysis_warnings = analysis::analyze(&board);
    if !args.no_warnings {
        for warning in &analysis_warnings {
            eprintln!("warning: {warning}");
        }
    }
    analysis::print_summary(
        &board,
        layer_count,
        validation_warnings.len() + analysis_warnings.len(),
    );

    let written = generate(&board, &graph, &args.output, &backend_ids, args.report).await?;

    println!(
        "Generated {} file(s) for board '{}' [{}] in {}",
        written.len(),
        board.name,
        backend_ids.join(", "),
        args.output.display()
    );
    if args.verbose {
        for path in &written {
            println!("  {}", path.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn cmd_lint(args: LintArgs) -> Result<ExitCode, ForgeError> {
    // Lint reports problems rather than aborting, so it collects hard errors as
    // diagnostics instead of bubbling them up.
    let raw = config::parse(&args.config)?;
    let mut diagnostics = Vec::new();

    match config::validate(raw) {
        Err(errors) => {
            diagnostics.extend(errors.into_iter().map(validation_to_diag));
        }
        Ok(validated) => {
            for w in validated.warnings {
                diagnostics.push(lint::Diagnostic::warning("config-warning", w));
            }
            match graph::build(&validated.board) {
                Err(errors) => diagnostics.extend(errors.into_iter().map(validation_to_diag)),
                Ok(_) => diagnostics.extend(lint::review(&validated.board)),
            }
        }
    }

    if args.json {
        println!("{}", lint::render_json(&diagnostics));
    } else {
        print!("{}", lint::render_human(&diagnostics));
    }

    Ok(if lint::has_errors(&diagnostics) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

async fn cmd_graph(args: GraphArgs) -> Result<ExitCode, ForgeError> {
    let (board, graph, _) = load_board(&args.config)?;
    print!("{}", graph::dot::to_dot(&board, &graph));
    Ok(ExitCode::SUCCESS)
}

async fn cmd_check(args: CheckArgs) -> Result<ExitCode, ForgeError> {
    let (board, graph, warnings) = load_board(&args.config)?;
    for warning in &warnings {
        eprintln!("warning: {warning}");
    }
    let layer_count = graph.layers().map(|l| l.len()).unwrap_or(0);
    analysis::print_summary(&board, layer_count, warnings.len());
    println!("{}: config is valid.", args.config.display());
    Ok(ExitCode::SUCCESS)
}

async fn cmd_ai(args: AiArgs) -> Result<ExitCode, ForgeError> {
    let datasheet = match &args.from_datasheet {
        Some(path) => Some(
            std::fs::read_to_string(path).map_err(|source| ForgeError::Io {
                path: path.clone(),
                source,
            })?,
        ),
        None => None,
    };
    let request = forge::frontend::ai::AiRequest {
        intent: args.intent.join(" "),
        datasheet,
    };

    let outcome = run_ai(&request).await?;

    println!("--- proposed board.toml ---");
    println!("{}", outcome.candidate_toml.trim_end());
    println!("---------------------------");

    if !outcome.is_valid() {
        eprintln!("\nThe AI proposal did NOT pass verification:");
        for err in &outcome.errors {
            eprintln!("  - {}", err.message);
        }
        eprintln!("\nNot writing an invalid config. Re-run to try again.");
        return Ok(ExitCode::FAILURE);
    }

    tokio::fs::write(&args.output, &outcome.candidate_toml)
        .await
        .map_err(|source| ForgeError::Write {
            path: args.output.clone(),
            source,
        })?;
    println!("\nVerified ✓  wrote {}", args.output.display());

    if !outcome.diagnostics.is_empty() {
        println!("\nDesign review:");
        print!("{}", lint::render_human(&outcome.diagnostics));
    }

    if args.build {
        let board = outcome.board.expect("valid outcome has a board");
        let graph = graph::build(&board).map_err(ForgeError::Validation)?;
        let backend_ids = resolve_backends(&args.backend)?;
        let output = PathBuf::from("./output");
        let written = generate(&board, &graph, &output, &backend_ids, false).await?;
        println!(
            "\nGenerated {} file(s) [{}] in {}",
            written.len(),
            backend_ids.join(", "),
            output.display()
        );
    }

    Ok(ExitCode::SUCCESS)
}

/// Run the AI frontend. With the `ai` feature this uses OpenAI; without it the
/// command is unavailable.
#[cfg(feature = "ai")]
async fn run_ai(
    request: &forge::frontend::ai::AiRequest,
) -> Result<forge::frontend::ai::AiOutcome, ForgeError> {
    let provider = forge::frontend::ai::openai::OpenAiProvider::from_env()?;
    forge::frontend::ai::synthesize(&provider, request).await
}

#[cfg(not(feature = "ai"))]
async fn run_ai(
    _request: &forge::frontend::ai::AiRequest,
) -> Result<forge::frontend::ai::AiOutcome, ForgeError> {
    Err(ForgeError::Usage(
        "the `ai` command requires building with `--features ai` and setting OPENAI_API_KEY"
            .to_string(),
    ))
}

/// Convert a hard validation error into an error-severity diagnostic.
fn validation_to_diag(err: ValidationError) -> lint::Diagnostic {
    lint::Diagnostic::error("invalid-config", err.message)
}

/// Build a validation error describing a dependency cycle (defensive: the graph
/// builder normally catches cycles first).
fn cycle_error(cycle: &[String]) -> ValidationError {
    ValidationError::new(format!(
        "Circular dependency detected in initialization order: {}",
        cycle.join(" -> ")
    ))
}

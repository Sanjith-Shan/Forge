//! `forge` — a vendor-neutral hardware bring-up compiler.
//!
//! Run `forge --help` for the full command list. The pipeline is: a frontend
//! (TOML, or the AI synthesizer) produces a board description, the deterministic
//! core validates and analyzes it, and a backend renders target code.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use forge::backend::{self, InitPlan};
use forge::error::{ForgeError, ValidationError};
use forge::graph::{self, DepGraph};
use forge::model::Board;
use forge::{analysis, config, lint, settings};

/// A starter board description written by `forge init`.
const INIT_TEMPLATE: &str = include_str!("templates/board.toml");

#[derive(Debug, Parser)]
#[command(
    name = "forge",
    version,
    about = "Vendor-neutral hardware bring-up compiler",
    long_about = "Forge turns a TOML (or AI-synthesized) description of an embedded board into \
                  verified, dependency-ordered initialization code for multiple targets.",
    propagate_version = true
)]
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
    /// Write a starter board.toml.
    Init(InitArgs),
    /// Inspect and edit Forge's user settings.
    Config(ConfigCmd),
    /// List the available code-generation backends.
    Backends,
    /// Print a shell completion script.
    Completions(CompletionsArgs),
}

#[derive(Debug, Parser)]
struct BuildArgs {
    /// Path to the board TOML config file.
    config: PathBuf,
    /// Output directory [default: ./output, or config `default_output`].
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Target backend(s): `c`, `zephyr`, or `all` [default: c, or config].
    #[arg(long)]
    backend: Option<String>,
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
    #[arg(long)]
    backend: Option<String>,
    /// OpenAI API key (prefer OPENAI_API_KEY env or `forge config set-key`).
    #[arg(long)]
    api_key: Option<String>,
    /// OpenAI model id [default: gpt-4o-mini, or config].
    #[arg(long)]
    model: Option<String>,
}

#[derive(Debug, Parser)]
struct InitArgs {
    /// Where to write the starter config.
    #[arg(default_value = "board.toml")]
    path: PathBuf,
    /// Overwrite an existing file.
    #[arg(short, long)]
    force: bool,
}

#[derive(Debug, Parser)]
struct ConfigCmd {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    /// Print the path to the config file.
    Path,
    /// Show the current settings (secrets redacted).
    Show,
    /// Set a non-secret field: model | backend | output.
    Set {
        /// Field name: `model`, `backend`, or `output`.
        field: String,
        /// New value.
        value: String,
    },
    /// Store the OpenAI API key, read from stdin (keeps it out of shell history).
    SetKey,
}

#[derive(Debug, Parser)]
struct CompletionsArgs {
    /// Shell to generate completions for.
    shell: Shell,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Load OPENAI_API_KEY (and friends) from a gitignored .env if present.
    #[cfg(feature = "ai")]
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let result = match cli.command {
        Command::Build(args) => cmd_build(args).await,
        Command::Lint(args) => cmd_lint(args).await,
        Command::Graph(args) => cmd_graph(args).await,
        Command::Check(args) => cmd_check(args).await,
        Command::Ai(args) => cmd_ai(args).await,
        Command::Init(args) => cmd_init(args),
        Command::Config(cmd) => cmd_config(cmd),
        Command::Backends => cmd_backends(),
        Command::Completions(args) => cmd_completions(args),
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

// ---- shared pipeline helpers ------------------------------------------------

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

// ---- commands ---------------------------------------------------------------

async fn cmd_build(args: BuildArgs) -> Result<ExitCode, ForgeError> {
    let settings = settings::load();
    let backend_str = settings::resolve_backend(args.backend.as_deref(), &settings);
    let backend_ids = resolve_backends(&backend_str)?;
    let output = PathBuf::from(settings::resolve_output(
        args.output.as_ref().and_then(|p| p.to_str()),
        &settings,
    ));

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

    let written = generate(&board, &graph, &output, &backend_ids, args.report).await?;

    println!(
        "Generated {} file(s) for board '{}' [{}] in {}",
        written.len(),
        board.name,
        backend_ids.join(", "),
        output.display()
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
        Err(errors) => diagnostics.extend(errors.into_iter().map(validation_to_diag)),
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
    let settings = settings::load();
    let api_key = settings::resolve_api_key(
        args.api_key.as_deref(),
        std::env::var("OPENAI_API_KEY").ok().as_deref(),
        &settings,
    );
    let model = settings::resolve_model(
        args.model.as_deref(),
        std::env::var("OPENAI_MODEL").ok().as_deref(),
        &settings,
    );

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

    let outcome = run_ai(&request, api_key, model).await?;

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
        let backend_str = settings::resolve_backend(args.backend.as_deref(), &settings);
        let backend_ids = resolve_backends(&backend_str)?;
        let output = PathBuf::from(settings::resolve_output(None, &settings));
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

fn cmd_init(args: InitArgs) -> Result<ExitCode, ForgeError> {
    if args.path.exists() && !args.force {
        return Err(ForgeError::Usage(format!(
            "{} already exists (use --force to overwrite)",
            args.path.display()
        )));
    }
    std::fs::write(&args.path, INIT_TEMPLATE).map_err(|source| ForgeError::Write {
        path: args.path.clone(),
        source,
    })?;
    println!("Wrote starter config to {}", args.path.display());
    println!(
        "Next: `forge check {0}`, then `forge build {0}`.",
        args.path.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn cmd_config(cmd: ConfigCmd) -> Result<ExitCode, ForgeError> {
    match cmd.action {
        ConfigAction::Path => {
            println!("{}", settings::config_path().display());
        }
        ConfigAction::Show => {
            let s = settings::load();
            println!("config file: {}", settings::config_path().display());
            println!(
                "openai_api_key: {}",
                match &s.openai_api_key {
                    Some(k) => settings::redact(k),
                    None => "(not set in config)".to_string(),
                }
            );
            let env_key = std::env::var("OPENAI_API_KEY")
                .map(|_| "set")
                .unwrap_or("not set");
            println!("OPENAI_API_KEY env: {env_key}");
            println!(
                "openai_model: {}",
                s.openai_model.as_deref().unwrap_or(settings::DEFAULT_MODEL)
            );
            println!(
                "default_backend: {}",
                s.default_backend
                    .as_deref()
                    .unwrap_or(settings::DEFAULT_BACKEND)
            );
            println!(
                "default_output: {}",
                s.default_output
                    .as_deref()
                    .unwrap_or(settings::DEFAULT_OUTPUT)
            );
        }
        ConfigAction::Set { field, value } => {
            let mut s = settings::load();
            match field.as_str() {
                "model" | "openai_model" => s.openai_model = Some(value),
                "backend" | "default_backend" => s.default_backend = Some(value),
                "output" | "default_output" => s.default_output = Some(value),
                "key" | "api_key" | "openai_api_key" => {
                    return Err(ForgeError::Usage(
                        "set the API key with `forge config set-key` (reads stdin) so it stays \
                         out of shell history"
                            .to_string(),
                    ));
                }
                other => {
                    return Err(ForgeError::Usage(format!(
                        "unknown field '{other}' (choose: model, backend, output)"
                    )));
                }
            }
            settings::save(&s)?;
            println!("Updated {field} in {}", settings::config_path().display());
        }
        ConfigAction::SetKey => {
            let mut key = String::new();
            std::io::stdin()
                .read_to_string(&mut key)
                .map_err(|e| ForgeError::Usage(format!("could not read stdin: {e}")))?;
            let key = key.trim().to_string();
            if key.is_empty() {
                return Err(ForgeError::Usage(
                    "no key provided on stdin (try: `forge config set-key < keyfile`)".to_string(),
                ));
            }
            let mut s = settings::load();
            let shown = settings::redact(&key);
            s.openai_api_key = Some(key);
            settings::save(&s)?;
            println!(
                "Saved API key ({shown}) to {}",
                settings::config_path().display()
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_backends() -> Result<ExitCode, ForgeError> {
    println!("Available backends:");
    for id in backend::BACKEND_IDS {
        let description = backend::by_id(id)
            .map(|b| b.description().to_string())
            .unwrap_or_default();
        println!("  {id:<8} {description}");
    }
    println!(
        "  {:<8} Every backend (into per-backend subdirectories)",
        "all"
    );
    Ok(ExitCode::SUCCESS)
}

fn cmd_completions(args: CompletionsArgs) -> Result<ExitCode, ForgeError> {
    let mut cmd = Cli::command();
    clap_complete::generate(args.shell, &mut cmd, "forge", &mut std::io::stdout());
    Ok(ExitCode::SUCCESS)
}

// ---- AI runner (feature-gated) ----------------------------------------------

#[cfg(feature = "ai")]
async fn run_ai(
    request: &forge::frontend::ai::AiRequest,
    api_key: Option<String>,
    model: String,
) -> Result<forge::frontend::ai::AiOutcome, ForgeError> {
    let api_key = api_key.ok_or_else(|| {
        ForgeError::Ai(
            "no OpenAI API key found. Provide one of:\n  \
             • OPENAI_API_KEY in your environment\n  \
             • a .env file containing OPENAI_API_KEY=...\n  \
             • `forge config set-key` (reads stdin)\n  \
             • the --api-key flag"
                .to_string(),
        )
    })?;
    let provider = forge::frontend::ai::openai::OpenAiProvider::new(api_key, model);
    forge::frontend::ai::synthesize(&provider, request).await
}

#[cfg(not(feature = "ai"))]
async fn run_ai(
    _request: &forge::frontend::ai::AiRequest,
    _api_key: Option<String>,
    _model: String,
) -> Result<forge::frontend::ai::AiOutcome, ForgeError> {
    Err(ForgeError::Usage(
        "the `ai` command requires building with `--features ai` (and an OpenAI API key)"
            .to_string(),
    ))
}

// ---- small helpers ----------------------------------------------------------

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

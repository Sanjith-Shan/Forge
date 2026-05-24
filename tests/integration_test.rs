//! End-to-end tests that exercise the compiled `forge` binary as a subprocess.

use std::path::PathBuf;
use std::process::Command;

/// Path to a fixture under `tests/fixtures/`.
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Path to the `forge` binary built for this test run.
fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forge")
}

/// Path to a file under the crate's `examples/` directory.
fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
fn generates_all_five_files_for_jupiter() {
    let out = tempfile::tempdir().expect("create temp dir");
    let status = Command::new(forge_bin())
        .arg(example("jupiter.toml"))
        .arg("-o")
        .arg(out.path())
        .status()
        .expect("run forge");
    assert!(status.success(), "forge should exit 0 on a valid config");

    for file in ["init.h", "init.c", "handlers.h", "handlers.c", "main.c"] {
        let path = out.path().join(file);
        assert!(path.exists(), "expected generated file {file} to exist");
    }
}

#[test]
fn check_flag_passes_on_valid_config() {
    let status = Command::new(forge_bin())
        .arg(example("minimal.toml"))
        .arg("--check")
        .status()
        .expect("run forge");
    assert!(status.success(), "--check should exit 0 on a valid config");
}

#[test]
fn check_flag_fails_on_invalid_config() {
    let status = Command::new(forge_bin())
        .arg(fixture("invalid_pin_conflict.toml"))
        .arg("--check")
        .status()
        .expect("run forge");
    assert!(
        !status.success(),
        "--check should exit non-zero on an invalid config"
    );
}

#[test]
fn check_flag_fails_on_dependency_cycle() {
    let status = Command::new(forge_bin())
        .arg(fixture("invalid_cycle.toml"))
        .arg("--check")
        .status()
        .expect("run forge");
    assert!(
        !status.success(),
        "--check should exit non-zero on a dependency cycle"
    );
}

#[test]
fn graph_flag_emits_dot_without_generating() {
    let out = tempfile::tempdir().expect("create temp dir");
    let result = Command::new(forge_bin())
        .arg(example("sensor_hub.toml"))
        .arg("-o")
        .arg(out.path())
        .arg("--graph")
        .output()
        .expect("run forge");
    assert!(result.status.success());

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(stdout.contains("digraph board_init"), "got: {stdout}");
    assert!(stdout.contains("rankdir=BT;"));
    // --graph must not generate any files.
    assert!(!out.path().join("init.c").exists());
}

#[test]
fn report_flag_writes_analysis_file() {
    let out = tempfile::tempdir().expect("create temp dir");
    let status = Command::new(forge_bin())
        .arg(example("sensor_hub.toml"))
        .arg("-o")
        .arg(out.path())
        .arg("--report")
        .status()
        .expect("run forge");
    assert!(status.success());

    let analysis = out.path().join("analysis.txt");
    assert!(analysis.exists(), "analysis.txt should be written");
    let contents = std::fs::read_to_string(&analysis).expect("read analysis.txt");
    assert!(contents.contains("DEPENDENCY GRAPH"));
    assert!(contents.contains("INIT ORDER"));
}

#[test]
fn dry_run_does_not_write_files() {
    let out = tempfile::tempdir().expect("create temp dir");
    let nested = out.path().join("generated");
    let status = Command::new(forge_bin())
        .arg(example("full.toml"))
        .arg("-o")
        .arg(&nested)
        .arg("--dry-run")
        .status()
        .expect("run forge");
    assert!(
        status.success(),
        "--dry-run should exit 0 on a valid config"
    );
    assert!(
        !nested.exists(),
        "--dry-run must not create the output directory"
    );
}

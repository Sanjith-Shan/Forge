//! End-to-end tests that exercise the compiled `forge` binary as a subprocess.

use std::path::PathBuf;
use std::process::Command;

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
    let bad =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/invalid_pin_conflict.toml");
    let status = Command::new(forge_bin())
        .arg(bad)
        .arg("--check")
        .status()
        .expect("run forge");
    assert!(
        !status.success(),
        "--check should exit non-zero on an invalid config"
    );
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

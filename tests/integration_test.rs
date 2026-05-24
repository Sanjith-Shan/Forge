//! End-to-end tests that drive the compiled `forge` binary as a subprocess.

use std::path::PathBuf;
use std::process::Command;

fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forge")
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn build_generates_c_files() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(forge_bin())
        .args(["build"])
        .arg(example("jupiter.toml"))
        .arg("-o")
        .arg(out.path())
        .status()
        .expect("run forge");
    assert!(status.success());
    for file in ["init.h", "init.c", "handlers.h", "handlers.c", "main.c"] {
        assert!(out.path().join(file).exists(), "missing {file}");
    }
}

#[test]
fn build_zephyr_backend_emits_overlay() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(forge_bin())
        .args(["build"])
        .arg(example("sensor_hub.toml"))
        .arg("-o")
        .arg(out.path())
        .args(["--backend", "zephyr"])
        .status()
        .expect("run forge");
    assert!(status.success());
    assert!(out.path().join("sensor_hub.overlay").exists());
    assert!(out.path().join("prj.conf").exists());
}

#[test]
fn build_all_backends_uses_subdirs() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(forge_bin())
        .args(["build"])
        .arg(example("sensor_hub.toml"))
        .arg("-o")
        .arg(out.path())
        .args(["--backend", "all"])
        .status()
        .expect("run forge");
    assert!(status.success());
    assert!(out.path().join("c/init.c").exists());
    assert!(out.path().join("zephyr/sensor_hub.overlay").exists());
}

#[test]
fn build_rejects_unknown_backend() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(forge_bin())
        .args(["build"])
        .arg(example("minimal.toml"))
        .arg("-o")
        .arg(out.path())
        .args(["--backend", "rust"])
        .status()
        .expect("run forge");
    assert!(!status.success(), "unknown backend should fail");
}

#[test]
fn check_passes_and_fails_appropriately() {
    let ok = Command::new(forge_bin())
        .args(["check"])
        .arg(example("minimal.toml"))
        .status()
        .unwrap();
    assert!(ok.success());

    let bad = Command::new(forge_bin())
        .args(["check"])
        .arg(fixture("invalid_pin_conflict.toml"))
        .status()
        .unwrap();
    assert!(!bad.success());

    let cycle = Command::new(forge_bin())
        .args(["check"])
        .arg(fixture("invalid_cycle.toml"))
        .status()
        .unwrap();
    assert!(!cycle.success());
}

#[test]
fn graph_emits_dot() {
    let out = Command::new(forge_bin())
        .args(["graph"])
        .arg(example("sensor_hub.toml"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("digraph board_init"));
    assert!(stdout.contains("rankdir=BT;"));
}

#[test]
fn lint_json_is_valid_json() {
    let out = Command::new(forge_bin())
        .args(["lint"])
        .arg(example("sensor_hub.toml"))
        .arg("--json")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Must parse as a JSON array.
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    assert!(parsed.is_array());
}

#[test]
fn lint_fails_on_invalid_config() {
    let out = Command::new(forge_bin())
        .args(["lint"])
        .arg(fixture("invalid_pin_conflict.toml"))
        .status()
        .unwrap();
    assert!(!out.success(), "lint should exit nonzero when errors exist");
}

#[test]
fn init_writes_a_config_that_checks_clean() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("board.toml");

    let init = Command::new(forge_bin())
        .args(["init"])
        .arg(&cfg)
        .status()
        .unwrap();
    assert!(init.success());
    assert!(cfg.exists());

    // The freshly-scaffolded config must pass `check`.
    let check = Command::new(forge_bin())
        .args(["check"])
        .arg(&cfg)
        .status()
        .unwrap();
    assert!(check.success(), "init template should check clean");
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("board.toml");
    std::fs::write(&cfg, "existing").unwrap();

    let status = Command::new(forge_bin())
        .args(["init"])
        .arg(&cfg)
        .status()
        .unwrap();
    assert!(!status.success(), "must not clobber an existing file");
}

#[test]
fn backends_lists_known_targets() {
    let out = Command::new(forge_bin()).arg("backends").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("c"));
    assert!(stdout.contains("zephyr"));
}

#[test]
fn completions_emit_a_script() {
    let out = Command::new(forge_bin())
        .args(["completions", "bash"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("forge"),
        "completion script mentions the binary"
    );
}

#[test]
fn report_flag_writes_analysis_file() {
    let out = tempfile::tempdir().unwrap();
    let status = Command::new(forge_bin())
        .args(["build"])
        .arg(example("sensor_hub.toml"))
        .arg("-o")
        .arg(out.path())
        .arg("--report")
        .status()
        .unwrap();
    assert!(status.success());
    let analysis = out.path().join("analysis.txt");
    assert!(analysis.exists());
    let contents = std::fs::read_to_string(&analysis).unwrap();
    assert!(contents.contains("DEPENDENCY GRAPH"));
    assert!(contents.contains("INIT ORDER"));
}

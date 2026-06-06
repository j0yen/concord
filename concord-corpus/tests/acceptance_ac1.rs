//! AC1: `cargo build` and `cargo test` green; `concord --help` lists the `corpus` subcommand.

use std::process::Command;

#[test]
fn concord_help_lists_corpus_subcommand() {
    // Build the binary first (in test context it should already be built).
    let output = Command::new(env!("CARGO_BIN_EXE_concord"))
        .arg("--help")
        .output()
        .expect("failed to run concord --help");

    assert!(
        output.status.success(),
        "concord --help should exit 0, got: {}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    assert!(
        combined.contains("corpus"),
        "concord --help output should mention 'corpus', got:\n{combined}"
    );
}

#[test]
fn corpus_build_help_exits_ok() {
    let output = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args(["corpus", "build", "--help"])
        .output()
        .expect("failed to run concord corpus build --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");
    assert!(
        output.status.success(),
        "concord corpus build --help should exit 0, got: {}\n{combined}",
        output.status
    );
}

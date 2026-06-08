//! AC5: `concord run "<claim>" --fixtures <dir>` integration test.
//! Verifies end-to-end pipeline: corpus → steelman → cruxes → bridge.
//! Also AC1 for bridge/run --help.

use std::process::Command;
use tempfile::TempDir;

// AC1 (bridge): `concord bridge --help` exits 0.
#[test]
fn concord_bridge_help_exits_ok() {
    let output = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args(["bridge", "--help"])
        .output()
        .expect("failed to run concord bridge --help");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "concord bridge --help should exit 0\n{combined}"
    );
}

// AC1 (run): `concord run --help` exits 0.
#[test]
fn concord_run_help_exits_ok() {
    let output = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args(["run", "--help"])
        .output()
        .expect("failed to run concord run --help");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "concord run --help should exit 0\n{combined}"
    );
}

// AC5: full pipeline integration — concord run produces brief.md with all four sections.
#[test]
fn integration_run_produces_brief_with_all_sections() {
    let fixtures_dir = {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.join("tests/fixtures/basic_claim")
    };

    let out_tmp = TempDir::new().expect("tempdir");
    let out_path = out_tmp.path().join("brief.md");

    let status = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args([
            "run",
            "Does coffee reduce diabetes risk?",
            "--fixtures",
            fixtures_dir.to_str().expect("fixtures path"),
            "--out",
            out_path.to_str().expect("out path"),
        ])
        .status()
        .expect("failed to run concord run");

    assert!(status.success(), "concord run should exit 0, got: {status}");
    assert!(out_path.exists(), "brief.md should be created");

    let md = std::fs::read_to_string(&out_path).expect("read brief.md");
    assert!(!md.is_empty(), "brief.md should not be empty");
    assert!(md.contains("## What Each Side Wants"), "missing WESAS");
    assert!(md.contains("## Where the Disagreement Actually Is"), "missing disagreement");
    assert!(md.contains("## Common Ground"), "missing common ground");
    assert!(md.contains("## What Would Change Minds"), "missing change minds");
    assert!(md.contains("## References"), "missing references");
}

// Sentinel: confirms this integration test file actually runs (per self_orphaned_mock_tests note).
#[test]
fn this_integration_file_is_wired() {
    assert!(true, "acceptance_bridge_run.rs is executing");
}

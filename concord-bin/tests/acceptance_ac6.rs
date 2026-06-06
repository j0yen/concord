//! AC6: `main()` calls `sigpipe::reset()` first; `concord corpus show <file> | head`
//! does not panic.

use std::process::{Command, Stdio};
use tempfile::TempDir;

fn build_sample_corpus(dir: &std::path::Path) -> std::path::PathBuf {
    let fixtures_dir = {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest_dir.join("tests/fixtures/basic_claim")
    };
    let out_path = dir.join("corpus.json");
    let status = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args([
            "corpus",
            "build",
            "Does coffee reduce diabetes risk?",
            "--fixtures",
            fixtures_dir.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("build sample corpus");
    assert!(status.success(), "sample corpus build should succeed");
    out_path
}

#[test]
fn corpus_show_piped_through_head_does_not_panic() {
    let tmp = TempDir::new().expect("tempdir");
    let corpus_path = build_sample_corpus(tmp.path());

    // Simulate `concord corpus show <file> | head -n 3` by reading only
    // 3 lines from stdout (closes the pipe early → SIGPIPE on the writer side
    // without sigpipe::reset()).
    let mut concord = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args(["corpus", "show", corpus_path.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn concord corpus show");

    // Read a few bytes then drop the pipe.
    if let Some(stdout) = concord.stdout.take() {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        let _: Vec<_> = reader.lines().take(3).collect();
        // Drop reader → pipe closed → SIGPIPE delivered to concord.
    }

    let status = concord.wait().expect("wait for concord");
    // With sigpipe::reset(), the process exits cleanly (0 or SIGPIPE signal
    // depending on OS, but NOT with exit code 101 which is a Rust panic).
    // On Linux SIGPIPE terminates with signal 13; status.code() will be None.
    // What we assert: NOT a panic exit code (101).
    assert_ne!(
        status.code(),
        Some(101),
        "concord must not panic (exit 101) when stdout pipe is broken"
    );
}

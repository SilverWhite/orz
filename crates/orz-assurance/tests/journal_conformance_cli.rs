//! End-to-end tests for the `journal-conformance` CLI (Task D S3,
//! 2026-09-06): the binary the repository gate invokes per run-event fixture
//! journal must accept every real fixture and reject a tampered copy, and
//! must fail closed on usage errors. The judge library itself is covered by
//! `fixture_journal_conformance.rs` and the family crosscheck — these tests
//! pin the CLI shell (argument wiring, exit codes, error surfacing).

use std::path::Path;
use std::process::Command;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn run_cli(journal: &Path, repo_root: &Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_journal-conformance"))
        .arg(journal)
        .arg("--repo-root")
        .arg(&repo_root)
        .output()
        .expect("spawn journal-conformance");
    (
        output.status.code().expect("exit code"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn cli_accepts_real_fixtures_from_both_tracks() {
    let repo_root = repo_root();
    for fixture in [
        "runtime/fixtures/run-event-v0.1/journals/plain-run.jsonl",
        "runtime/fixtures/run-event-v0.2/journals/orientation-fire-run.jsonl",
    ] {
        let (code, stdout, stderr) = run_cli(&repo_root.join(fixture), &repo_root);
        assert_eq!(code, 0, "{fixture} must pass: {stderr}");
        assert!(stdout.starts_with("OK "), "{fixture}: {stdout}");
    }
}

#[test]
fn cli_rejects_tampered_copy_with_error_lines() {
    let repo_root = repo_root();
    let fixture = repo_root.join("runtime/fixtures/run-event-v0.2/journals/plain-run.jsonl");
    let text = std::fs::read_to_string(&fixture).expect("read fixture");

    // Tamper: corrupt the first 64-char hex run (a chain/payload digest) so
    // the chain recompute must fail.
    let bytes = text.as_bytes();
    let is_hex = |c: u8| c.is_ascii_hexdigit();
    let hex_range = {
        let mut scan = 0usize;
        loop {
            match text[scan..].find(|c: char| c.is_ascii_hexdigit()) {
                Some(offset) => {
                    let start = scan + offset;
                    let mut end = start;
                    while end < bytes.len() && is_hex(bytes[end]) {
                        end += 1;
                    }
                    if end - start == 64 {
                        break start..end;
                    }
                    scan = end.max(start + 1);
                }
                None => panic!("fixture carries no 64-hex digest"),
            }
        }
    };
    let hex = &text[hex_range.clone()];
    let flipped = if hex.as_bytes()[0] == b'0' { "1" } else { "0" };
    let tampered = text.replacen(hex, &format!("{flipped}{}", &hex[1..]), 1);
    assert_ne!(tampered, text, "tamper must change the journal");

    let dir = std::env::temp_dir().join(format!(
        "orz-jc-tamper-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let tampered_path = dir.join("tampered.jsonl");
    std::fs::write(&tampered_path, &tampered).expect("write tampered copy");

    let (code, stdout, stderr) = run_cli(&tampered_path, &repo_root);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        code, 1,
        "tampered journal must be rejected: {stdout}{stderr}"
    );
    assert!(stdout.is_empty(), "no stdout on rejection: {stdout}");
    assert!(
        stderr.contains("error: "),
        "judge errors on stderr: {stderr}"
    );
    assert!(stderr.contains("INVALID"), "summary line present: {stderr}");
}

#[test]
fn cli_usage_errors_exit_2() {
    let exe = env!("CARGO_BIN_EXE_journal-conformance");
    let repo_root = repo_root();

    // Missing --repo-root.
    let output = Command::new(exe)
        .arg(repo_root.join("runtime/fixtures/run-event-v0.1/journals/plain-run.jsonl"))
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(2));

    // Unknown flag.
    let output = Command::new(exe).arg("--bogus").output().expect("spawn");
    assert_eq!(output.status.code(), Some(2));

    // Two journal paths.
    let output = Command::new(exe)
        .arg("a.jsonl")
        .arg("b.jsonl")
        .arg("--repo-root")
        .arg(&repo_root)
        .output()
        .expect("spawn");
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn cli_reports_missing_journal_file() {
    let repo_root = repo_root();
    let missing = repo_root.join("runtime/fixtures/does-not-exist.jsonl");
    let (code, stdout, stderr) = run_cli(&missing, &repo_root);
    assert_eq!(
        code, 1,
        "missing file is an invalid-journal verdict, not a crash"
    );
    assert!(stderr.contains("journal file not found"), "{stderr}");
    assert!(stdout.is_empty(), "{stdout}");
}

//! Process-level tests for the `--real` flag (alpha test ruling 2026-08-06).
//!
//! Only the mutually-exclusive arg path is tested here — `--real` itself
//! reads the ADR-0006 Windows Credential Manager entry and hits the real
//! API, which must never happen in the offline suite.

use std::process::Command;

#[test]
fn real_and_fake_provider_are_mutually_exclusive() {
    let out = Command::new(env!("CARGO_BIN_EXE_orz-codex"))
        .args(["--real", "--fake-provider"])
        .output()
        .expect("spawn orz-codex");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "stderr should name the conflict: {stderr}"
    );
}

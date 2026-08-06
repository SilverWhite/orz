//! Process-level tests for the `--real` flag (alpha test ruling 2026-08-06).
//!
//! Only the fail-closed / mutually-exclusive arg paths are tested here —
//! running `--real` itself reads the ADR-0006 Windows Credential Manager
//! entry and hits the real API, which must never happen in the offline suite
//! (same discipline as the `ORZ_TEST_LIVE` gate).

use std::process::Command;

#[test]
fn real_and_fake_provider_are_mutually_exclusive() {
    let out = Command::new(env!("CARGO_BIN_EXE_orz"))
        .args(["--real", "--fake-provider"])
        .output()
        .expect("spawn orz");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mutually exclusive"),
        "stderr should name the conflict: {stderr}"
    );
}

#[test]
fn real_and_fake_provider_conflict_regardless_of_order() {
    let out = Command::new(env!("CARGO_BIN_EXE_orz"))
        .args(["--fake-provider", "--real"])
        .output()
        .expect("spawn orz");
    assert_eq!(out.status.code(), Some(2));
}

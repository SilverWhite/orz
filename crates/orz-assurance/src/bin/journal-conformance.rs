//! `journal-conformance` — standalone CLI wrapper around the offline Rust
//! journal judge (`crate::journal::conformance::validate_journal_file`).
//!
//! Task D S3 (2026-09-06, 用户裁决：接线形态 = 独立 CLI): the repository gate
//! `scripts/check_repository.py` runs this binary once per run-event fixture
//! journal instead of calling the Python judge entry point
//! (`assurance/run_event_journal_validation.py::validate_journal_file` —
//! retired from enforcement, now a frozen reference). This binary adds no
//! judgement of its own; the library function is the single enforcement
//! implementation, so the CLI surface stays trivially faithful to it.
//!
//! Usage:
//! ```text
//! journal-conformance <journal.jsonl> --repo-root <repository root>
//! ```
//! `--repo-root` may be repeated (the last value wins); any other flag or a
//! second positional argument is a usage error. Exit codes: `0` = journal
//! valid; `1` = journal invalid (one `error: …` line per judge error on
//! stderr); `2` = usage error. `ORZ_JOURNAL_CONFORMANCE_BIN` is not read
//! here — the gate resolves the executable; this binary only validates.

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: journal-conformance <journal.jsonl> --repo-root <repository root>";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut journal: Option<PathBuf> = None;
    let mut repo_root: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--repo-root" => match args.next() {
                Some(value) => repo_root = Some(PathBuf::from(value)),
                None => {
                    eprintln!("usage error: --repo-root requires a value\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            other if other.starts_with('-') => {
                eprintln!("usage error: unknown flag {other:?}\n{USAGE}");
                return ExitCode::from(2);
            }
            other => {
                if journal.is_some() {
                    eprintln!("usage error: exactly one journal path is accepted\n{USAGE}");
                    return ExitCode::from(2);
                }
                journal = Some(PathBuf::from(other));
            }
        }
    }
    let (Some(journal), Some(repo_root)) = (journal, repo_root) else {
        eprintln!("usage error: <journal> and --repo-root are both required\n{USAGE}");
        return ExitCode::from(2);
    };

    match orz_assurance::journal::validate_journal_file(&journal, &repo_root) {
        report if report.valid => {
            println!("OK {} events — {}", report.event_count, journal.display());
            ExitCode::SUCCESS
        }
        report => {
            for error in &report.errors {
                eprintln!("error: {error}");
            }
            eprintln!(
                "INVALID {} ({} error(s))",
                journal.display(),
                report.errors.len()
            );
            ExitCode::FAILURE
        }
    }
}

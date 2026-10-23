use anyhow::{Context, bail};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn check_protoc_good(protoc: &Path) -> anyhow::Result<()> {
    let output = Command::new(protoc)
        .arg("--version")
        .output()
        .context("Failed to execute protoc")?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "protoc --version failed, likely dotslash is missing; \
             try `cargo install dotslash`; stdout: {stdout:?}, stderr: {stderr:?}"
        );
    }
    Ok(())
}

fn is_github_actions() -> bool {
    env::var_os("GITHUB_ACTIONS").is_some()
}

/// Find `protoc` command.
///
/// Search order:
/// 1. `$PROTOC` environment variable (set by Bazel `build_script_env` or user override)
/// 2. `bin/protoc` walking up parent directories (dotslash wrapper for local dev)
/// 3. `protoc` on `$PATH` (system install or other tooling)
///
/// When `bin/protoc` exists but fails to execute (e.g. the dotslash wrapper running
/// in Bazel remote execution where `dotslash` is not installed), the error is not fatal —
/// we fall through to the PATH-based lookup instead.
///
/// Returns `Ok(None)` if not found and not in a strict environment (GitHub Actions).
pub fn find_protoc() -> anyhow::Result<Option<PathBuf>> {
    // 1. Check the PROTOC env var first. This is the standard override used by prost-build
    //    and is set by Bazel cargo_build_script build_script_env to point at a hermetic
    //    protoc binary instead of the dotslash wrapper.
    if let Ok(protoc_env) = env::var("PROTOC") {
        let protoc = PathBuf::from(&protoc_env);
        if protoc.try_exists()? {
            check_protoc_good(&protoc)?;
            return Ok(Some(protoc));
        }
    }

    // 2. Walk up directories looking for bin/protoc (dotslash wrapper).
    //    344 批（2026-10-23）：Windows 的 vendored 副本是 `bin/protoc.exe`
    //    （`build.ps1`/`build.bat` 的 PROTOC 即指向该处）；dotslash 包装
    //    `bin/protoc` 无后缀，Windows 上 `Command::new` 永不命中——两形态
    //    都试，按平台排优先序。
    let cwd = env::current_dir()?;
    let mut dir = cwd.clone();
    let mut dir_rel = PathBuf::new();
    loop {
        let candidates: [PathBuf; 2] = if cfg!(windows) {
            [dir_rel.join("bin/protoc.exe"), dir_rel.join("bin/protoc")]
        } else {
            [dir_rel.join("bin/protoc"), dir_rel.join("bin/protoc.exe")]
        };
        let mut attempted: Option<anyhow::Result<()>> = None;
        for protoc in candidates {
            if !protoc.try_exists()? {
                continue;
            }
            match check_protoc_good(&protoc) {
                Ok(()) => return Ok(Some(protoc)),
                Err(e) => {
                    // bin/protoc exists but can't execute — likely the dotslash wrapper
                    // in an environment without dotslash (e.g. Bazel remote execution).
                    // Fall through to PATH-based lookup below.
                    attempted = Some(Err(e));
                }
            }
        }
        if let Some(Err(e)) = attempted {
            eprintln!(
                "bin/protoc[.exe] found but failed to execute: {e:#}; \
                 trying protoc from PATH as fallback"
            );
            break;
        }
        if !dir.pop() {
            break;
        }
        dir_rel.push("..");
    }

    // 3. Try protoc from PATH (system install or other tooling).
    if check_protoc_good(Path::new("protoc")).is_ok() {
        return Ok(Some(PathBuf::from("protoc")));
    }

    // 4. Not found anywhere.
    let hint = "checked $PROTOC env, <workspace>/bin/protoc[.exe], and PATH; \
                this repo vendors `orz/bin/protoc.exe` — run `build.ps1`/`build.bat` \
                (they set PROTOC), or install protobuf and set PROTOC yourself";
    if is_github_actions() {
        return Err(anyhow::anyhow!("`protoc` not found ({hint})"));
    }
    eprintln!("`protoc` not found ({hint})");
    Ok(None)
}

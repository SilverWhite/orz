//! Build script for bundling ripgrep for the xai-grok-tools crate.
//!
//! - If `GROK_TOOLS_BUNDLE_RG_PATH` is set, always bundle it
//! - Otherwise, only bundle in release builds
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

const RG_VER: &str = "15.0.0";
const BFS_VER: &str = "4.1";
const UGREP_VER: &str = "7.7.0";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    bundle_rg()?;
    // bfs/ugrep back the bash-harness find/grep shadows (embedded_search_tools).
    bundle_search_tool("bfs", "BFS", BFS_VER)?;
    bundle_search_tool("ugrep", "UGREP", UGREP_VER)?;
    Ok(())
}

/// Bundle a prebuilt **static** search-tool binary (`bfs`/`ugrep`) when
/// `GROK_TOOLS_BUNDLE_<NAME>_PATH` points at one (supplied by the release
/// pipeline). Emits
/// `cfg(bundle_<name>)` so the crate's `include_bytes!` + self-extract engages.
///
/// No auto-download (unlike ripgrep): bfs/ugrep publish no prebuilt static
/// release assets, so the release pipeline supplies the path. Unset → not
/// bundled (the runtime resolver falls back to `~/.grok/vendor` / `$PATH`);
/// never a hard failure, so an un-wired build still succeeds.
fn bundle_search_tool(
    name: &str,
    name_uc: &str,
    ver: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let override_env = format!("GROK_TOOLS_BUNDLE_{name_uc}_PATH");
    println!("cargo:rerun-if-env-changed={override_env}");
    // Always declare the cfg so `#[cfg(bundle_<name>)]` is lint-clean when unset.
    println!("cargo:rustc-check-cfg=cfg(bundle_{name})");

    // The consumer (`embedded_search_tools`) is `#[cfg(unix)]`, so embedding on a
    // Windows target is dead weight — skip (mirrors the ripgrep Windows skip).
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        return Ok(());
    }

    let Some(src) = env::var(&override_env).ok().filter(|s| !s.is_empty()) else {
        return Ok(());
    };

    let gen_dir = PathBuf::from(env::var("OUT_DIR")?).join(format!("bundle-{name}"));
    fs::create_dir_all(&gen_dir)?;
    let dest = gen_dir.join(format!("{name}-{ver}-override.bin"));
    let _ = fs::remove_file(&dest);
    fs::copy(&src, &dest)
        .map_err(|e| format!("copy {override_env} from {src} to {}: {e}", dest.display()))?;

    println!("cargo:rustc-cfg=bundle_{name}");
    println!("cargo:rustc-env=GROK_TOOLS_{name_uc}_VER={ver}");
    println!("cargo:rustc-env=GROK_TOOLS_{name_uc}_TARGET=override");
    Ok(())
}

/// Download + embed ripgrep. Unchanged behavior; split out of `main` so the new
/// search-tool bundling runs regardless of ripgrep's early returns.
fn bundle_rg() -> Result<(), Box<dyn std::error::Error>> {
    // Only bundle in release builds to avoid slowing down cargo check.
    println!("cargo:rerun-if-env-changed=GROK_TOOLS_BUNDLE_RG_PATH");
    // Declare our custom cfg to the compiler so cfg(bundle_rg) is recognized by lints
    println!("cargo:rustc-check-cfg=cfg(bundle_rg)");

    let gen_dir = PathBuf::from(env::var("OUT_DIR")?).join("bundle-rg");
    fs::create_dir_all(&gen_dir)?;

    // Decide whether to bundle: path override OR release build
    let path_override = env::var("GROK_TOOLS_BUNDLE_RG_PATH").ok();
    let is_release = env::var("PROFILE").as_deref() == Ok("release");
    if path_override.is_none() && !is_release {
        return Ok(());
    }

    // Skip auto-bundling on Windows: ripgrep ships .zip on Windows (not
    // .tar.gz) and we have no zip-extraction path. Returning here BEFORE
    // emitting `cargo:rustc-cfg=bundle_rg` keeps include_bytes! macros gated
    // on cfg(bundle_rg) compiled-out, so the runtime falls back to `rg` on
    // PATH. Users install ripgrep separately (winget / scoop). An explicit
    // GROK_TOOLS_BUNDLE_RG_PATH still bundles regardless of target.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" && path_override.is_none() {
        return Ok(());
    }

    // Expose cfg so the crate can include the bundled bytes.
    println!("cargo:rustc-cfg=bundle_rg");
    println!("cargo:rustc-env=GROK_TOOLS_RG_VER={}", RG_VER);

    // If a local rg binary is provided, copy it directly (skips target check).
    if let Some(path) = path_override {
        let dest = gen_dir.join(format!("rg-{}-override.bin", RG_VER));
        println!("cargo:rustc-env=GROK_TOOLS_RG_TARGET=override");
        let _ = fs::remove_file(&dest);
        fs::copy(PathBuf::from(path.clone()), &dest).map_err(|e| {
            format!(
                "Failed copying GROK_TOOLS_BUNDLE_RG_PATH: {e} from path {path} to dest {}",
                dest.display()
            )
        })?;
        // FUS-TOOL-SCOPE-CONTRACT (2026-08-17): the override is embedded into
        // the orz binary and extracted at runtime in arbitrary containers
        // (e.g. Debian bookworm). A glibc-dynamic rg (e.g. Debian trixie's
        // /usr/bin/rg, requiring GLIBC_2.39) fails to load in those
        // containers with exit 1 + empty stdout — which grep used to collapse
        // into "No matches found". Refuse to bundle a dynamically-linked
        // binary on non-Windows targets so the failure surfaces at build
        // time, not in eval. Provide a static build (musl) or omit the
        // override to auto-download the official static release.
        if target_os != "windows" && elf_is_dynamically_linked(&dest)? {
            return Err(format!(
                "GROK_TOOLS_BUNDLE_RG_PATH ({path}) is a dynamically-linked \
                 binary; the bundled rg must be statically linked so it runs \
                 in any task container (FUS-TOOL-SCOPE-CONTRACT, 2026-08-17). \
                 Build one with `cargo install ripgrep --target \
                 x86_64-unknown-linux-musl`, or omit the override to \
                 auto-download the official static release."
            )
            .into());
        }
        return Ok(());
    }

    // Determine supported ripgrep asset triple for auto-download.
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let asset_triple = match (target_os.as_str(), target_arch.as_str()) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        _ => {
            return Err(format!(
                "Unsupported target for ripgrep bundling: {os}-{arch}. Set GROK_TOOLS_BUNDLE_RG_PATH to a local rg binary for offline or unsupported builds.",
                os = target_os,
                arch = target_arch
            ).into());
        }
    };

    println!("cargo:rustc-env=GROK_TOOLS_RG_TARGET={}", asset_triple);
    let dest = gen_dir.join(format!("rg-{}-{}.bin", RG_VER, asset_triple));
    let _ = fs::remove_file(&dest);

    let url = format!(
        "https://github.com/BurntSushi/ripgrep/releases/download/{v}/ripgrep-{v}-{t}.tar.gz",
        v = RG_VER,
        t = asset_triple
    );

    let bytes: Vec<u8> = {
        let resp = reqwest::blocking::get(&url).map_err(|e| {
            format!(
                "Failed to download ripgrep: {}\nSet GROK_TOOLS_BUNDLE_RG_PATH to a local rg for offline builds.",
                e
            )
        })?;
        if !resp.status().is_success() {
            return Err(format!(
                "HTTP {} downloading ripgrep. Set GROK_TOOLS_BUNDLE_RG_PATH for offline builds.",
                resp.status()
            )
            .into());
        }
        resp.bytes()?.to_vec()
    };

    let gz = flate2::read::GzDecoder::new(&bytes[..]);
    let mut ar = tar::Archive::new(gz);
    let mut found = false;
    for entry in ar.entries()? {
        let mut e = entry?;
        let p = e.path()?;
        if p.file_name().is_some_and(|n| n == "rg") {
            let data: Vec<u8> = {
                let mut v = Vec::new();
                io::copy(&mut e, &mut v)?;
                v
            };
            fs::write(&dest, &data)?;
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!(
            "Could not find 'rg' in ripgrep archive {}. Set GROK_TOOLS_BUNDLE_RG_PATH for offline builds.",
            url
        )
        .into());
    }

    Ok(())
}

/// Minimal ELF check: is the binary dynamically linked (has a PT_INTERP
/// program header)? Static musl builds have none. Returns `Err` for non-ELF
/// input (the override is unusable either way).
fn elf_is_dynamically_linked(path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() < 64 || &bytes[0..4] != b"\x7fELF" {
        return Err(format!("{} is not an ELF binary", path.display()).into());
    }
    let is_64 = bytes[4] == 2;
    let (phoff, phentsize, phnum): (usize, usize, usize) = if is_64 {
        (
            u64::from_le_bytes(bytes[32..40].try_into()?) as usize,
            u16::from_le_bytes(bytes[54..56].try_into()?) as usize,
            u16::from_le_bytes(bytes[56..58].try_into()?) as usize,
        )
    } else {
        (
            u32::from_le_bytes(bytes[28..32].try_into()?) as usize,
            u16::from_le_bytes(bytes[42..44].try_into()?) as usize,
            u16::from_le_bytes(bytes[44..46].try_into()?) as usize,
        )
    };
    for i in 0..phnum {
        let off = phoff.saturating_add(i * phentsize);
        if off.checked_add(8).is_none_or(|end| end > bytes.len()) {
            continue;
        }
        // PT_INTERP = 3.
        if u32::from_le_bytes(bytes[off..off + 4].try_into()?) == 3 {
            return Ok(true);
        }
    }
    Ok(false)
}

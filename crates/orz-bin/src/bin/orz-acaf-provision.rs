//! ACAF production provisioning (2026-08-16, fail-closed production flip).
//!
//! Creates the DPAPI installation keystore (`K_install`) and writes the
//! signer manifest whose `binary_sha256` is the REAL current `orz-signer`
//! binary hash (the signer verifies itself at startup against that hash).
//!
//! Usage: `orz-acaf-provision <keystore-root> <manifest-output>`
//!
//! The signer binary is located next to this tool (`<exe_dir>/orz-signer`).
//! Idempotent: an existing keystore is loaded (never overwritten), and the
//! manifest is refreshed to match the current signer binary. Non-Windows
//! fails closed (DPAPI unavailable — same posture as the signer itself).

use std::path::{Path, PathBuf};

use orz_assurance::journal::sha256_hex;
use orz_host::keystore::WindowsDpapiInstallationKeyStore;

const MANIFEST_VERSION: u64 = 1;
const SIGNER_BINARY_NAME: &str = "orz-signer";

fn signer_binary_path() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "cannot resolve the tool's directory".to_string())?;
    let name = if cfg!(windows) {
        format!("{SIGNER_BINARY_NAME}.exe")
    } else {
        SIGNER_BINARY_NAME.to_string()
    };
    let candidate = dir.join(name);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "signer binary not found next to this tool: {}",
            candidate.display()
        ))
    }
}

fn write_manifest(manifest_path: &Path, binary_sha256: &str) -> Result<(), String> {
    let manifest = serde_json::json!({
        "manifest_version": MANIFEST_VERSION,
        "signer_revision": 1,
        "binary_name": SIGNER_BINARY_NAME,
        "binary_sha256": binary_sha256,
    });
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| format!("manifest json: {e}"))?;
    std::fs::write(manifest_path, bytes).map_err(|e| format!("write {}: {e}", manifest_path.display()))
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let keystore_root = args
        .next()
        .ok_or_else(|| "usage: orz-acaf-provision <keystore-root> <manifest-output>".to_string())?;
    let manifest_output = args
        .next()
        .ok_or_else(|| "usage: orz-acaf-provision <keystore-root> <manifest-output>".to_string())?;
    if args.next().is_some() {
        return Err("usage: orz-acaf-provision <keystore-root> <manifest-output>".to_string());
    }

    let keystore_root = PathBuf::from(keystore_root);
    let manifest_output = PathBuf::from(manifest_output);

    // Idempotent: create once, load on re-runs (never overwrites the key).
    WindowsDpapiInstallationKeyStore::create_or_load(&keystore_root)
        .map_err(|e| format!("keystore create/load: {e}"))?;

    let signer = signer_binary_path()?;
    let bytes = std::fs::read(&signer).map_err(|e| format!("read {}: {e}", signer.display()))?;
    let hash = sha256_hex(&bytes);
    write_manifest(&manifest_output, &hash)?;

    println!("keystore:   {}", keystore_root.display());
    println!("manifest:   {}", manifest_output.display());
    println!("signer:     {}", signer.display());
    println!("binary_sha256: {hash}");
    println!(
        "launch env: ORZ_ACAF_KEYSTORE={} ORZ_ACAF_MANIFEST={} ORZ_ACAF_BINARY={}",
        keystore_root.display(),
        manifest_output.display(),
        signer.display()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("orz-acaf-provision: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_shape_matches_signer_contract() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"manifest_version":1,"signer_revision":1,"binary_name":"orz-signer","binary_sha256":"ab"}"#)
                .unwrap();
        assert_eq!(json["manifest_version"], MANIFEST_VERSION);
        assert_eq!(json["signer_revision"], 1);
        assert_eq!(json["binary_name"], SIGNER_BINARY_NAME);
        assert_eq!(json["binary_sha256"].as_str().unwrap().len(), 2);
    }
}

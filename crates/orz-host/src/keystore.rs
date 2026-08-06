//! Installation key store — the host-side permit signing seam (P1 permit).
//!
//! Mirrors the Python authority (`assurance/keystore.py` +
//! `installation-key-metadata-v0.1.schema.json`):
//!
//! - 32-byte random installation key
//! - Windows DPAPI-protected on disk: `<root>/installation-key.dpapi` =
//!   `MAGIC` ‖ `CryptProtectData(secret, entropy)` (fixed entropy, UI
//!   forbidden, current-user scope)
//! - `key_id = KEY-{sha256(secret)[:20].upper()}` (schema pattern
//!   `^KEY-[A-Z0-9._-]+$`)
//! - `<root>/installation-key.json` metadata written per
//!   `installation-key-metadata-v0.1.schema.json` — a Rust-created store is
//!   readable by the Python authority and validates against the schema
//!   (conformance parity, same MAGIC/entropy/blob layout).
//!
//! DPAPI is the only OS keystore — non-Windows creation fails closed,
//! matching the Python store's platform gate. `MemoryInstallationKeyStore`
//! mirrors Python's test-only adapter ("never accepted as an OS keystore").
//!
//! Signing delegates to `orz_assurance::permit::HmacSha256Signer` (ring);
//! loaded secrets are zeroised after use (stronger than the Python
//! best-effort scrub — Drop/`zeroize_bytes`).

use std::path::{Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use orz_assurance::credential::zeroize_bytes;
use orz_assurance::journal::{canonical_json, sha256_hex};
use orz_assurance::permit::{HmacSha256Signer, PermitError, PermitSigner};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};

/// Installation key size in bytes (schema const `key_bytes` = 32).
pub const KEY_BYTES: usize = 32;

/// Blob header marker (Python parity).
pub const MAGIC: &[u8] = b"LIF-ASSURANCE-INSTALLATION-KEY\x00\x01";

/// DPAPI entropy (Python parity — same protected blob layout).
pub const ENTROPY: &[u8] = b"lif-assurance-installation-key-v0.1";

/// Protected blob file name (Python parity).
pub const BLOB_NAME: &str = "installation-key.dpapi";

/// Metadata file name (Python parity).
pub const METADATA_NAME: &str = "installation-key.json";

/// Storage adapter id (schema enum value, Python parity).
pub const STORAGE_ID: &str = "windows-dpapi-current-user";

/// Errors from the installation key store.
#[derive(Debug, thiserror::Error)]
pub enum KeystoreError {
    #[error("installation key root is not a directory: {0}")]
    InvalidRoot(PathBuf),
    #[error("refusing to overwrite an existing installation key at {0}")]
    AlreadyExists(PathBuf),
    #[error("installation key files must be regular files: {0}")]
    NotRegularFile(PathBuf),
    #[error("installation key protected blob has an invalid header")]
    InvalidBlobHeader,
    #[error("installation key protected-blob digest mismatch")]
    BlobDigestMismatch,
    #[error("installation key identity check failed")]
    IdentityMismatch,
    #[error("installation key metadata is invalid: {0}")]
    Metadata(String),
    #[error("installation key must be exactly {0} bytes")]
    InvalidKeySize(usize),
    #[error("Windows DPAPI key storage is unavailable on this platform")]
    DpapiUnavailable,
    #[error("windows dpapi error: {0}")]
    Windows(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Install key metadata file — field parity with
/// `installation-key-metadata-v0.1.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub schema_version: String,
    pub metadata_kind: String,
    pub key_id: String,
    pub created_at: String,
    pub key_algorithm: String,
    pub key_bytes: u32,
    pub storage: String,
    pub protected_blob_sha256: Option<String>,
    pub secret_material_persisted_in_metadata: bool,
    pub limitations: Vec<String>,
}

fn key_id_for(secret: &[u8]) -> String {
    format!("KEY-{}", sha256_hex(secret)[..20].to_uppercase())
}

/// Cryptographically secure random key material (ring, like the Python
/// authority's `secrets.token_bytes`).
fn random_secret() -> Result<Vec<u8>, KeystoreError> {
    let mut secret = vec![0u8; KEY_BYTES];
    SystemRandom::new()
        .fill(&mut secret)
        .map_err(|e| KeystoreError::Windows(format!("secure random fill failed: {e}")))?;
    Ok(secret)
}

fn secret_ok(secret: &[u8]) -> bool {
    secret.len() == KEY_BYTES
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, KeystoreError> {
    let content = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

/// Atomically write `bytes` (temp + rename) — same discipline as the
/// snapshot store's `write_atomic`.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), KeystoreError> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Test-only key store; never accepted as an OS keystore (Python parity).
#[derive(Debug)]
pub struct MemoryInstallationKeyStore {
    key_id: String,
    secret: Vec<u8>,
}

impl MemoryInstallationKeyStore {
    /// Fresh random 32-byte key.
    pub fn new() -> Self {
        let secret = random_secret().expect("secure random key generation");
        let key_id = key_id_for(&secret);
        Self { key_id, secret }
    }

    /// Fixed key for tests.
    pub fn from_secret(secret: &[u8]) -> Result<Self, KeystoreError> {
        if !secret_ok(secret) {
            return Err(KeystoreError::InvalidKeySize(KEY_BYTES));
        }
        Ok(Self {
            key_id: key_id_for(secret),
            secret: secret.to_vec(),
        })
    }
}

impl Default for MemoryInstallationKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PermitSigner for MemoryInstallationKeyStore {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PermitError> {
        HmacSha256Signer::new(&self.key_id, &self.secret).sign(payload)
    }

    fn verify(&self, payload: &[u8], signature: &[u8]) -> bool {
        HmacSha256Signer::new(&self.key_id, &self.secret).verify(payload, signature)
    }
}

impl Drop for MemoryInstallationKeyStore {
    fn drop(&mut self) {
        zeroize_bytes(&mut self.secret);
    }
}

/// Windows DPAPI-backed installation key store (Python parity).
///
/// `create` requires an empty root; `load` validates magic, blob digest and
/// key identity. Non-Windows creation fails closed (`DpapiUnavailable`).
#[derive(Debug)]
pub struct WindowsDpapiInstallationKeyStore {
    root: PathBuf,
    key_id: String,
}

impl WindowsDpapiInstallationKeyStore {
    pub fn create(root: &Path) -> Result<Self, KeystoreError> {
        // Python parity: `create` creates the root itself
        // (`root.mkdir(parents=True, exist_ok=True)`).
        std::fs::create_dir_all(root)?;
        let blob_path = root.join(BLOB_NAME);
        let metadata_path = root.join(METADATA_NAME);
        // Python parity: refuse to overwrite either keystore file.
        if blob_path.exists() || metadata_path.exists() {
            return Err(KeystoreError::AlreadyExists(root.to_path_buf()));
        }
        let mut secret = random_secret()?;
        let key_id = key_id_for(&secret);
        let result = (|| {
            let protected = dpapi::protect(&secret, ENTROPY)?;
            let blob = [MAGIC, protected.as_slice()].concat();
            write_atomic(&blob_path, &blob)?;
            let metadata = KeyMetadata {
                schema_version: "0.1.0-draft".to_string(),
                metadata_kind: "installation_key_metadata".to_string(),
                key_id: key_id.clone(),
                created_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                key_algorithm: "hmac-sha256".to_string(),
                key_bytes: KEY_BYTES as u32,
                storage: STORAGE_ID.to_string(),
                protected_blob_sha256: Some(sha256_hex(&blob)),
                secret_material_persisted_in_metadata: false,
                limitations: vec![
                    "DPAPI ciphertext is scoped to the current Windows user and machine context."
                        .to_string(),
                ],
            };
            write_atomic(&root.join(METADATA_NAME), &canonical_json(&metadata)?)?;
            Ok(())
        })();
        // Zeroise the secret copy on every path (Python rollback parity: on
        // failure the partial artifacts are removed).
        zeroize_bytes(&mut secret);
        if let Err(e) = result {
            let _ = std::fs::remove_file(&blob_path);
            let _ = std::fs::remove_file(root.join(METADATA_NAME));
            return Err(e);
        }
        Ok(Self {
            root: root.to_path_buf(),
            key_id,
        })
    }

    pub fn load(root: &Path) -> Result<Self, KeystoreError> {
        if !root.is_dir() {
            return Err(KeystoreError::InvalidRoot(root.to_path_buf()));
        }
        let blob_path = root.join(BLOB_NAME);
        let metadata_path = root.join(METADATA_NAME);
        for path in [&blob_path, &metadata_path] {
            if !path.is_file() {
                return Err(KeystoreError::NotRegularFile(path.clone()));
            }
        }
        let metadata: KeyMetadata = load_json(&metadata_path)?;
        // Schema const parity (installation-key-metadata-v0.1.schema.json) —
        // fail closed on any drifted field.
        if metadata.storage != STORAGE_ID {
            return Err(KeystoreError::Metadata(format!(
                "installation key metadata uses the wrong storage adapter: {}",
                metadata.storage
            )));
        }
        if metadata.schema_version != "0.1.0-draft"
            || metadata.metadata_kind != "installation_key_metadata"
            || metadata.key_algorithm != "hmac-sha256"
            || metadata.key_bytes != KEY_BYTES as u32
            || metadata.secret_material_persisted_in_metadata
        {
            return Err(KeystoreError::Metadata(
                "installation key metadata violates schema consts".to_string(),
            ));
        }
        Ok(Self {
            root: root.to_path_buf(),
            key_id: metadata.key_id,
        })
    }

    /// Load an existing store or create a fresh one.
    pub fn create_or_load(root: &Path) -> Result<Self, KeystoreError> {
        std::fs::create_dir_all(root)?;
        if root.join(BLOB_NAME).exists() {
            Self::load(root)
        } else {
            Self::create(root)
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Read, validate and unprotect the stored secret. Callers must zeroise
    /// the returned buffer (RAII in this module — see `sign`).
    fn load_secret(&self) -> Result<Vec<u8>, KeystoreError> {
        let blob_path = self.root.join(BLOB_NAME);
        let metadata_path = self.root.join(METADATA_NAME);
        let metadata: KeyMetadata = load_json(&metadata_path)?;
        let stored = std::fs::read(&blob_path)?;
        if metadata.protected_blob_sha256.as_deref() != Some(sha256_hex(&stored).as_str()) {
            return Err(KeystoreError::BlobDigestMismatch);
        }
        if !stored.starts_with(MAGIC) || stored.len() == MAGIC.len() {
            return Err(KeystoreError::InvalidBlobHeader);
        }
        let mut secret = dpapi::unprotect(&stored[MAGIC.len()..], ENTROPY)?;
        let valid = secret_ok(&secret) && key_id_for(&secret) == self.key_id;
        if !valid {
            zeroize_bytes(&mut secret);
            return Err(KeystoreError::IdentityMismatch);
        }
        Ok(secret)
    }
}

impl PermitSigner for WindowsDpapiInstallationKeyStore {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, PermitError> {
        let mut secret = self
            .load_secret()
            .map_err(|e| PermitError::Signing(e.to_string()))?;
        let result = HmacSha256Signer::new(&self.key_id, &secret).sign(payload);
        zeroize_bytes(&mut secret);
        result
    }

    fn verify(&self, payload: &[u8], signature: &[u8]) -> bool {
        let mut secret = match self.load_secret() {
            Ok(secret) => secret,
            Err(_) => return false,
        };
        let ok = HmacSha256Signer::new(&self.key_id, &secret).verify(payload, signature);
        zeroize_bytes(&mut secret);
        ok
    }
}

#[cfg(windows)]
mod dpapi {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };
    use windows::core::w;

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        }
    }

    /// CryptProtectData with the fixed keystore entropy, UI forbidden.
    /// Returns the ciphertext (caller keeps `plaintext` alive during the call).
    pub(super) fn protect(
        plaintext: &[u8],
        entropy: &[u8],
    ) -> Result<Vec<u8>, super::KeystoreError> {
        let input = blob(plaintext);
        let ent = blob(entropy);
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &input,
                w!("LIF Assurance installation key v0.1"),
                Some(&ent),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .map_err(|e| super::KeystoreError::Windows(format!("CryptProtectData failed: {e}")))?;
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }

    /// CryptUnprotectData with the fixed keystore entropy.
    pub(super) fn unprotect(
        ciphertext: &[u8],
        entropy: &[u8],
    ) -> Result<Vec<u8>, super::KeystoreError> {
        let input = blob(ciphertext);
        let ent = blob(entropy);
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptUnprotectData(
                &input,
                None,
                Some(&ent),
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
            .map_err(|e| {
                super::KeystoreError::Windows(format!("CryptUnprotectData failed: {e}"))
            })?;
        }
        let result =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe {
            let _ = LocalFree(Some(HLOCAL(output.pbData as *mut _)));
        }
        Ok(result)
    }
}

#[cfg(not(windows))]
mod dpapi {
    pub(super) fn protect(
        _plaintext: &[u8],
        _entropy: &[u8],
    ) -> Result<Vec<u8>, super::KeystoreError> {
        Err(super::KeystoreError::DpapiUnavailable)
    }

    pub(super) fn unprotect(
        _ciphertext: &[u8],
        _entropy: &[u8],
    ) -> Result<Vec<u8>, super::KeystoreError> {
        Err(super::KeystoreError::DpapiUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-keystore-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn memory_store_sign_verify_round_trip() {
        let store = MemoryInstallationKeyStore::new();
        assert!(
            store.key_id().starts_with("KEY-"),
            "key id: {}",
            store.key_id()
        );
        let payload = b"canonical permit body";
        let signature = store.sign(payload).unwrap();
        assert!(store.verify(payload, &signature));
        assert!(!store.verify(b"tampered", &signature));
    }

    #[test]
    fn memory_store_key_id_matches_authority_derivation() {
        // Python `_key_id`: `KEY-{sha256(secret)[:20].upper()}`.
        let secret = [0x11u8; 32];
        let store = MemoryInstallationKeyStore::from_secret(&secret).unwrap();
        let expected = format!("KEY-{}", sha256_hex(&secret)[..20].to_uppercase());
        assert_eq!(store.key_id(), expected);
    }

    #[test]
    fn memory_store_rejects_wrong_key_size() {
        assert!(MemoryInstallationKeyStore::from_secret(&[0u8; 16]).is_err());
    }

    #[test]
    fn create_rejects_existing_key() {
        let dir = test_dir();
        let store = WindowsDpapiInstallationKeyStore::create(&dir);
        match store {
            Ok(_) => {
                // Windows: a second create must refuse to overwrite.
                let second = WindowsDpapiInstallationKeyStore::create(&dir);
                assert!(matches!(second, Err(KeystoreError::AlreadyExists(_))));
            }
            Err(KeystoreError::DpapiUnavailable) => {
                // Non-Windows fail-closed — the module contract.
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn dpapi_store_create_load_sign_round_trip() {
        let dir = test_dir();
        let store = WindowsDpapiInstallationKeyStore::create(&dir).unwrap();
        assert!(dir.join(BLOB_NAME).is_file());
        assert!(dir.join(METADATA_NAME).is_file());

        let payload = b"permit canonical body";
        let signature = store.sign(payload).unwrap();
        assert!(store.verify(payload, &signature));
        assert!(!store.verify(b"other", &signature));

        // Reload from disk — identity + blob digest validated.
        let loaded = WindowsDpapiInstallationKeyStore::load(&dir).unwrap();
        assert_eq!(loaded.key_id(), store.key_id());
        assert!(loaded.verify(payload, &signature));

        // Tampered blob → digest mismatch.
        let blob_path = dir.join(BLOB_NAME);
        let mut blob = std::fs::read(&blob_path).unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        std::fs::write(&blob_path, &blob).unwrap();
        assert!(matches!(
            WindowsDpapiInstallationKeyStore::load(&dir)
                .unwrap()
                .sign(payload),
            Err(PermitError::Signing(_))
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

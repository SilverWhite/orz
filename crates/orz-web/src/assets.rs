//! Embedded static assets (0br S2). Every path maps to an
//! `include_bytes!` — the table is explicit so "每区块标注来源件" stays
//! text-auditable and no build-time codegen is involved.
//!
//! Vendored third-party files live under `vendor/` and carry their upstream
//! version headers; their license obligations are recorded in the workspace
//! `THIRD-PARTY-NOTICES` (98.css / XP.css / marked — all MIT) and the
//! component register.

/// (route path, content type, bytes)
pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub bytes: &'static [u8],
}

macro_rules! asset {
    ($path:literal, $ctype:literal, $file:literal) => {
        Asset {
            path: $path,
            content_type: $ctype,
            bytes: include_bytes!(concat!("../assets/", $file)),
        }
    };
}

/// All embedded assets. Lookup is by exact route path.
pub static ASSETS: &[Asset] = &[
    asset!("/", "text/html; charset=utf-8", "index.html"),
    asset!("/index.html", "text/html; charset=utf-8", "index.html"),
    asset!("/assets/app.css", "text/css; charset=utf-8", "app.css"),
    asset!(
        "/assets/vendor/98.css",
        "text/css; charset=utf-8",
        "vendor/98.css"
    ),
    asset!(
        "/assets/vendor/XP.css",
        "text/css; charset=utf-8",
        "vendor/XP.css"
    ),
    asset!(
        "/assets/vendor/ms_sans_serif.woff",
        "font/woff",
        "vendor/ms_sans_serif.woff"
    ),
    asset!(
        "/assets/vendor/ms_sans_serif.woff2",
        "font/woff2",
        "vendor/ms_sans_serif.woff2"
    ),
    asset!(
        "/assets/vendor/ms_sans_serif_bold.woff",
        "font/woff",
        "vendor/ms_sans_serif_bold.woff"
    ),
    asset!(
        "/assets/vendor/ms_sans_serif_bold.woff2",
        "font/woff2",
        "vendor/ms_sans_serif_bold.woff2"
    ),
    asset!(
        "/assets/vendor/marked.umd.js",
        "application/javascript; charset=utf-8",
        "vendor/marked.umd.js"
    ),
    asset!(
        "/assets/app/state.js",
        "application/javascript; charset=utf-8",
        "app/state.js"
    ),
    asset!(
        "/assets/app/md.js",
        "application/javascript; charset=utf-8",
        "app/md.js"
    ),
    asset!(
        "/assets/app/api.js",
        "application/javascript; charset=utf-8",
        "app/api.js"
    ),
    asset!(
        "/assets/app/acp.js",
        "application/javascript; charset=utf-8",
        "app/acp.js"
    ),
    asset!(
        "/assets/app/journal.js",
        "application/javascript; charset=utf-8",
        "app/journal.js"
    ),
    asset!(
        "/assets/app/projection.js",
        "application/javascript; charset=utf-8",
        "app/projection.js"
    ),
    asset!(
        "/assets/app/widgets.js",
        "application/javascript; charset=utf-8",
        "app/widgets.js"
    ),
    asset!(
        "/assets/app/dialogs.js",
        "application/javascript; charset=utf-8",
        "app/dialogs.js"
    ),
    asset!(
        "/assets/app/keymap.js",
        "application/javascript; charset=utf-8",
        "app/keymap.js"
    ),
    asset!(
        "/assets/app/main.js",
        "application/javascript; charset=utf-8",
        "app/main.js"
    ),
];

pub fn lookup(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|a| a.path == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_asset_is_present_and_nonempty() {
        assert!(!ASSETS.is_empty());
        for a in ASSETS {
            assert!(!a.bytes.is_empty(), "asset {} is empty", a.path);
            assert!(!a.content_type.is_empty());
        }
        assert!(lookup("/").is_some(), "index must be served at /");
        assert!(lookup("/index.html").is_some());
        assert!(lookup("/assets/app/main.js").is_some());
        assert!(lookup("/assets/vendor/98.css").is_some());
        assert!(lookup("/assets/vendor/marked.umd.js").is_some());
        assert!(lookup("/nope").is_none());
    }

    #[test]
    fn vendored_files_carry_upstream_version_headers() {
        let css = lookup("/assets/vendor/98.css").unwrap();
        let head = String::from_utf8_lossy(&css.bytes[..120]);
        assert!(head.contains("98.css v0.1.21"), "provenance header missing");
        let xp = lookup("/assets/vendor/XP.css").unwrap();
        let head = String::from_utf8_lossy(&xp.bytes[..120]);
        assert!(head.contains("XP.css v0.2.6"), "provenance header missing");
        let js = lookup("/assets/vendor/marked.umd.js").unwrap();
        let head = String::from_utf8_lossy(&js.bytes[..200]);
        assert!(
            head.contains("marked v18.0.14"),
            "provenance header missing"
        );
    }

    /// Supply-chain pin: every vendored asset's embedded bytes must match
    /// the recorded sha256 in `MANIFEST.sha256.txt` (the digest set the
    /// 0br S2 receipt and the component register point at). A vendored
    /// file swapped without updating the MANIFEST fails the build's test
    /// gate.
    #[test]
    fn vendored_bytes_match_manifest_sha256() {
        const MANIFEST: &[u8] = include_bytes!("../assets/vendor/MANIFEST.sha256.txt");
        let text = std::str::from_utf8(MANIFEST).expect("manifest is utf-8");
        let entries = text.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(entries, 7, "manifest records exactly the 7 vendor files");
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let (digest, name) = line
                .split_once(char::is_whitespace)
                .expect("manifest line is `<sha256> <name>`");
            let name = name.trim_start_matches(['*', ' ']).trim();
            let route = format!("/assets/vendor/{name}");
            let asset = lookup(&route)
                .unwrap_or_else(|| panic!("manifest names {name} but it is not embedded"));
            let actual = hex_encode(asset.bytes);
            assert_eq!(
                actual, digest,
                "embedded {name} no longer matches the recorded sha256"
            );
        }
    }

    fn hex_encode(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        let mut s = String::with_capacity(out.len() * 2);
        for b in out {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }
}

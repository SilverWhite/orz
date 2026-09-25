//! Security primitives for the loopback workbench bridge (0br S2, survey
//! §6): per-boot token, loopback-only bind validation, run-id path safety,
//! Origin/Host checks.
//!
//! The distribution face must not become a new externally reachable surface
//! (design: `docs/audits/0BR_S1_WEB_FORM_SURVEY_2026-09-24.md` §6):
//! loopback-only bind + per-boot token on every WS/API route + Origin check
//! on WS upgrades. Static assets are the only tokenless routes.

/// Generate a per-boot access token: 256 bits of OS entropy, hex-encoded.
///
/// `rand::rng()` is seeded from the OS per process, so every `orz web`
/// launch gets a fresh token (frontend receives it via the URL fragment —
/// never sent to tokenless endpoints).
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let mut out = String::with_capacity(64);
    for _ in 0..4 {
        out.push_str(&format!("{:016x}", rng.random::<u64>()));
    }
    out
}

/// Constant-time comparison (XOR fold). For a per-boot loopback token there
/// is no timing oracle worth exploiting, but the fold costs nothing and
/// removes the question entirely.
pub fn token_ok(provided: Option<&str>, expected: &str) -> bool {
    let Some(provided) = provided else {
        return false;
    };
    let a = provided.as_bytes();
    let b = expected.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

/// Only loopback bind targets are allowed (`127.0.0.1` / `::1`).
pub fn bind_addr_allowed(ip: std::net::IpAddr) -> bool {
    ip.is_loopback()
}

/// Run ids are `RUN-<hex8>-<n>` shaped; anything outside
/// `[A-Za-z0-9_-]` is rejected outright so the `.gsa/runs/{id}` join can
/// never traverse (`..` contains none of the allowed characters but is
/// rejected by the same rule — no dots at all). Windows reserved device
/// names are additionally refused: `open(".gsa/runs/NUL/events.jsonl")`
/// would silently bind the NUL device instead of failing the lookup.
pub fn run_id_ok(run_id: &str) -> bool {
    if run_id.is_empty() || run_id.len() > 128 {
        return false;
    }
    if !run_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return false;
    }
    !matches!(
        run_id.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}

/// Session ids (`{session8}` segments of archive/conversation package
/// names — `.gsa/archives/{s8}.json.gz`, `.gsa/conversations/{s8}.json`)
/// follow the same path-safety rules as run ids: one flat token of
/// `[A-Za-z0-9_-]`, no dots or separators, no Windows reserved device
/// names.
pub fn session_id_ok(id: &str) -> bool {
    run_id_ok(id)
}

/// Strip the port from a Host/Origin authority (`[v6]:port` aware).
/// `None` when the authority is malformed (a non-numeric port segment must
/// not silently pass as its host prefix: `127.0.0.1:1.evil.com` is not
/// loopback just because it starts like it).
fn authority_host(authority: &str) -> Option<&str> {
    if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal: `[::1]` or `[::1]:port`.
        let end = rest.find(']')?;
        let host = &authority[..=end + 1]; // includes brackets
        let port = &authority[end + 2..];
        if port.is_empty()
            || port
                .strip_prefix(':')
                .is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return Some(host);
        }
        return None;
    }
    match authority.rfind(':') {
        // Exactly one colon separates host and (numeric) port.
        Some(pos) if authority.matches(':').count() == 1 => {
            let port = &authority[pos + 1..];
            if !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()) {
                Some(&authority[..pos])
            } else {
                None
            }
        }
        // Bare IPv6 without brackets has multiple colons — treat as host.
        Some(_) => Some(authority),
        None => Some(authority),
    }
}

/// Host-header check (DNS-rebinding defence): the authority must name the
/// loopback host. `[::]` (unspecified address) is deliberately absent —
/// no legitimate browser request names it against a loopback bind.
pub fn host_header_ok(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    let Some(name) = authority_host(host) else {
        return false;
    };
    matches!(
        name,
        "127.0.0.1" | "localhost" | "[::1]" | "[0:0:0:0:0:0:0:1]"
    )
}

/// Origin check on WS upgrades: browsers always send `Origin` on WS; a
/// cross-origin page (drive-by) names a non-loopback host and is rejected.
/// Non-browser clients may omit the header — the token still gates them.
pub fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    host_header_ok(Some(authority))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_hex_and_distinct_per_call() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64);
        assert_eq!(b.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(b.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(
            a, b,
            "per-boot tokens draw OS entropy; collision is a defect"
        );
    }

    #[test]
    fn token_gate_is_exact() {
        assert!(token_ok(Some("abc"), "abc"));
        assert!(!token_ok(Some("abcd"), "abc"));
        assert!(!token_ok(None, "abc"));
        assert!(!token_ok(Some(""), "abc"));
    }

    #[test]
    fn only_loopback_binds_are_allowed() {
        use std::net::IpAddr;
        assert!(bind_addr_allowed("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(bind_addr_allowed("::1".parse::<IpAddr>().unwrap()));
        assert!(!bind_addr_allowed("0.0.0.0".parse::<IpAddr>().unwrap()));
        assert!(!bind_addr_allowed("192.168.1.9".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn run_ids_reject_traversal_and_separators() {
        assert!(run_id_ok("RUN-6ab3dbe5-1"));
        assert!(run_id_ok("RUN-ab12cd34-12"));
        for bad in [
            "",
            "../etc",
            "a/b",
            "a\\b",
            "..",
            "RUN-x.y",
            "RUN x",
            "中文名",
            "a%2e%2e",
            // Windows reserved device names must never reach the join.
            "CON",
            "con",
            "NUL",
            "COM1",
            "LPT9",
        ] {
            assert!(!run_id_ok(bad), "run id {bad:?} must be rejected");
        }
    }

    #[test]
    fn session_ids_follow_the_run_id_path_rules() {
        assert!(session_id_ok("6ab3dbe5"));
        assert!(session_id_ok("RUN-CLI-suffix"));
        for bad in ["", "../x", "a/b", "a\\b", "x.y", "NUL", "中文名"] {
            assert!(!session_id_ok(bad), "session id {bad:?} must be rejected");
        }
    }

    #[test]
    fn host_header_rejects_malformed_port_segments() {
        assert!(host_header_ok(Some("127.0.0.1:21487")));
        assert!(!host_header_ok(Some("127.0.0.1:21487.evil.com")));
        assert!(!host_header_ok(Some("localhost:80eight")));
        assert!(!host_header_ok(Some("[::1]:junk")));
        assert!(host_header_ok(Some("[::1]:21487")));
        assert!(
            !host_header_ok(Some("[::]:21487")),
            "unspecified address is not loopback"
        );
    }

    #[test]
    fn host_header_must_be_loopback() {
        assert!(host_header_ok(Some("127.0.0.1:21487")));
        assert!(host_header_ok(Some("127.0.0.1")));
        assert!(host_header_ok(Some("localhost")));
        assert!(host_header_ok(Some("localhost:8080")));
        assert!(host_header_ok(Some("[::1]:21487")));
        assert!(!host_header_ok(Some("evil.example")));
        assert!(!host_header_ok(Some("evil.example:80")));
        assert!(!host_header_ok(None));
    }

    #[test]
    fn ws_origin_must_name_loopback_or_be_absent() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("http://127.0.0.1:21487")));
        assert!(origin_allowed(Some("http://localhost:3000")));
        assert!(origin_allowed(Some("https://127.0.0.1:21487")));
        assert!(!origin_allowed(Some("https://evil.example")));
        assert!(!origin_allowed(Some("http://evil.example:8080")));
        assert!(!origin_allowed(Some("ftp://127.0.0.1")));
        assert!(!origin_allowed(Some("null")));
    }
}

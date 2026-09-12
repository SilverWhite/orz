//! Pre-dispatch resource gate (FUS-HOST-RESOURCE-SAFETY §4.1, 0z S1).
//!
//! The 2026-09-12 real-machine run died twice on its own actions: once when
//! `D:` filled up (journal `os error 112` → fatal), once when host commit ran
//! out (`memory allocation of 200720 bytes failed`). Neither failure was a
//! concurrency artifact — the neighbouring session was asleep. So the framework
//! takes one job it can mechanize without becoming a host scheduler
//! (design §2): **read the headroom before dispatching heavy work, and refuse
//! when it is not there**.
//!
//! Three mechanical pieces, all testable without a real machine:
//!
//! 1. [`classify_action`] — a static classifier. Tool name first (a heavy tool
//!    is heavy whatever its arguments), then the command string for
//!    `run_terminal_cmd`. No model self-reporting, no natural language.
//! 2. [`CapacityProbe`] — read-only headroom readings for the volume the work
//!    would actually run on (`cwd`; 不换盘不换卷, design §2) plus host commit.
//! 3. [`ResourceGate::evaluate`] — the decision. Light actions are never gated;
//!    heavy actions need `free ≥ 8 GiB` **and** `commit free ≥ 25% limit`;
//!    readings that cannot be obtained are treated as insufficient
//!    (fail-closed, design §2 item 4).
//!
//! The tier ladder (`watch → soft → reclaim-direct → hard`) is computed here
//! and reported with every decision; the *actions* attached to the upper tiers
//! (cache reclaim, tree kill) belong to §4.6/§4.8 and land in S2. What S1 owns
//! is that the refusal happens **before dispatch** and carries its readings.

use std::path::Path;
use std::sync::{Arc, Mutex};

use xai_tty_utils::JobLimits;

/// One gibibyte, the unit every threshold in this module is expressed in.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Stable refusal code (design §5: pre-issue family, same envelope shape as the
/// budget / candidate / order refusals).
pub const CODE_RESOURCE_INSUFFICIENT: &str = "resource_insufficient";

/// Heavy-work release: the target volume must have at least this much free.
pub const HEAVY_RELEASE_FREE_BYTES: u64 = 8 * GIB;
/// Heavy-work release: commit headroom as a percentage of the commit limit.
pub const HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT: u32 = 25;

/// Ladder thresholds (design §11 裁决 1/3).
pub const WATCH_FREE_BYTES: u64 = 16 * GIB;
pub const SOFT_FREE_BYTES: u64 = 8 * GIB;
pub const RECLAIM_DIRECT_FREE_BYTES: u64 = 5 * GIB;
pub const HARD_FREE_BYTES: u64 = 2 * GIB;
pub const WATCH_COMMIT_USED_PERCENT: u32 = 70;
pub const SOFT_COMMIT_USED_PERCENT: u32 = 85;
pub const HARD_COMMIT_USED_PERCENT: u32 = 95;

/// Hard-limit derivation (design §4.7 / §11 裁决 4): commit = min(80% × limit,
/// limit − 4 GiB); CPU 80%; concurrency = cores.
pub const RUN_COMMIT_LIMIT_PERCENT: u64 = 80;
pub const RUN_COMMIT_RESERVE_BYTES: u64 = 4 * GIB;
pub const RUN_CPU_RATE_PERCENT: u32 = 80;

// ---------------------------------------------------------------------------
// Action classification
// ---------------------------------------------------------------------------

/// What a tool call costs the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionClass {
    /// Reads, small writes, unclassified commands.
    Light,
    /// Builds, links, package installs, container work, archive extraction,
    /// large output redirection.
    Heavy,
}

impl ActionClass {
    /// Machine-readable key (`light` | `heavy`).
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionClass::Light => "light",
            ActionClass::Heavy => "heavy",
        }
    }
}

/// Tools that are heavy by name, whatever the arguments say.
pub const HEAVY_TOOLS: &[&str] = &["run_tests"];

/// Programs whose very invocation is heavy work.
pub const HEAVY_PROGRAMS: &[&str] = &[
    "7z",
    "7za",
    "7zr",
    "ant",
    "apk",
    "apt",
    "apt-get",
    "ar",
    "bazel",
    "buck",
    "buildah",
    "bun",
    "cargo",
    "cargo-nextest",
    "cc",
    "choco",
    "clang",
    "clang++",
    "clang-cl",
    "cmake",
    "compress-archive",
    "conan",
    "conda",
    "convert",
    "csc",
    "ctest",
    "cypress",
    "deno",
    "dnf",
    "docker",
    "docker-compose",
    "dotnet",
    "dune",
    "esbuild",
    "expand",
    "expand-archive",
    "ffmpeg",
    "g++",
    "gcc",
    "ghc",
    "go",
    "gradle",
    "gradlew",
    "gunzip",
    "gzip",
    "hatch",
    "jest",
    "kotlinc",
    "ld",
    "ld.lld",
    "libtool",
    "lld",
    "magick",
    "make",
    "makecab",
    "mamba",
    "meson",
    "mingw32-make",
    "mix",
    "mold",
    "msbuild",
    "mvn",
    "mvnw",
    "nextest",
    "ninja",
    "nmake",
    "npm",
    "npx",
    "nuget",
    "nx",
    "objcopy",
    "opam",
    "pacman",
    "parcel",
    "pdm",
    "pip",
    "pip3",
    "pipenv",
    "playwright",
    "pnpm",
    "podman",
    "poetry",
    "qemu-system-x86_64",
    "qmake",
    "rollup",
    "rustc",
    "rustup",
    "sbt",
    "scoop",
    "soffice",
    "stack",
    "strip",
    "tar",
    "tsc",
    "turbo",
    "unzip",
    "uv",
    "vbcsc",
    "vcpkg",
    "vite",
    "vitest",
    "webpack",
    "winget",
    "wsl",
    "xz",
    "yarn",
    "yum",
    "zip",
    "zstd",
];

/// Interpreters that are heavy *only* when the invocation asks for an install
/// or a build — `python script.py` and `node -e …` stay light.
pub const CONDITIONAL_PROGRAMS: &[&str] = &[
    "java", "node", "perl", "php", "py", "python", "python3", "pythonw", "rscript", "ruby",
];

/// Sub-commands that make a [`CONDITIONAL_PROGRAMS`] entry heavy.
pub const CONDITIONAL_HEAVY_TOKENS: &[&str] =
    &["build", "bdist_wheel", "dist", "install", "sdist", "wheel"];

/// Shell wrappers we look through to reach the real program.
const WRAPPER_PROGRAMS: &[&str] = &[
    "bash",
    "cmd",
    "dash",
    "doas",
    "env",
    "ionice",
    "ksh",
    "nohup",
    "nice",
    "powershell",
    "pwsh",
    "setsid",
    "sh",
    "sudo",
    "time",
    "zsh",
];

/// Command flags that introduce a nested command string.
const COMMAND_FLAGS: &[&str] = &[
    "-c",
    "-command",
    "-encodedcommand",
    "-ic",
    "-lc",
    "-xc",
    "/c",
    "/k",
];

/// Redirection targets that are not files.
const NULL_TARGETS: &[&str] = &["$null", "/dev/null", "nul", "none"];

/// Classify a tool call. The tool name decides first; only
/// `run_terminal_cmd` inspects its command string.
pub fn classify_action(tool: &str, args: &serde_json::Value) -> ActionClass {
    if HEAVY_TOOLS.contains(&tool) {
        return ActionClass::Heavy;
    }
    if tool != "run_terminal_cmd" {
        return ActionClass::Light;
    }
    let Some(command) = args
        .get("command")
        .and_then(|c| c.as_str())
        .or_else(|| args.get("cmd").and_then(|c| c.as_str()))
    else {
        return ActionClass::Light;
    };
    classify_command(command)
}

/// Classify a shell command string (static; no shell actually runs).
pub fn classify_command(command: &str) -> ActionClass {
    for segment in split_segments(command) {
        let tokens = tokenize(&segment);
        if tokens.is_empty() {
            continue;
        }
        if segment_is_heavy(&tokens) {
            return ActionClass::Heavy;
        }
    }
    ActionClass::Light
}

/// Split a command line into independent commands (`;`, `&&`, `||`, `|`, `&`,
/// newlines). Quote handling is deliberately crude — this is a static
/// classifier, and a miss costs one extra reading or one refusal under
/// pressure, never a wrong action.
fn split_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = command.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        let doubled = (c == '&' && next == Some('&')) || (c == '|' && next == Some('|'));
        if doubled {
            segments.push(std::mem::take(&mut current));
            index += 2;
            continue;
        }
        if matches!(c, ';' | '|' | '&' | '\n' | '\r') {
            segments.push(std::mem::take(&mut current));
            index += 1;
            continue;
        }
        current.push(c);
        index += 1;
    }
    segments.push(current);
    segments
}

/// Whitespace tokenizer with minimal quote handling.
fn tokenize(segment: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in segment.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Does this segment write command output to a real file?
fn has_file_redirection(tokens: &[String]) -> bool {
    for (index, token) in tokens.iter().enumerate() {
        // `2>&1`, `1>&2`, `&>` shuffle descriptors, they do not create files.
        if token.contains(">&") {
            continue;
        }
        if token.starts_with('>') {
            let trimmed = token.trim_start_matches('>').trim();
            if !trimmed.is_empty() {
                if !is_null_target(trimmed) {
                    return true;
                }
            } else if let Some(next) = tokens.get(index + 1) {
                // Bare `>` / `>>`: the target is the next token.
                if !is_null_target(next) {
                    return true;
                }
            }
            continue;
        }
    }
    false
}

fn is_null_target(target: &str) -> bool {
    let normalized = target
        .trim_matches(|c| c == '"' || c == '\'')
        .to_ascii_lowercase();
    NULL_TARGETS.contains(&normalized.as_str())
}

fn normalize_program(token: &str) -> String {
    let trimmed = token.trim_matches(|c| c == '"' || c == '\'');
    let base = trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(trimmed)
        .to_ascii_lowercase();
    for suffix in [".exe", ".cmd", ".bat", ".com", ".ps1"] {
        if let Some(stripped) = base.strip_suffix(suffix) {
            return stripped.to_string();
        }
    }
    base
}

fn is_env_assignment(token: &str) -> bool {
    let Some((key, _)) = token.split_once('=') else {
        return false;
    };
    !key.is_empty()
        && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !key.starts_with('-')
}

/// What a wrapper's prologue asks us to do next.
enum WrapperStep {
    /// Keep scanning this segment's tokens from this index.
    Next(usize),
    /// The wrapper's own command string starts here — classify it as a command.
    Nested(String),
}

/// Walk past wrappers and flags to the program that will actually run.
fn segment_is_heavy(tokens: &[String]) -> bool {
    if has_file_redirection(tokens) {
        return true;
    }
    let mut index = 0usize;
    let mut guard = 0usize;
    while index < tokens.len() && guard < 8 {
        guard += 1;
        let raw = tokens[index].trim_matches(|c| c == '"' || c == '\'');
        if raw.is_empty() || raw == "&" || is_env_assignment(raw) {
            index += 1;
            continue;
        }
        let program = normalize_program(raw);
        if WRAPPER_PROGRAMS.contains(&program.as_str()) {
            match skip_wrapper(tokens, index) {
                WrapperStep::Next(next) => {
                    index = next;
                    continue;
                }
                WrapperStep::Nested(command) => {
                    if command.trim().is_empty() {
                        return false;
                    }
                    return classify_command(&command) == ActionClass::Heavy;
                }
            }
        }
        if HEAVY_PROGRAMS.contains(&program.as_str()) {
            return true;
        }
        if CONDITIONAL_PROGRAMS.contains(&program.as_str()) {
            return tokens[index + 1..].iter().any(|token| {
                let lowered = token
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_ascii_lowercase();
                CONDITIONAL_HEAVY_TOKENS.contains(&lowered.as_str())
            });
        }
        // A real program that is neither heavy nor conditional: the segment is
        // light unless a *later* pipeline member is heavy — those became their
        // own segments in `split_segments`.
        return false;
    }
    false
}

/// Interpret a wrapper's prologue.
fn skip_wrapper(tokens: &[String], index: usize) -> WrapperStep {
    let mut cursor = index + 1;
    while cursor < tokens.len() {
        let raw = tokens[cursor].trim_matches(|c| c == '"' || c == '\'');
        let lowered = raw.to_ascii_lowercase();
        if is_command_flag(&lowered) {
            // Everything after the flag is the nested command string (the shell
            // re-parses it, so we do too).
            return WrapperStep::Nested(tokens[cursor + 1..].join(" "));
        }
        if raw.starts_with('-') || raw.starts_with('/') || is_env_assignment(raw) {
            cursor += 1;
            continue;
        }
        return WrapperStep::Next(cursor);
    }
    WrapperStep::Next(cursor)
}

/// `-c` / `-lc` / `-Command` / `/C` … — the flag that hands over a command.
fn is_command_flag(lowered: &str) -> bool {
    if COMMAND_FLAGS.contains(&lowered) {
        return true;
    }
    // Bundled short flags (`-lc`, `-ic`, `-xc`): all letters, ends in `c`.
    lowered.starts_with('-')
        && !lowered.starts_with("--")
        && lowered.len() >= 2
        && lowered.ends_with('c')
        && lowered[1..].chars().all(|c| c.is_ascii_alphabetic())
}

// ---------------------------------------------------------------------------
// Readings
// ---------------------------------------------------------------------------

/// Quality of a reading set — `unavailable` is treated as "not enough"
/// (fail-closed, design §2 item 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceQuality {
    Available,
    Unavailable,
}

impl SourceQuality {
    /// Machine-readable key.
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceQuality::Available => "available",
            SourceQuality::Unavailable => "unavailable",
        }
    }
}

/// One mechanical headroom reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostCapacitySnapshot {
    /// Unix milliseconds when the reading was taken.
    pub collected_at_ms: u64,
    /// Live volume free bytes (the volume the heavy work would run on).
    pub volume_free_bytes: u64,
    /// Volume capacity in bytes (0 when unknown).
    pub volume_total_bytes: u64,
    /// Host commit limit in bytes (0 when unknown).
    pub commit_limit_bytes: u64,
    /// Host committed bytes (0 when unknown).
    pub commit_used_bytes: u64,
    pub source_quality: SourceQuality,
}

impl HostCapacitySnapshot {
    /// An unreadable probe result (fail-closed input).
    pub fn unavailable() -> Self {
        Self {
            collected_at_ms: now_ms(),
            volume_free_bytes: 0,
            volume_total_bytes: 0,
            commit_limit_bytes: 0,
            commit_used_bytes: 0,
            source_quality: SourceQuality::Unavailable,
        }
    }

    /// Commit headroom in bytes (`None` when the limit is unknown).
    pub fn commit_free_bytes(&self) -> Option<u64> {
        (self.commit_limit_bytes > 0).then(|| {
            self.commit_limit_bytes
                .saturating_sub(self.commit_used_bytes)
        })
    }

    /// Commit used, in percent of the limit (`None` when the limit is unknown).
    pub fn commit_used_percent(&self) -> Option<u32> {
        (self.commit_limit_bytes > 0).then(|| {
            ((self.commit_used_bytes as u128 * 100) / self.commit_limit_bytes as u128) as u32
        })
    }

    /// JSON rendering for the refusal/observation envelope (mechanical
    /// readings, never prose).
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "collected_at_ms": self.collected_at_ms,
            "volume_free_bytes": self.volume_free_bytes,
            "volume_total_bytes": self.volume_total_bytes,
            "commit_limit_bytes": self.commit_limit_bytes,
            "commit_used_bytes": self.commit_used_bytes,
            "commit_free_bytes": self.commit_free_bytes(),
            "source_quality": self.source_quality.as_str(),
        })
    }

    /// Human-readable headroom line (both faces see the same numbers).
    pub fn describe(&self) -> String {
        if self.source_quality == SourceQuality::Unavailable {
            return "readings unavailable (capacity probe failed or unsupported)".to_string();
        }
        let commit = match (self.commit_free_bytes(), self.commit_used_percent()) {
            (Some(free), Some(percent)) => format!(
                "commit {} free of {} ({}% used)",
                gib(free),
                gib(self.commit_limit_bytes),
                percent
            ),
            _ => "commit unknown".to_string(),
        };
        format!(
            "volume {} free of {}, {}",
            gib(self.volume_free_bytes),
            gib(self.volume_total_bytes),
            commit
        )
    }
}

/// Read-only capacity probe. Injected at host assembly, like the session-volume
/// family (`SessionVolumeRoot` / `SessionVolumeAccess`).
pub trait CapacityProbe: Send + Sync {
    /// Read the headroom for the volume holding `path` plus host commit.
    fn probe(&self, path: &Path) -> HostCapacitySnapshot;
}

/// The production probe: volume free space + host commit charge.
///
/// Windows: `GetDiskFreeSpaceExW` + `GlobalMemoryStatusEx`
/// (`ullTotalPageFile` is the commit limit Run B exhausted).
/// Linux: `statvfs` + `/proc/meminfo` (`CommitLimit` / `Committed_AS`).
/// Anything the platform cannot answer comes back as
/// [`SourceQuality::Unavailable`] — never a guessed number.
#[derive(Debug, Default)]
pub struct SystemCapacityProbe;

impl CapacityProbe for SystemCapacityProbe {
    fn probe(&self, path: &Path) -> HostCapacitySnapshot {
        match (probe_volume(path), probe_commit()) {
            (Some((free, total)), Some((limit, used))) => HostCapacitySnapshot {
                collected_at_ms: now_ms(),
                volume_free_bytes: free,
                volume_total_bytes: total,
                commit_limit_bytes: limit,
                commit_used_bytes: used,
                source_quality: SourceQuality::Available,
            },
            // Partial readings are not enough to release heavy work: the
            // decision needs both axes (fail-closed).
            _ => HostCapacitySnapshot::unavailable(),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(windows)]
fn probe_volume(path: &Path) -> Option<(u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    use windows::core::PCWSTR;

    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut available_to_caller = 0u64;
    let mut total = 0u64;
    let mut total_free = 0u64;
    unsafe {
        GetDiskFreeSpaceExW(
            PCWSTR(wide.as_ptr()),
            Some(&mut available_to_caller),
            Some(&mut total),
            Some(&mut total_free),
        )
    }
    .ok()?;
    let free = if available_to_caller > 0 {
        available_to_caller
    } else {
        total_free
    };
    Some((free, total))
}

#[cfg(windows)]
fn probe_commit() -> Option<(u64, u64)> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    let limit = status.ullTotalPageFile;
    let free = status.ullAvailPageFile;
    (limit > 0).then(|| (limit, limit.saturating_sub(free)))
}

#[cfg(target_os = "linux")]
fn probe_volume(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;

    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) } != 0 {
        return None;
    }
    let block = stats.f_frsize.max(stats.f_bsize) as u64;
    Some((
        block.saturating_mul(stats.f_bavail as u64),
        block.saturating_mul(stats.f_blocks as u64),
    ))
}

#[cfg(target_os = "linux")]
fn probe_commit() -> Option<(u64, u64)> {
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    let read_kib = |key: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|line| line.starts_with(key))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u64>().ok())
            .map(|kib| kib * 1024)
    };
    let limit = read_kib("CommitLimit:")?;
    let used = read_kib("Committed_AS:")?;
    (limit > 0).then_some((limit, used.min(limit)))
}

#[cfg(not(any(windows, target_os = "linux")))]
fn probe_volume(_path: &Path) -> Option<(u64, u64)> {
    None
}

#[cfg(not(any(windows, target_os = "linux")))]
fn probe_commit() -> Option<(u64, u64)> {
    None
}

// ---------------------------------------------------------------------------
// Tier ladder and decision
// ---------------------------------------------------------------------------

/// Where the machine currently sits on the ladder (design §4.1 table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceTier {
    Normal,
    Watch,
    Soft,
    ReclaimDirect,
    Hard,
}

impl ResourceTier {
    /// Machine-readable key.
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceTier::Normal => "normal",
            ResourceTier::Watch => "watch",
            ResourceTier::Soft => "soft",
            ResourceTier::ReclaimDirect => "reclaim_direct",
            ResourceTier::Hard => "hard",
        }
    }
}

/// Derive the ladder tier from a reading. The tier is *reported* (and drives
/// the refusal reason); the upper-tier actions (reclaim, tree kill) are S2.
pub fn tier_for(snapshot: &HostCapacitySnapshot) -> ResourceTier {
    let used = snapshot.commit_used_percent().unwrap_or(0);
    let free = snapshot.volume_free_bytes;
    if free < HARD_FREE_BYTES || used > HARD_COMMIT_USED_PERCENT {
        ResourceTier::Hard
    } else if free < RECLAIM_DIRECT_FREE_BYTES {
        ResourceTier::ReclaimDirect
    } else if free < SOFT_FREE_BYTES || used > SOFT_COMMIT_USED_PERCENT {
        ResourceTier::Soft
    } else if free < WATCH_FREE_BYTES || used > WATCH_COMMIT_USED_PERCENT {
        ResourceTier::Watch
    } else {
        ResourceTier::Normal
    }
}

/// The gate's verdict.
#[derive(Clone, Debug)]
pub enum GateDecision {
    /// Dispatch may proceed.
    Allow {
        class: ActionClass,
        tier: ResourceTier,
        snapshot: HostCapacitySnapshot,
    },
    /// Dispatch is refused before anything starts.
    Refuse {
        code: &'static str,
        reason: String,
        class: ActionClass,
        tier: Option<ResourceTier>,
        snapshot: HostCapacitySnapshot,
    },
}

impl GateDecision {
    /// True when the call was allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, GateDecision::Allow { .. })
    }
}

/// The gate: one probe handle, one last-reading cell for the observation face.
pub struct ResourceGate {
    probe: Arc<dyn CapacityProbe>,
    last: Mutex<Option<HostCapacitySnapshot>>,
}

impl ResourceGate {
    /// Build a gate around a probe.
    pub fn new(probe: Arc<dyn CapacityProbe>) -> Self {
        Self {
            probe,
            last: Mutex::new(None),
        }
    }

    /// The probe handle (observation / re-reads).
    pub fn probe(&self) -> Arc<dyn CapacityProbe> {
        self.probe.clone()
    }

    /// The most recent reading this gate took.
    pub fn last_snapshot(&self) -> Option<HostCapacitySnapshot> {
        *self.last.lock().unwrap()
    }

    /// Decide whether `class` may be dispatched with the work running on the
    /// volume holding `volume_path`.
    pub fn evaluate(&self, volume_path: &Path, class: ActionClass) -> GateDecision {
        let snapshot = self.probe.probe(volume_path);
        *self.last.lock().unwrap() = Some(snapshot);
        // Light actions are never gated: the gate exists for build-shaped work,
        // and gating reads would make an out-of-space machine unusable.
        if class == ActionClass::Light {
            return GateDecision::Allow {
                class,
                tier: tier_for(&snapshot),
                snapshot,
            };
        }
        if snapshot.source_quality == SourceQuality::Unavailable {
            return GateDecision::Refuse {
                code: CODE_RESOURCE_INSUFFICIENT,
                reason: format!(
                    "heavy action refused before dispatch — {}: headroom cannot be \
                     verified, so it is treated as insufficient (fail-closed). \
                     Nothing was started.",
                    snapshot.describe()
                ),
                class,
                tier: None,
                snapshot,
            };
        }
        let tier = tier_for(&snapshot);
        let free_ok = snapshot.volume_free_bytes >= HEAVY_RELEASE_FREE_BYTES;
        let commit_headroom_ok = snapshot.commit_free_bytes().is_some_and(|free| {
            (free as u128 * 100)
                >= HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT as u128
                    * snapshot.commit_limit_bytes as u128
        });
        if free_ok && commit_headroom_ok {
            return GateDecision::Allow {
                class,
                tier,
                snapshot,
            };
        }
        let mut shortfalls = Vec::new();
        if !free_ok {
            shortfalls.push(format!(
                "volume free {} < {} required",
                gib(snapshot.volume_free_bytes),
                gib(HEAVY_RELEASE_FREE_BYTES)
            ));
        }
        if !commit_headroom_ok {
            shortfalls.push(format!(
                "commit headroom {}% < {}% required",
                snapshot
                    .commit_used_percent()
                    .map(|used| 100u32.saturating_sub(used))
                    .unwrap_or(0),
                HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT
            ));
        }
        GateDecision::Refuse {
            code: CODE_RESOURCE_INSUFFICIENT,
            reason: format!(
                "heavy action refused before dispatch — resource headroom insufficient \
                 (tier {}): {}; {}. Nothing was started.",
                tier.as_str(),
                shortfalls.join("; "),
                snapshot.describe()
            ),
            class,
            tier: Some(tier),
            snapshot,
        }
    }
}

/// Run-level hard ceilings derived from a reading (design §4.7 / §11 裁决 4).
///
/// `commit = min(80% × limit, limit − 4 GiB)`, CPU 80%, concurrency = cores.
/// Unknown commit limit → no commit ceiling (the gate, not the kernel, is then
/// the only defence — recorded rather than silently invented).
pub fn default_job_limits(snapshot: &HostCapacitySnapshot) -> JobLimits {
    let commit_limit_bytes = (snapshot.commit_limit_bytes > 0)
        .then(|| {
            let eighty = snapshot.commit_limit_bytes / 100 * RUN_COMMIT_LIMIT_PERCENT;
            let reserved = snapshot
                .commit_limit_bytes
                .saturating_sub(RUN_COMMIT_RESERVE_BYTES);
            eighty.min(reserved)
        })
        // The reserve rule can floor the ceiling at zero (a machine whose whole
        // commit limit is the reserve): a zero ceiling would make the job
        // unable to start anything, so it means "no commit ceiling" instead.
        .filter(|value| *value > 0);
    JobLimits {
        commit_limit_bytes,
        active_process: Some(
            std::thread::available_parallelism()
                .map(|n| n.get() as u32)
                .unwrap_or(4),
        ),
        cpu_rate_percent: Some(RUN_CPU_RATE_PERCENT),
    }
}

/// Render bytes as GiB with two decimals (audit text only).
pub fn gib(bytes: u64) -> String {
    format!("{:.2} GiB", bytes as f64 / GIB as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct StubProbe {
        snapshot: HostCapacitySnapshot,
    }

    impl StubProbe {
        fn available(free: u64, commit_limit: u64, commit_used: u64) -> Arc<dyn CapacityProbe> {
            Arc::new(Self {
                snapshot: HostCapacitySnapshot {
                    collected_at_ms: 1,
                    volume_free_bytes: free,
                    volume_total_bytes: 100 * GIB,
                    commit_limit_bytes: commit_limit,
                    commit_used_bytes: commit_used,
                    source_quality: SourceQuality::Available,
                },
            })
        }

        fn unavailable() -> Arc<dyn CapacityProbe> {
            Arc::new(Self {
                snapshot: HostCapacitySnapshot::unavailable(),
            })
        }
    }

    impl CapacityProbe for StubProbe {
        fn probe(&self, _path: &Path) -> HostCapacitySnapshot {
            self.snapshot
        }
    }

    fn make_gate(probe: Arc<dyn CapacityProbe>) -> ResourceGate {
        ResourceGate::new(probe)
    }

    fn path() -> PathBuf {
        PathBuf::from(".")
    }

    // ── classifier ──────────────────────────────────────────────────────

    #[test]
    fn heavy_programs_are_heavy() {
        for command in [
            "cargo build --release",
            "cargo test -p orz-host",
            "rustc main.rs -o main",
            "make -j8",
            "cmake -B build",
            "docker build -t x .",
            "npm install",
            "pnpm install --frozen-lockfile",
            "7z x archive.7z",
            "tar -xzf src.tar.gz",
            "go build ./...",
            "dotnet build",
            "C:\\tools\\cargo.exe build",
            "./build/rustc --version",
        ] {
            assert_eq!(
                classify_command(command),
                ActionClass::Heavy,
                "expected heavy: {command}"
            );
        }
    }

    #[test]
    fn light_commands_stay_light() {
        for command in [
            "git status --short",
            "ls -la",
            "dir",
            "type README.md",
            "rg --files",
            "grep -r cargo src/",
            "echo hello",
            "python script.py --check",
            "node -e \"console.log(1)\"",
            "java -version",
            "cat build.log",
            "cmd /C ping -n 2 127.0.0.1 > NUL",
            "echo done > /dev/null",
            "echo done > $null",
            "python -c \"print(1)\" 2>&1",
        ] {
            assert_eq!(
                classify_command(command),
                ActionClass::Light,
                "expected light: {command}"
            );
        }
    }

    #[test]
    fn wrappers_and_pipelines_reach_the_real_program() {
        assert_eq!(classify_command("cmd /C cargo build"), ActionClass::Heavy);
        assert_eq!(
            classify_command("powershell -NoProfile -Command \"npm install\""),
            ActionClass::Heavy
        );
        assert_eq!(classify_command("bash -lc 'make -j4'"), ActionClass::Heavy);
        assert_eq!(
            classify_command("sudo env FOO=1 cargo build"),
            ActionClass::Heavy
        );
        assert_eq!(
            classify_command("git pull && cargo test"),
            ActionClass::Heavy
        );
        assert_eq!(
            classify_command("cd /tmp | pip install requests"),
            ActionClass::Heavy
        );
    }

    #[test]
    fn conditional_programs_need_a_build_word() {
        assert_eq!(
            classify_command("python -m pip install requests"),
            ActionClass::Heavy
        );
        assert_eq!(
            classify_command("python setup.py build"),
            ActionClass::Heavy
        );
        assert_eq!(classify_command("npm run dev"), ActionClass::Heavy);
        assert_eq!(
            classify_command("node scripts/check.js"),
            ActionClass::Light
        );
    }

    #[test]
    fn file_redirection_is_heavy_but_null_redirection_is_not() {
        assert_eq!(classify_command("ls > listing.txt"), ActionClass::Heavy);
        assert_eq!(classify_command("cmd /C dir > out.log"), ActionClass::Heavy);
        assert_eq!(
            classify_command("ping -n 1 127.0.0.1 > NUL"),
            ActionClass::Light
        );
    }

    #[test]
    fn tool_name_decides_first() {
        assert_eq!(
            classify_action("run_tests", &serde_json::json!({})),
            ActionClass::Heavy
        );
        assert_eq!(
            classify_action("read_file", &serde_json::json!({ "path": "a.rs" })),
            ActionClass::Light
        );
        assert_eq!(
            classify_action(
                "run_terminal_cmd",
                &serde_json::json!({ "command": "cargo build" })
            ),
            ActionClass::Heavy
        );
        // Missing command string: nothing to classify → light (the tool's own
        // validation owns a malformed call).
        assert_eq!(
            classify_action("run_terminal_cmd", &serde_json::json!({})),
            ActionClass::Light
        );
    }

    // ── tiers ───────────────────────────────────────────────────────────

    fn snapshot(free: u64, commit_limit: u64, commit_used: u64) -> HostCapacitySnapshot {
        HostCapacitySnapshot {
            collected_at_ms: 0,
            volume_free_bytes: free,
            volume_total_bytes: 100 * GIB,
            commit_limit_bytes: commit_limit,
            commit_used_bytes: commit_used,
            source_quality: SourceQuality::Available,
        }
    }

    #[test]
    fn tiers_follow_the_ladder() {
        assert_eq!(
            tier_for(&snapshot(40 * GIB, 32 * GIB, 8 * GIB)),
            ResourceTier::Normal
        );
        assert_eq!(
            tier_for(&snapshot(12 * GIB, 32 * GIB, 8 * GIB)),
            ResourceTier::Watch
        );
        assert_eq!(
            tier_for(&snapshot(6 * GIB, 32 * GIB, 8 * GIB)),
            ResourceTier::Soft
        );
        assert_eq!(
            tier_for(&snapshot(4 * GIB, 32 * GIB, 8 * GIB)),
            ResourceTier::ReclaimDirect
        );
        assert_eq!(
            tier_for(&snapshot(GIB, 32 * GIB, 8 * GIB)),
            ResourceTier::Hard
        );
        // The commit axis alone can push the tier up (95% used → hard).
        assert_eq!(
            tier_for(&snapshot(40 * GIB, 32 * GIB, 31 * GIB)),
            ResourceTier::Hard
        );
        // 70% used → watch on the commit axis alone.
        assert_eq!(
            tier_for(&snapshot(40 * GIB, 100 * GIB, 71 * GIB)),
            ResourceTier::Watch
        );
    }

    // ── decisions ───────────────────────────────────────────────────────

    #[test]
    fn heavy_allowed_with_ample_headroom() {
        let gate = make_gate(StubProbe::available(40 * GIB, 32 * GIB, 8 * GIB));
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Allow { tier, .. } => assert_eq!(tier, ResourceTier::Normal),
            other => panic!("expected allow, got {other:?}"),
        }
    }

    #[test]
    fn heavy_refused_below_the_release_thresholds() {
        // Volume fine, commit headroom only ~9% (design requires 25%).
        let gate = make_gate(StubProbe::available(40 * GIB, 32 * GIB, 29 * GIB));
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Refuse {
                code,
                reason,
                tier,
                class,
                ..
            } => {
                assert_eq!(code, CODE_RESOURCE_INSUFFICIENT);
                assert_eq!(class, ActionClass::Heavy);
                assert_eq!(tier, Some(ResourceTier::Soft));
                assert!(
                    reason.contains("commit headroom"),
                    "reason must name the shortfall: {reason}"
                );
                assert!(
                    reason.contains("Nothing was started"),
                    "refusal must state that nothing ran: {reason}"
                );
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        // Free space below the release threshold, commit healthy.
        let gate = make_gate(StubProbe::available(7 * GIB, 32 * GIB, 4 * GIB));
        assert!(!gate.evaluate(&path(), ActionClass::Heavy).is_allowed());
    }

    #[test]
    fn release_thresholds_are_inclusive() {
        // Exactly 8 GiB free and exactly 25% commit headroom → allowed.
        let gate = make_gate(StubProbe::available(
            HEAVY_RELEASE_FREE_BYTES,
            32 * GIB,
            24 * GIB,
        ));
        assert!(gate.evaluate(&path(), ActionClass::Heavy).is_allowed());
        // One byte short on either axis → refused.
        let gate = make_gate(StubProbe::available(
            HEAVY_RELEASE_FREE_BYTES - 1,
            32 * GIB,
            24 * GIB,
        ));
        assert!(!gate.evaluate(&path(), ActionClass::Heavy).is_allowed());
        let gate = make_gate(StubProbe::available(
            HEAVY_RELEASE_FREE_BYTES,
            32 * GIB,
            24 * GIB + 1,
        ));
        assert!(!gate.evaluate(&path(), ActionClass::Heavy).is_allowed());
    }

    #[test]
    fn watch_tier_does_not_change_behavior() {
        // 12 GiB free → watch, still above the release threshold.
        let gate = make_gate(StubProbe::available(12 * GIB, 32 * GIB, 8 * GIB));
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Allow { tier, .. } => assert_eq!(tier, ResourceTier::Watch),
            other => panic!("expected allow at watch tier, got {other:?}"),
        }
    }

    #[test]
    fn unavailable_readings_fail_closed_for_heavy_only() {
        let gate = make_gate(StubProbe::unavailable());
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Refuse {
                reason,
                tier,
                snapshot,
                ..
            } => {
                assert_eq!(tier, None);
                assert_eq!(snapshot.source_quality, SourceQuality::Unavailable);
                assert!(
                    reason.contains("fail-closed"),
                    "reason must state the fail-closed rule: {reason}"
                );
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        assert!(
            gate.evaluate(&path(), ActionClass::Light).is_allowed(),
            "light actions are never gated, even without readings"
        );
    }

    #[test]
    fn refusal_envelope_renders_readings() {
        let gate = make_gate(StubProbe::available(3 * GIB, 32 * GIB, 28 * GIB));
        let GateDecision::Refuse { snapshot, tier, .. } =
            gate.evaluate(&path(), ActionClass::Heavy)
        else {
            panic!("expected refusal");
        };
        assert_eq!(tier, Some(ResourceTier::ReclaimDirect));
        let json = snapshot.to_json();
        assert_eq!(json["source_quality"], "available");
        assert_eq!(json["volume_free_bytes"], 3 * GIB);
        assert_eq!(json["commit_limit_bytes"], 32 * GIB);
        assert!(json["commit_free_bytes"].as_u64().is_some());
        assert_eq!(gate.last_snapshot(), Some(snapshot));
    }

    // ── hard-limit derivation ───────────────────────────────────────────

    #[test]
    fn job_limits_use_the_eighty_percent_or_reserve_rule() {
        // 30 GiB limit: 80% vs limit−4 GiB = 26 GiB → the 80% arm (integer
        // math, so the expectation mirrors the formula exactly).
        let limits = default_job_limits(&snapshot(40 * GIB, 30 * GIB, 8 * GIB));
        assert_eq!(limits.commit_limit_bytes, Some(30 * GIB / 100 * 80));
        assert!(limits.commit_limit_bytes.unwrap() < 30 * GIB - RUN_COMMIT_RESERVE_BYTES);
        assert_eq!(limits.cpu_rate_percent, Some(RUN_CPU_RATE_PERCENT));
        assert!(limits.active_process.is_some_and(|n| n >= 1));

        // Large limit: the 80% arm wins (100 GiB → 80 vs 96).
        assert_eq!(
            default_job_limits(&snapshot(40 * GIB, 100 * GIB, 8 * GIB)).commit_limit_bytes,
            Some(80 * GIB)
        );
        // Small limit: the reserve arm floors the ceiling at 0, which means
        // "no commit ceiling" (a zero ceiling would block every spawn).
        assert_eq!(
            default_job_limits(&snapshot(40 * GIB, 4 * GIB, GIB)).commit_limit_bytes,
            None
        );
        // Unknown limit → no commit ceiling, never an invented one.
        assert_eq!(
            default_job_limits(&snapshot(40 * GIB, 0, 0)).commit_limit_bytes,
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn system_probe_reads_this_machine() {
        let snapshot = SystemCapacityProbe.probe(Path::new("."));
        assert_eq!(
            snapshot.source_quality,
            SourceQuality::Available,
            "Windows probe must produce readings: {snapshot:?}"
        );
        assert!(snapshot.volume_total_bytes > 0);
        assert!(snapshot.commit_limit_bytes > 0);
        assert!(snapshot.volume_free_bytes <= snapshot.volume_total_bytes);
        assert!(snapshot.describe().contains("volume"));
    }
}

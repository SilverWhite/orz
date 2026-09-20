//! Post-OS-delegation resource observation + volume soft hint
//! (HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20 §1–§6, 0aw).
//!
//! **orz 不做资源准入。** The 2026-09-20 user ruling retired the pre-dispatch
//! admission gate (`HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT` — structurally
//! unsatisfiable on this machine's 83–87 % used baseline) together with the
//! action classifier and both 0z S2 action arms (hard-tier tree kill + reclaim
//! trigger, user ruling ④). Memory/CPU scheduling and limits belong to the
//! operating system (Windows: the kernel-enforced run-level Job Object,
//! `install_global_run_job`; Linux: whatever cgroup/systemd limit the
//! deployment grants — with no enforcement face the run simply records
//! `enforced=false`); disk headroom is a **pre-dispatch mechanical soft hint**.
//! orz dispatches processes, collects results, and relays the OS's answers
//! (`ERROR_DISK_FULL` / `ENOSPC` — the 0z C degradation chain owns the disk
//! full face).
//!
//! Three mechanical pieces, all testable without a real machine:
//!
//! 1. [`CapacityProbe`] — read-only headroom readings for the volume the work
//!    would actually run on (`cwd`; 不换盘不换卷) plus host commit.
//! 2. [`tier_for`] — the ladder tier (`watch → soft → reclaim-direct → hard`)
//!    survives **as an observation label only**: the `host_resource_snapshot`
//!    `tier` field keeps its meaning (deleting it would churn the event family,
//!    verifier and e2e expectations for zero benefit). Nothing hangs actions on
//!    the tier any more.
//! 3. [`ResourceHint::soft_hint`] — the soft hint. When a target volume's free
//!    space is below [`VOLUME_HINT_FREE_BYTES`] the dispatched action STILL
//!    runs; its tool result carries one `[资源软提示]` line (中文短句 ＋ 英文
//!    机械读数, 0af 混排定案). **Once per run per volume** (mechanical dedup —
//!    no spam, same direction as the 0ar 去噪 discipline).
//!
//! 模型面提示的语言形态（0af，2026-09-15 用户定案）：中文定案句 ＋ 英文机械
//! 读数的混排是有意选择。软提示不是拒绝：没有 `resource_insufficient`、没有
//! `Nothing was started`、没有建议引擎——模型读到读数后自行改方案（§2 决策者）。

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use xai_tty_utils::JobLimits;

/// One gibibyte, the unit every threshold in this module is expressed in.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Ladder thresholds (kept as the observation label's scale; design v1.1 §4 —
/// the tier is a readings summary, not a classifier).
pub const WATCH_FREE_BYTES: u64 = 16 * GIB;
pub const SOFT_FREE_BYTES: u64 = 8 * GIB;
pub const RECLAIM_DIRECT_FREE_BYTES: u64 = 5 * GIB;
pub const HARD_FREE_BYTES: u64 = 2 * GIB;
pub const WATCH_COMMIT_USED_PERCENT: u32 = 70;
pub const SOFT_COMMIT_USED_PERCENT: u32 = 85;
pub const HARD_COMMIT_USED_PERCENT: u32 = 95;

/// 卷软水位（设计 v1.1 §4；2026-09-20 用户裁决 = 4 GiB）：目标卷
/// `free < VOLUME_HINT_FREE_BYTES` 时派发前附一条机械软提示——**不阻断、
/// 不改变动作**（§1 一句话设计：磁盘余量降为派发前的机械软提示；真失败面
/// 回到 OS 的 `ERROR_DISK_FULL`／`ENOSPC`，由 0z C 的降级链承担）。
pub const VOLUME_HINT_FREE_BYTES: u64 = 4 * GIB;

/// Hard-limit derivation (design §4.7 / §11 裁决 4): commit = min(80% × limit,
/// limit − 4 GiB); CPU 80%; concurrency = cores.
pub const RUN_COMMIT_LIMIT_PERCENT: u64 = 80;
pub const RUN_COMMIT_RESERVE_BYTES: u64 = 4 * GIB;
pub const RUN_CPU_RATE_PERCENT: u32 = 80;
/// Headroom rule (independent review F-4, 2026-09-12): the ceiling must express
/// what *this run* may still consume, not a fraction of the host limit a busy
/// machine cannot hand out. The binding value is the assembly-time commit
/// headroom minus this reserve, floored so light work can still spawn, and
/// capped by the historical `min(80% × limit, limit − 4 GiB)` arm.
///
/// 0aw（设计 v1.1 §3/§4 裁决点 B，2026-09-20 用户采纳）：该推导**保留、只作
/// 上限、不作拒绝**——内核强制的 run 级 Job Object 是 2026-09-12 Run B 死因
/// 的承重保护面（orz 是 Job 持有者、不在 Job 内），退役准入门不可以连带退役
/// Job 上限。
pub const RUN_COMMIT_HEADROOM_RESERVE_BYTES: u64 = GIB;
pub const RUN_COMMIT_FLOOR_BYTES: u64 = 2 * GIB;
/// Active-process ceiling (independent review F-3): `2 × cores + 8`, at least
/// 16. This ceiling exists to contain a runaway, not to schedule.
pub const RUN_ACTIVE_PROCESS_MULTIPLIER: u32 = 2;
pub const RUN_ACTIVE_PROCESS_BASE: u32 = 8;
pub const RUN_ACTIVE_PROCESS_MIN: u32 = 16;

// ---------------------------------------------------------------------------
// Write targets (目标卷解析 — kept from the former gate module)
// ---------------------------------------------------------------------------

/// The command string of a tool call, when the tool carries one.
fn command_of<'a>(tool: &str, args: &'a serde_json::Value) -> Option<&'a str> {
    if tool != "run_terminal_cmd" {
        return None;
    }
    args.get("command")
        .and_then(|c| c.as_str())
        .or_else(|| args.get("cmd").and_then(|c| c.as_str()))
}

/// The volumes a tool call would **write to** (design §4.1 "目标卷";
/// independent review F-5). Always includes `cwd` (the session volume, and the
/// `不换盘不换卷` default); a command that statically redirects output, changes
/// directory, or re-points a build output directory adds that volume.
///
/// This is a static read of the command string — no shell runs, no environment
/// is resolved. A path that cannot be parsed contributes nothing, and a path
/// that does not exist yet still resolves to its volume (the probe walks up to
/// the nearest existing ancestor).
pub fn write_targets(tool: &str, args: &serde_json::Value, cwd: &Path) -> Vec<PathBuf> {
    let mut targets: Vec<PathBuf> = vec![cwd.to_path_buf()];
    let Some(command) = command_of(tool, args) else {
        return targets;
    };
    for segment in split_segments(&strip_payload_bodies(command)) {
        let tokens = tokenize(&segment);
        if tokens.is_empty() {
            continue;
        }
        for candidate in segment_write_targets(&tokens) {
            if !looks_absolute(&candidate) {
                continue;
            }
            let path = PathBuf::from(candidate.trim_matches(|c| c == '"' || c == '\''));
            if targets.iter().any(|existing| same_path(existing, &path)) {
                continue;
            }
            targets.push(path);
            // Defensive: a pathological command must not turn into an unbounded
            // probe list.
            if targets.len() >= MAX_WRITE_TARGETS {
                return targets;
            }
        }
    }
    targets
}

const MAX_WRITE_TARGETS: usize = 6;

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy().to_ascii_lowercase() == right.to_string_lossy().to_ascii_lowercase()
}

/// Path shapes that can name another volume on this platform.
#[cfg(windows)]
fn looks_absolute(token: &str) -> bool {
    let bytes = token.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || token.starts_with("\\\\")
}

#[cfg(not(windows))]
fn looks_absolute(token: &str) -> bool {
    token.starts_with('/')
}

/// Write-target tokens inside one segment: redirection targets, explicit
/// output-directory flags, directory-changing commands, and build-cache env
/// assignments.
fn segment_write_targets(tokens: &[String]) -> Vec<String> {
    // A quoted wrapper command (`cmd /c "cd /d X && cargo test"`) arrives as one
    // token containing whitespace; re-tokenize those interiors so the flags and
    // operands inside are visible. One pass is enough: the re-tokenized pieces
    // are single words or flag/value pairs.
    let expanded: Vec<String> = tokens
        .iter()
        .flat_map(|token| {
            if token.split_whitespace().count() > 1 {
                tokenize(token)
            } else {
                vec![token.clone()]
            }
        })
        .collect();
    let tokens = expanded.as_slice();
    let mut found = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let raw = token.trim_matches(|c| c == '"' || c == '\'');
        // `> file` / `>> file` (a bare `>` takes the next token).
        if let Some(target) = redirection_target(tokens, index) {
            found.push(target);
            continue;
        }
        let lowered = raw.to_ascii_lowercase();
        let env_key = lowered
            .trim_start_matches("$env:")
            .trim_start_matches('%')
            .trim_end_matches('%');
        if matches!(
            env_key,
            "cargo_target_dir" | "cargo_home" | "npm_config_cache" | "pip_cache_dir"
        ) {
            // `NAME=value`, `NAME = value`, `$env:NAME = 'value'`.
            if let Some((_, value)) = raw.split_once('=') {
                if !value.trim().is_empty() {
                    found.push(value.trim().to_string());
                }
            } else if let Some(next) = tokens[index + 1..].iter().find(|next| next.trim() != "=") {
                found.push(next.clone());
            }
            continue;
        }
        for flag in ["--target-dir", "--out-dir"] {
            if let Some(value) = flag_value(&lowered, flag) {
                found.push(value);
            } else if lowered == flag
                && let Some(next) = tokens.get(index + 1)
            {
                found.push(next.clone());
            }
        }
        if matches!(lowered.as_str(), "--target-dir" | "--out-dir") {
            continue;
        }
        if matches!(
            lowered.as_str(),
            "cd" | "chdir" | "set-location" | "sl" | "pushd"
        ) {
            if let Some(next) = tokens[index + 1..].iter().find(|next| {
                let candidate = next.trim_matches(|c| c == '"' || c == '\'');
                !candidate.starts_with('-') && !candidate.starts_with('/')
            }) {
                found.push(next.clone());
            }
        }
    }
    found
}

/// `--flag=value` → the value (`--flag value` is handled by the caller, which
/// has the token list).
fn flag_value(lowered: &str, flag: &str) -> Option<String> {
    lowered
        .strip_prefix(flag)?
        .strip_prefix('=')
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// The redirection target at `index`, when the token there redirects to a real
/// file (`> NUL` / `/dev/null` / `$null` do not count).
fn redirection_target(tokens: &[String], index: usize) -> Option<String> {
    let token = &tokens[index];
    if token.contains(">&") {
        return None;
    }
    if !token.starts_with('>') {
        return None;
    }
    let trimmed = token.trim_start_matches('>').trim();
    if !trimmed.is_empty() {
        return (!is_null_target(trimmed)).then(|| trimmed.to_string());
    }
    let next = tokens.get(index + 1)?;
    (!is_null_target(next)).then(|| next.clone())
}

/// Split a command line into independent commands (`;`, `&&`, `||`, `|`, `&`,
/// newlines) — the write-target parser's segment boundary.
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

/// Drop the **bodies** of PowerShell here-strings (`@' … '@` / `@" … "@`) and
/// POSIX heredocs (`<<EOF … EOF`), keeping every header line.
///
/// Independent review F-7: an inline script's `x = 1 > 0` was read as a file
/// redirection because the body is not in a quoting state for [`tokenize`]. The
/// bodies are payload, so dropping them cannot hide a real redirection (the
/// header line — where `cat <<EOF > file` lives — is preserved). A body is only
/// dropped when its terminator actually exists later in the command; otherwise
/// the text is left alone, so a stray `<<` can never silently swallow real
/// command lines (the direction that would *miss* a write target).
fn strip_payload_bodies(command: &str) -> String {
    let lines: Vec<&str> = command.split_inclusive('\n').collect();
    let mut out = String::with_capacity(command.len());
    let mut skip_until: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        let body = line.trim_end_matches(['\n', '\r']);
        if let Some(limit) = skip_until {
            if index <= limit {
                continue;
            }
            skip_until = None;
        }
        if let Some((delimiter, here_string)) = payload_start(body)
            && let Some(end) = find_terminator(&lines, index + 1, &delimiter, here_string)
        {
            out.push_str(line);
            skip_until = Some(end);
            continue;
        }
        out.push_str(line);
    }
    out
}

/// The opener on this line: `(terminator-word, is_power_shell_here_string)`.
fn payload_start(line: &str) -> Option<(String, bool)> {
    if let Some(quote) = here_string_quote(line) {
        // PowerShell terminates with the same quote followed by `@`.
        return Some((format!("{quote}@"), true));
    }
    heredoc_delimiter(line).map(|delimiter| (delimiter, false))
}

/// `@'` / `@"` at the end of a line (the PowerShell here-string opener rule).
fn here_string_quote(line: &str) -> Option<char> {
    let trimmed = line.trim_end();
    let mut chars = trimmed.chars().rev();
    match (chars.next(), chars.next()) {
        (Some('\''), Some('@')) => Some('\''),
        (Some('"'), Some('@')) => Some('"'),
        _ => None,
    }
}

/// `<<EOF`, `<< 'EOF'`, `<<-"EOF"` → the delimiter word.
fn heredoc_delimiter(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut index = 0;
    while let Some(position) = line[index..].find("<<") {
        let at = index + position;
        let preceded_by_space = at == 0 || bytes[at - 1].is_ascii_whitespace();
        if preceded_by_space && bytes.get(at + 2) != Some(&b'<') {
            let rest = &line[at + 2..];
            let rest = rest.strip_prefix('-').unwrap_or(rest);
            let rest = rest.trim_start();
            let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"');
            let word = quote.map_or(rest, |q| &rest[q.len_utf8()..]);
            let word: String = word
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !word.is_empty() && !word.chars().all(|c| c.is_ascii_digit()) {
                return Some(word);
            }
        }
        index = at + 2;
    }
    None
}

/// The index of the line that terminates a payload opened at `start`.
fn find_terminator(
    lines: &[&str],
    start: usize,
    delimiter: &str,
    here_string: bool,
) -> Option<usize> {
    for (offset, line) in lines.iter().enumerate().skip(start) {
        let body = line.trim_end_matches(['\n', '\r']);
        let matched = if here_string {
            body.trim_start().starts_with(delimiter)
        } else {
            body.trim_start_matches('\t').trim() == delimiter
        };
        if matched {
            return Some(offset);
        }
    }
    None
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

fn is_null_target(target: &str) -> bool {
    let normalized = target
        .trim_matches(|c| c == '"' || c == '\'')
        .to_ascii_lowercase();
    NULL_TARGETS.contains(&normalized.as_str())
}

/// Redirection targets that are not files.
const NULL_TARGETS: &[&str] = &["$null", "/dev/null", "nul", "none"];

// ---------------------------------------------------------------------------
// Readings
// ---------------------------------------------------------------------------

/// Quality of a reading set — observation-only now (the former fail-closed
/// admission is retired; an unreadable probe simply produces no hint and the
/// `unknown` tier label).
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
    /// Live volume free bytes (the volume the work would run on).
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
    /// An unreadable probe result (observation label `unknown`).
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

    /// JSON rendering for the observation envelope (mechanical readings,
    /// never prose).
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
/// Linux: `statvfs` + `/proc/meminfo` (`CommitLimit` / `Committed_AS` —
/// observation fields only since 0aw: under the default overcommit policy
/// neither is an enforcement threshold).
/// Anything the platform cannot answer comes back as
/// [`SourceQuality::Unavailable`] — never a guessed number.
#[derive(Debug, Default)]
pub struct SystemCapacityProbe;

impl CapacityProbe for SystemCapacityProbe {
    fn probe(&self, path: &Path) -> HostCapacitySnapshot {
        match (probe_volume_nearest(path), probe_commit()) {
            (Some((free, total)), Some((limit, used))) => HostCapacitySnapshot {
                collected_at_ms: now_ms(),
                volume_free_bytes: free,
                volume_total_bytes: total,
                commit_limit_bytes: limit,
                commit_used_bytes: used,
                source_quality: SourceQuality::Available,
            },
            _ => HostCapacitySnapshot::unavailable(),
        }
    }
}

/// Probe the volume holding `path`, walking up to the nearest existing
/// ancestor first. A write target that does not exist yet (`--target-dir` into a
/// fresh directory) must still be judged by its volume, not reported as an
/// unreadable machine (independent review F-5).
fn probe_volume_nearest(path: &Path) -> Option<(u64, u64)> {
    let mut candidate = Some(path);
    while let Some(current) = candidate {
        if let Some(reading) = probe_volume(current) {
            return Some(reading);
        }
        candidate = current.parent().filter(|parent| *parent != current);
    }
    None
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
// Tier ladder (observation label only) and the soft hint
// ---------------------------------------------------------------------------

/// Where the machine currently sits on the ladder (design §4.1 table).
/// **Observation label only** — the `host_resource_snapshot` `tier` field and
/// the soft-hint context keep using it; no action hangs on the tier any more
/// (the hard-tier tree kill and the reclaim trigger were retired with the
/// admission gate, user ruling ④ 2026-09-20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceTier {
    Normal,
    Watch,
    Soft,
    ReclaimDirect,
    Hard,
    /// Readings could not be taken. Machine-readable key `unknown` — never
    /// folded into `hard` (independent review F-6): an observation label must
    /// not look like a full disk.
    Unknown,
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
            ResourceTier::Unknown => "unknown",
        }
    }
}

/// Derive the ladder tier from a reading. The tier is *reported* only.
pub fn tier_for(snapshot: &HostCapacitySnapshot) -> ResourceTier {
    if snapshot.source_quality == SourceQuality::Unavailable {
        return ResourceTier::Unknown;
    }
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

/// One volume's reading inside a (possibly multi-volume) dispatch reading.
#[derive(Clone, Debug)]
pub struct VolumeReading {
    /// The path whose volume was probed (the work's write target, design §4.1
    /// "目标卷").
    pub path: String,
    pub snapshot: HostCapacitySnapshot,
}

impl VolumeReading {
    /// `{ path, readings }` — mechanical, never prose.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "path": self.path,
            "readings": self.snapshot.to_json(),
        })
    }
}

/// The per-dispatch observation + soft-hint face (the former admission gate).
/// One probe handle, one last-reading cell for the observation face, and the
/// per-run per-volume hint dedup set.
pub struct ResourceHint {
    probe: Arc<dyn CapacityProbe>,
    last: Mutex<Option<HostCapacitySnapshot>>,
    hinted: Mutex<HashSet<String>>,
}

impl ResourceHint {
    /// Build the hint face around a probe.
    pub fn new(probe: Arc<dyn CapacityProbe>) -> Self {
        Self {
            probe,
            last: Mutex::new(None),
            hinted: Mutex::new(HashSet::new()),
        }
    }

    /// The probe handle (observation / re-reads).
    pub fn probe(&self) -> Arc<dyn CapacityProbe> {
        self.probe.clone()
    }

    /// The most recent binding reading this face took.
    pub fn last_snapshot(&self) -> Option<HostCapacitySnapshot> {
        *self.last.lock().unwrap()
    }

    /// Read **every** volume the action would write to (design §4.1 "目标卷";
    /// independent review F-5). The binding reading (the one with the least
    /// headroom) is what the observation face publishes. Readings never gate —
    /// they feed the tier-change snapshot and the soft hint.
    pub fn read_for_volumes(&self, volumes: &[PathBuf]) -> Vec<VolumeReading> {
        let readings: Vec<VolumeReading> = volumes
            .iter()
            .map(|path| VolumeReading {
                path: path.display().to_string(),
                snapshot: self.probe.probe(path),
            })
            .collect();
        let binding = readings
            .iter()
            .enumerate()
            .min_by_key(|(_, reading)| {
                (
                    reading.snapshot.source_quality != SourceQuality::Available,
                    reading.snapshot.volume_free_bytes,
                )
            })
            .map(|(index, _)| index)
            .unwrap_or(0);
        *self.last.lock().unwrap() = Some(readings[binding].snapshot);
        readings
    }

    /// The pre-dispatch soft hint for these readings, or `None` when every
    /// target volume is at or above [`VOLUME_HINT_FREE_BYTES`], a reading is
    /// unavailable (no hint without numbers — never a fabricated warning), or
    /// every short volume was already hinted this run (机械去重：每 run 每卷
    /// 至多一次；hinting marks the volume even when several volumes are short).
    pub fn soft_hint(&self, readings: &[VolumeReading]) -> Option<String> {
        let short: Vec<&VolumeReading> = readings
            .iter()
            .filter(|reading| {
                reading.snapshot.source_quality == SourceQuality::Available
                    && reading.snapshot.volume_free_bytes < VOLUME_HINT_FREE_BYTES
            })
            .collect();
        if short.is_empty() {
            return None;
        }
        let mut hinted = self.hinted.lock().unwrap();
        let fresh: Vec<&&VolumeReading> = short
            .iter()
            .filter(|reading| hinted.insert(volume_hint_key(&reading.path)))
            .collect();
        if fresh.is_empty() {
            return None;
        }
        let details: Vec<String> = fresh
            .into_iter()
            .map(|reading| {
                format!(
                    "volume {} free {} of {} < {} hint threshold",
                    reading.path,
                    gib(reading.snapshot.volume_free_bytes),
                    gib(reading.snapshot.volume_total_bytes),
                    gib(VOLUME_HINT_FREE_BYTES)
                )
            })
            .collect();
        // 0af 混排定案：中文短句 ＋ 英文机械读数；只报事实，无建议。
        Some(format!(
            "[资源软提示] 目标卷余量低于软水位（{}），动作照常执行。{}",
            gib(VOLUME_HINT_FREE_BYTES),
            details.join("; ")
        ))
    }
}

/// Run-level hard ceilings derived from a reading (design §4.7 / §11 裁决 4).
///
/// `commit = min(cap, headroom − 1 GiB)` where the cap is the historical
/// `min(80% × limit, limit − 4 GiB)` and the floor keeps light work able to
/// spawn; CPU 80%; active processes `2 × cores + 8` (≥ 16). Unknown commit
/// limit → no commit ceiling (recorded rather than silently invented).
///
/// 0aw（裁决点 B）：推导**保留、只作上限**——超限的答复由内核给出
/// （`JOB_OBJECT_LIMIT_JOB_MEMORY`：进程试图提交超过作业总额的内存时**它**
/// 分配失败），orz 只如实转达，不再有 orz 侧预检拒绝。
pub fn default_job_limits(snapshot: &HostCapacitySnapshot) -> JobLimits {
    let commit_limit_bytes = (snapshot.commit_limit_bytes > 0)
        .then(|| {
            let eighty = snapshot.commit_limit_bytes / 100 * RUN_COMMIT_LIMIT_PERCENT;
            let reserved = snapshot
                .commit_limit_bytes
                .saturating_sub(RUN_COMMIT_RESERVE_BYTES);
            let cap = eighty.min(reserved);
            // The binding value answers "how much may this run still commit?" —
            // assembly-time headroom minus the reserve, never below the floor,
            // never above the cap (independent review F-4: the cap alone sat
            // above the point where this host actually died).
            let binding = snapshot
                .commit_free_bytes()
                .map(|headroom| {
                    headroom
                        .saturating_sub(RUN_COMMIT_HEADROOM_RESERVE_BYTES)
                        .max(RUN_COMMIT_FLOOR_BYTES)
                })
                .unwrap_or(cap);
            binding.min(cap)
        })
        // The reserve rule can floor the ceiling at zero (a machine whose whole
        // commit limit is the reserve): a zero ceiling would make the job
        // unable to start anything, so it means "no commit ceiling" instead.
        .filter(|value| *value > 0);
    JobLimits {
        commit_limit_bytes,
        active_process: Some(run_active_process_limit()),
        cpu_rate_percent: Some(RUN_CPU_RATE_PERCENT),
    }
}

/// The run's active-process ceiling: `2 × cores + 8`, at least 16.
pub fn run_active_process_limit() -> u32 {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    cores
        .saturating_mul(RUN_ACTIVE_PROCESS_MULTIPLIER)
        .saturating_add(RUN_ACTIVE_PROCESS_BASE)
        .max(RUN_ACTIVE_PROCESS_MIN)
}

/// Render bytes as GiB with two decimals (audit text only).
pub fn gib(bytes: u64) -> String {
    format!("{:.2} GiB", bytes as f64 / GIB as f64)
}

/// The per-run hint dedup key for "每 run 每卷一次" (设计 v1.1 §5). On
/// Windows the volume identity is the path prefix (`D:` / `\\server\share`)
/// — two write-target paths on one volume share a key, so a volume hints at
/// most once per run regardless of how the command spelled the path. A
/// relative path (no prefix; it lives on the cwd's volume) falls back to the
/// path string. Non-Windows has no mount identity without statfs — the path
/// string is the key (documented limitation: two mounts under `/` dedup as
/// one; the session volume's cwd is in every target set, so the no-spam goal
/// still holds).
fn volume_hint_key(path: &str) -> String {
    #[cfg(windows)]
    {
        use std::path::Component;
        if let Some(Component::Prefix(prefix)) = Path::new(path).components().next() {
            return prefix.as_os_str().to_string_lossy().to_ascii_lowercase();
        }
    }
    path.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct StubProbe {
        snapshot: HostCapacitySnapshot,
    }

    /// A probe that answers differently per path — the multi-volume seam.
    struct PerPathProbe {
        cwd: PathBuf,
        cwd_snapshot: HostCapacitySnapshot,
        other_snapshot: HostCapacitySnapshot,
    }

    impl CapacityProbe for PerPathProbe {
        fn probe(&self, path: &Path) -> HostCapacitySnapshot {
            if path == self.cwd {
                self.cwd_snapshot
            } else {
                self.other_snapshot
            }
        }
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

    fn path() -> PathBuf {
        PathBuf::from(".")
    }

    // ── tiers（观测标签） ────────────────────────────────────────────────

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
        // Unreadable readings are their own tier, never `hard` (review F-6).
        assert_eq!(
            tier_for(&HostCapacitySnapshot::unavailable()),
            ResourceTier::Unknown
        );
    }

    // ── soft hint（观测 ＋ 软提示；0aw 定案面） ──────────────────────────

    #[test]
    fn soft_hint_fires_below_the_water_line_and_dedups_per_volume() {
        // 3 GiB free < 4 GiB ⇒ one hint, once per volume per run.
        let hint = ResourceHint::new(StubProbe::available(3 * GIB, 32 * GIB, 4 * GIB));
        let readings = hint.read_for_volumes(&[path()]);
        let line = hint.soft_hint(&readings).expect("hint below the line");
        assert!(line.contains("[资源软提示]"), "{line}");
        assert!(line.contains("动作照常执行"), "hint never blocks: {line}");
        assert!(
            line.contains("volume . free 3.00 GiB of 100.00 GiB < 4.00 GiB"),
            "mechanical readings ride the line: {line}"
        );
        // Second dispatch on the same volume: deduped, no second hint.
        let readings = hint.read_for_volumes(&[path()]);
        assert!(
            hint.soft_hint(&readings).is_none(),
            "per-run per-volume once"
        );
    }

    #[test]
    fn soft_hint_is_silent_at_or_above_the_water_line_and_without_readings() {
        // 4 GiB exactly = at the threshold ⇒ no hint (strictly below fires).
        let hint = ResourceHint::new(StubProbe::available(
            VOLUME_HINT_FREE_BYTES,
            32 * GIB,
            4 * GIB,
        ));
        let readings = hint.read_for_volumes(&[path()]);
        assert!(hint.soft_hint(&readings).is_none());
        // Unavailable readings produce no hint either (never fabricate).
        let blind = ResourceHint::new(StubProbe::unavailable());
        let readings = blind.read_for_volumes(&[path()]);
        assert!(blind.soft_hint(&readings).is_none());
        // Observations still update the last-reading cell.
        assert_eq!(
            blind.last_snapshot().map(|s| s.source_quality),
            Some(SourceQuality::Unavailable)
        );
    }

    #[test]
    fn any_short_write_target_volume_hints_once_per_volume() {
        let cwd = PathBuf::from(".");
        let elsewhere = PathBuf::from("..");
        let probe = Arc::new(PerPathProbe {
            cwd: cwd.clone(),
            cwd_snapshot: snapshot(40 * GIB, 32 * GIB, 4 * GIB),
            other_snapshot: snapshot(3 * GIB, 32 * GIB, 4 * GIB),
        });
        let hint = ResourceHint::new(probe);
        // The session volume alone stays silent.
        let readings = hint.read_for_volumes(&[cwd.clone()]);
        assert!(hint.soft_hint(&readings).is_none());
        // With the second write target in play the hint names that volume…
        let readings = hint.read_for_volumes(&[cwd.clone(), elsewhere.clone()]);
        let line = hint.soft_hint(&readings).expect("short volume hinted");
        assert!(line.contains(".."), "hint names the short volume: {line}");
        assert_eq!(readings.len(), 2, "every probed volume is read");
        // …and the same volume is not hinted twice, while the first dispatch's
        // cwd (never short) stays eligible for a future hint.
        let readings = hint.read_for_volumes(&[cwd, elsewhere]);
        assert!(hint.soft_hint(&readings).is_none(), "per-volume dedup");
    }

    // ── hard-limit derivation（保留推导、只作上限——裁决点 B） ────────────

    #[test]
    fn job_limits_use_the_eighty_percent_or_reserve_rule() {
        // 30 GiB limit with 8 GiB already committed: the cap is 80% = 24 GiB,
        // and the binding value is the assembly-time headroom minus 1 GiB
        // reserve = 21 GiB (review F-4 — a busy host cannot hand out the cap).
        let limits = default_job_limits(&snapshot(40 * GIB, 30 * GIB, 8 * GIB));
        assert_eq!(
            limits.commit_limit_bytes,
            Some(30 * GIB - 8 * GIB - RUN_COMMIT_HEADROOM_RESERVE_BYTES)
        );
        assert!(limits.commit_limit_bytes.unwrap() < 30 * GIB / 100 * 80);
        assert_eq!(limits.cpu_rate_percent, Some(RUN_CPU_RATE_PERCENT));
        assert_eq!(limits.active_process, Some(run_active_process_limit()));

        // An idle host: the cap is the binding value (headroom − 1 GiB = 91 GiB
        // is above the 80% arm at 80 GiB).
        assert_eq!(
            default_job_limits(&snapshot(40 * GIB, 100 * GIB, 8 * GIB)).commit_limit_bytes,
            Some(80 * GIB)
        );
        // A host that is nearly out of headroom: the ceiling collapses to the
        // floor instead of the cap — a tool failure, not a dead machine.
        assert_eq!(
            default_job_limits(&snapshot(40 * GIB, 30 * GIB, 27 * GIB)).commit_limit_bytes,
            Some(RUN_COMMIT_FLOOR_BYTES)
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
        // The active-process ceiling leaves room for cargo's own default
        // parallelism plus its children (review F-3).
        let cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);
        assert_eq!(
            run_active_process_limit(),
            (cores * RUN_ACTIVE_PROCESS_MULTIPLIER + RUN_ACTIVE_PROCESS_BASE)
                .max(RUN_ACTIVE_PROCESS_MIN)
        );
        assert!(run_active_process_limit() > cores);
    }

    #[cfg(windows)]
    #[test]
    fn same_volume_paths_hint_once_per_run() {
        // Two distinct absolute paths on ONE volume (`D:\a`, `D:\b`) share the
        // volume dedup key — the volume hints once, naming the first fresh
        // path, and a later dispatch on the sibling path stays silent
        // (设计 v1.1 §5「每 run 每卷一次」：键是卷不是路径串).
        let probe = Arc::new(PerPathProbe {
            cwd: PathBuf::from(r"D:\a"),
            cwd_snapshot: snapshot(3 * GIB, 32 * GIB, 4 * GIB),
            other_snapshot: snapshot(3 * GIB, 32 * GIB, 4 * GIB),
        });
        let hint = ResourceHint::new(probe);
        let readings = hint.read_for_volumes(&[PathBuf::from(r"D:\a"), PathBuf::from(r"D:\b")]);
        let line = hint.soft_hint(&readings).expect("short volume hinted");
        assert!(line.contains(r"D:\a"), "names the first fresh path: {line}");
        assert!(
            !line.contains(r"D:\b"),
            "the sibling path is the same volume, not a second hint: {line}"
        );
        let readings = hint.read_for_volumes(&[PathBuf::from(r"D:\b")]);
        assert!(
            hint.soft_hint(&readings).is_none(),
            "the volume was already hinted this run"
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

    // ── write targets (review F-5) ──────────────────────────────────────

    #[cfg(windows)]
    #[test]
    fn write_targets_follow_the_real_corpus_shapes() {
        let cwd = PathBuf::from(r"D:\CLI");
        let cases: [(&str, &[&str]); 5] = [
            ("cargo build --release", &[]),
            (r"cd D:\CLI\orz; cargo test -p orz-host", &[r"D:\CLI\orz"]),
            (
                r"cargo build --target-dir D:\CLI\.gsa\cargo-target",
                &[r"D:\CLI\.gsa\cargo-target"],
            ),
            (
                r"cargo build --target-dir=D:\other\target",
                &[r"D:\other\target"],
            ),
            (
                "cmd /c \"cd /d C:\\builds\\orz && cargo test\"",
                &[r"C:\builds\orz"],
            ),
        ];
        for (command, expected) in cases {
            let args = serde_json::json!({ "command": command });
            let targets = write_targets("run_terminal_cmd", &args, &cwd);
            assert_eq!(targets[0], cwd, "cwd is always the first target");
            for want in expected {
                assert!(
                    targets.iter().any(|target| target == &PathBuf::from(want)),
                    "command {command:?} must include {want}: {targets:?}"
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn write_targets_ignore_reads_and_null_redirection() {
        let cwd = PathBuf::from(r"D:\CLI");
        for command in [
            r"grep -n cargo D:\other\Cargo.toml",
            r"cargo test 2>$null",
            r"dir > NUL",
            r"echo hi >> nul",
        ] {
            let args = serde_json::json!({ "command": command });
            assert_eq!(
                write_targets("run_terminal_cmd", &args, &cwd),
                vec![cwd.clone()],
                "no real write target in {command:?}"
            );
        }
        // A non-terminal tool never carries a command string.
        assert_eq!(
            write_targets("read_file", &serde_json::json!({"target_file": "x"}), &cwd),
            vec![cwd]
        );
    }

    #[cfg(windows)]
    #[test]
    fn write_targets_capture_redirection_and_build_cache_env() {
        let cwd = PathBuf::from(r"D:\CLI");
        let args = serde_json::json!({
            "command": r#"$env:CARGO_TARGET_DIR = "C:\cache\target"; echo hi > D:\logs\out.txt"#
        });
        let targets = write_targets("run_terminal_cmd", &args, &cwd);
        assert!(
            targets.contains(&PathBuf::from(r"C:\cache\target")),
            "{targets:?}"
        );
        assert!(
            targets.contains(&PathBuf::from(r"D:\logs\out.txt")),
            "{targets:?}"
        );
    }

    // ── payload bodies (review F-7)：写目标解析的 here-string/heredoc 语义 ──

    #[test]
    fn here_string_bodies_are_not_command_syntax() {
        let cwd = PathBuf::from(".");
        let command = "cd D:\\CLI; @'\nimport json\nx = 1 > 0\n'@ | python -";
        let args = serde_json::json!({ "command": command });
        let targets = write_targets("run_terminal_cmd", &args, &cwd);
        // The `cd D:\CLI` prologue is a real write target; the `x = 1 > 0`
        // comparison inside the here-string body must NOT add one.
        assert_eq!(
            targets,
            vec![cwd.clone(), PathBuf::from("D:\\CLI")],
            "body comparisons are payload, not redirections: {targets:?}"
        );
        // The header line still counts: a real redirection next to the opener
        // yields the target.
        let args = serde_json::json!({ "command": "cat > D:\\out.txt <<EOF\nhello\nEOF" });
        let targets = write_targets("run_terminal_cmd", &args, &cwd);
        assert!(
            targets.iter().any(|t| t == &PathBuf::from(r"D:\out.txt")),
            "{targets:?}"
        );
    }

    #[test]
    fn heredoc_bodies_are_not_command_syntax() {
        let command = "python - <<'PY'\nprint(1 > 0)\nPY\necho done";
        let args = serde_json::json!({ "command": command });
        assert_eq!(
            write_targets("run_terminal_cmd", &args, &PathBuf::from(".")),
            vec![PathBuf::from(".")],
            "the heredoc body is payload"
        );
    }

    #[test]
    fn unterminated_payload_markers_do_not_swallow_commands() {
        // `<<` without a terminator must leave the text alone, so the redirect
        // after it is still parsed (false negatives are the dangerous
        // direction).
        let args = serde_json::json!({ "command": "echo a << b;\ncargo build > D:\\build.log" });
        let targets = write_targets("run_terminal_cmd", &args, &PathBuf::from("."));
        assert!(
            targets.iter().any(|t| t == &PathBuf::from(r"D:\build.log")),
            "{targets:?}"
        );
    }
}

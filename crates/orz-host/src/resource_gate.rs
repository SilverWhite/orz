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
//!
//! 模型面 reason 的语言形态（0af，2026-09-15 用户定案）：**中文定案句 ＋ 英文
//! 机械读数的混排是有意选择**——定案句用用户工作语言让模型不必解码术语即可
//! 行动，读数（短少明细／`readings` 信封）保持英文机械原样以便逐字核对。
//! 不做建议引擎／恢复指引（定案边界）。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use xai_tty_utils::JobLimits;

/// One gibibyte, the unit every threshold in this module is expressed in.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// Stable refusal code (design §5: pre-issue family, same envelope shape as the
/// budget / candidate / order refusals).
pub const CODE_RESOURCE_INSUFFICIENT: &str = "resource_insufficient";

// 0af（2026-09-15 用户定案）：拦截理由的模型面定案句，按**实际耗尽轴**标注
// （内存＝commit、储存＝卷余量，双轴同短时合并标注）；机械读数以英文随附
// （见模块头「混排是有意选择」）。Unknown 档（读数不可得、fail-closed 拒绝）
// 用**变体句**——读数缺席时不得断言「即将耗尽」，防不实陈述。
const DENIAL_HEADLINE_STORAGE: &str = "宿主机储存资源即将耗尽，无法新增派发，请寻找其他方案";
const DENIAL_HEADLINE_MEMORY: &str = "宿主机内存资源即将耗尽，无法新增派发，请寻找其他方案";
const DENIAL_HEADLINE_BOTH: &str = "宿主机内存/储存资源即将耗尽，无法新增派发，请寻找其他方案";
const DENIAL_HEADLINE_UNREADABLE: &str =
    "主机资源读数不可得，无法确认余量，已按 fail-closed 规则拒绝新增派发，请寻找其他方案";

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
/// Headroom rule (independent review F-4, 2026-09-12): the ceiling must express
/// what *this run* may still consume, not a fraction of the host limit a busy
/// machine cannot hand out. The binding value is the assembly-time commit
/// headroom minus this reserve, floored so light work can still spawn, and
/// capped by the historical `min(80% × limit, limit − 4 GiB)` arm.
pub const RUN_COMMIT_HEADROOM_RESERVE_BYTES: u64 = GIB;
pub const RUN_COMMIT_FLOOR_BYTES: u64 = 2 * GIB;
/// Active-process ceiling (independent review F-3): `2 × cores + 8`, at least
/// 16. The value the user's ruling originally fixed (cores) is exactly cargo's
/// default `-j`, so cargo + its rustc children + linkers + test harnesses would
/// cross it and the kernel would refuse a legitimate `CreateProcess`. This
/// ceiling exists to contain a runaway, not to schedule.
pub const RUN_ACTIVE_PROCESS_MULTIPLIER: u32 = 2;
pub const RUN_ACTIVE_PROCESS_BASE: u32 = 8;
pub const RUN_ACTIVE_PROCESS_MIN: u32 = 16;

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
    let Some(command) = command_of(tool, args) else {
        return ActionClass::Light;
    };
    classify_command(command)
}

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

/// Classify a shell command string (static; no shell actually runs).
pub fn classify_command(command: &str) -> ActionClass {
    for segment in split_segments(&strip_payload_bodies(command)) {
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

/// Drop the **bodies** of PowerShell here-strings (`@' … '@` / `@" … "@`) and
/// POSIX heredocs (`<<EOF … EOF`), keeping every header line.
///
/// Independent review F-7: an inline script's `x = 1 > 0` was read as a file
/// redirection because the body is not in a quoting state for [`tokenize`]. The
/// bodies are payload, so dropping them cannot hide a real redirection (the
/// header line — where `cat <<EOF > file` lives — is preserved). A body is only
/// dropped when its terminator actually exists later in the command; otherwise
/// the text is left alone, so a stray `<<` can never silently swallow real
/// command lines (the direction that would *miss* heavy work).
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
        match (probe_volume_nearest(path), probe_commit()) {
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
    /// Readings could not be taken. Machine-readable key `unknown` — never
    /// folded into `hard` (independent review F-6): S2 hangs reclaim/tree-kill
    /// decisions on the tier, and a probe failure must not look like a full
    /// disk.
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

/// Derive the ladder tier from a reading. The tier is *reported* (and drives
/// the refusal reason); the upper-tier actions (reclaim, tree kill) are S2.
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

/// One volume's reading inside a (possibly multi-volume) decision.
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

/// The gate's verdict.
#[derive(Clone, Debug)]
pub enum GateDecision {
    /// Dispatch may proceed.
    Allow {
        class: ActionClass,
        tier: ResourceTier,
        snapshot: HostCapacitySnapshot,
    },
    /// Dispatch is refused before anything starts. `snapshot` is the binding
    /// (worst-headroom) reading; `volumes` carries every probed volume.
    Refuse {
        code: &'static str,
        reason: String,
        class: ActionClass,
        tier: ResourceTier,
        snapshot: HostCapacitySnapshot,
        volumes: Vec<VolumeReading>,
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
    /// volume holding `volume_path`. Single-volume convenience wrapper over
    /// [`Self::evaluate_for_volumes`].
    pub fn evaluate(&self, volume_path: &Path, class: ActionClass) -> GateDecision {
        self.evaluate_for_volumes(std::slice::from_ref(&volume_path.to_path_buf()), class)
    }

    /// Decide whether `class` may be dispatched, probing **every** volume the
    /// action would write to (design §4.1 "目标卷"; independent review F-5).
    ///
    /// All volumes must clear the release thresholds — the action is only as
    /// safe as its worst write target — and the binding reading (the one with
    /// the least headroom) is what the refusal carries as `snapshot`.
    pub fn evaluate_for_volumes(&self, volumes: &[PathBuf], class: ActionClass) -> GateDecision {
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
        let snapshot = readings[binding].snapshot;
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
        if readings
            .iter()
            .any(|reading| reading.snapshot.source_quality == SourceQuality::Unavailable)
        {
            let unreadable: Vec<String> = readings
                .iter()
                .filter(|reading| reading.snapshot.source_quality == SourceQuality::Unavailable)
                .map(|reading| reading.path.clone())
                .collect();
            return GateDecision::Refuse {
                code: CODE_RESOURCE_INSUFFICIENT,
                reason: format!(
                    "{DENIAL_HEADLINE_UNREADABLE}（unreadable targets: {}）。Nothing was started.",
                    unreadable.join(", ")
                ),
                class,
                tier: ResourceTier::Unknown,
                snapshot,
                volumes: readings,
            };
        }
        let tier = tier_for(&snapshot);
        let short_volumes: Vec<&VolumeReading> = readings
            .iter()
            .filter(|reading| reading.snapshot.volume_free_bytes < HEAVY_RELEASE_FREE_BYTES)
            .collect();
        let free_ok = short_volumes.is_empty();
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
            let listed: Vec<String> = short_volumes
                .iter()
                .map(|reading| {
                    format!(
                        "{} free {} < {} required",
                        reading.path,
                        gib(reading.snapshot.volume_free_bytes),
                        gib(HEAVY_RELEASE_FREE_BYTES)
                    )
                })
                .collect();
            shortfalls.push(listed.join("; "));
        }
        if !commit_headroom_ok {
            // 0af 审查补充：headroom 百分比一律**一位小数、向下取整**——整数
            // 截断曾把实值 24.4% 显示成「25% < 25% required」的字面自相矛盾；
            // 向下取整保证显示值 ≤ 真值，故拒绝理由里不可能出现「25.0% < 25%
            // required」形态。字节直读同行随附。
            let commit_free = snapshot.commit_free_bytes().unwrap_or(0);
            shortfalls.push(format!(
                "commit headroom {} ({} of {}) < {}% required",
                headroom_percent_floor(commit_free, snapshot.commit_limit_bytes),
                gib(commit_free),
                gib(snapshot.commit_limit_bytes),
                HEAVY_RELEASE_COMMIT_HEADROOM_PERCENT
            ));
        }
        // 0af：按实际耗尽轴标注定案句（储存＝卷余量、内存＝commit）。
        let headline = match (!free_ok, !commit_headroom_ok) {
            (true, true) => DENIAL_HEADLINE_BOTH,
            (false, true) => DENIAL_HEADLINE_MEMORY,
            (true, false) => DENIAL_HEADLINE_STORAGE,
            (false, false) => unreachable!("both axes cleared — the allow arm returned above"),
        };
        GateDecision::Refuse {
            code: CODE_RESOURCE_INSUFFICIENT,
            reason: format!(
                "{headline}（tier {}；{}；readings: {}）。Nothing was started.",
                tier.as_str(),
                shortfalls.join("; "),
                snapshot.describe()
            ),
            class,
            tier,
            snapshot,
            volumes: readings,
        }
    }
}

/// Run-level hard ceilings derived from a reading (design §4.7 / §11 裁决 4).
///
/// `commit = min(cap, headroom − 1 GiB)` where the cap is the historical
/// `min(80% × limit, limit − 4 GiB)` and the floor keeps light work able to
/// spawn; CPU 80%; active processes `2 × cores + 8` (≥ 16). Unknown commit
/// limit → no commit ceiling (the gate, not the kernel, is then the only
/// defence — recorded rather than silently invented).
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

/// Commit headroom as a percentage string with exactly one decimal, **floored**
/// (never rounded up). The refusal copy must never display a headroom that is
/// at or above the required threshold: flooring keeps the displayed value ≤ the
/// true value, so `25.0% < 25% required` (the literal self-contradiction the
/// integer-truncation copy once produced) cannot be rendered. Integer math
/// throughout — no float formatting in the mechanical face.
fn headroom_percent_floor(free: u64, limit: u64) -> String {
    if limit == 0 {
        return "0.0%".to_string();
    }
    let tenths = (u128::from(free) * 1000 / u128::from(limit)) as u64;
    format!("{}.{}%", tenths / 10, tenths % 10)
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
        // Unreadable readings are their own tier, never `hard` (review F-6).
        assert_eq!(
            tier_for(&HostCapacitySnapshot::unavailable()),
            ResourceTier::Unknown
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
                assert_eq!(tier, ResourceTier::Soft);
                assert!(
                    reason.contains("commit headroom"),
                    "reason must name the shortfall: {reason}"
                );
                assert!(
                    reason.contains("Nothing was started"),
                    "refusal must state that nothing ran: {reason}"
                );
                // 0af：定案句按轴标注——只有内存（commit）轴短 ⇒ 不带「储存」。
                assert!(
                    reason.contains("宿主机内存资源即将耗尽"),
                    "memory-axis headline missing: {reason}"
                );
                assert!(
                    !reason.contains("储存资源即将耗尽"),
                    "storage axis is not short here: {reason}"
                );
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        // Free space below the release threshold, commit healthy.
        let gate = make_gate(StubProbe::available(7 * GIB, 32 * GIB, 4 * GIB));
        assert!(!gate.evaluate(&path(), ActionClass::Heavy).is_allowed());
    }

    /// 0af 审查补充钉子：headroom 百分比一位小数、向下取整——整数截断曾把
    /// 实值 24.4% 显示成「commit headroom 25% < 25% required」的字面自相
    /// 矛盾（真机 run 实录）。构造实值 24.375%（used 整数截断 75% ⇒ 旧文案
    /// 恰好显示 25%）的读数：新文案必须显示 24.3% 且不出现「25% <」形态。
    #[test]
    fn refusal_headroom_percent_never_self_contradicts() {
        let used = GIB * 242 / 10; // 24.2 GiB of a 32 GiB limit
        let gate = make_gate(StubProbe::available(40 * GIB, 32 * GIB, used));
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Refuse { reason, .. } => {
                assert!(
                    reason.contains("24.3%"),
                    "true headroom 24.375% must floor-display as 24.3%: {reason}"
                );
                assert!(
                    !reason.contains("25% <"),
                    "the displayed headroom may never reach the required line: {reason}"
                );
                // 字节直读同行随附。
                assert!(reason.contains("of 32.00 GiB"), "{reason}");
            }
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    /// 0af：双轴同短 ⇒ 定案句合并标注「内存/储存」。
    #[test]
    fn denial_headline_names_every_short_axis() {
        let gate = make_gate(StubProbe::available(3 * GIB, 32 * GIB, 30 * GIB));
        match gate.evaluate(&path(), ActionClass::Heavy) {
            GateDecision::Refuse { reason, .. } => {
                assert!(
                    reason.contains("宿主机内存/储存资源即将耗尽"),
                    "both axes short must merge into one headline: {reason}"
                );
            }
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    /// 0af：`headroom_percent_floor` 的取整方向钉子——四舍五入陷阱值
    /// （真值 24.99%）必须显示 24.9%，不得进位成 25.0%。
    #[test]
    fn headroom_percent_floor_never_rounds_up() {
        assert_eq!(headroom_percent_floor(2_499, 10_000), "24.9%");
        assert_eq!(headroom_percent_floor(2_500, 10_000), "25.0%");
        assert_eq!(headroom_percent_floor(0, 10_000), "0.0%");
        assert_eq!(headroom_percent_floor(9_999, 10_000), "99.9%");
        assert_eq!(headroom_percent_floor(10_000, 10_000), "100.0%");
        assert_eq!(headroom_percent_floor(1, 0), "0.0%", "unknown limit");
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
                assert_eq!(tier, ResourceTier::Unknown);
                assert_eq!(snapshot.source_quality, SourceQuality::Unavailable);
                assert!(
                    reason.contains("fail-closed"),
                    "reason must state the fail-closed rule: {reason}"
                );
                // 0af 审查补充：读数不可得的变体句——不得断言「即将耗尽」
                //（读数缺席时这是不实陈述），须如实说「无法确认余量」。
                assert!(
                    reason.contains("读数不可得，无法确认余量"),
                    "unknown tier must use the honest variant copy: {reason}"
                );
                assert!(
                    !reason.contains("即将耗尽"),
                    "unreadable readings must not claim exhaustion: {reason}"
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
        assert_eq!(tier, ResourceTier::ReclaimDirect);
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

    #[test]
    fn any_short_write_target_volume_refuses_the_heavy_action() {
        let cwd = PathBuf::from(".");
        let elsewhere = PathBuf::from("..");
        let probe = Arc::new(PerPathProbe {
            cwd: cwd.clone(),
            cwd_snapshot: HostCapacitySnapshot {
                collected_at_ms: 1,
                volume_free_bytes: 40 * GIB,
                volume_total_bytes: 100 * GIB,
                commit_limit_bytes: 32 * GIB,
                commit_used_bytes: 4 * GIB,
                source_quality: SourceQuality::Available,
            },
            other_snapshot: HostCapacitySnapshot {
                collected_at_ms: 1,
                volume_free_bytes: 3 * GIB,
                volume_total_bytes: 100 * GIB,
                commit_limit_bytes: 32 * GIB,
                commit_used_bytes: 4 * GIB,
                source_quality: SourceQuality::Available,
            },
        });
        let gate = ResourceGate::new(probe);
        // The session volume alone would sail through.
        assert!(gate.evaluate(&cwd, ActionClass::Heavy).is_allowed());
        // With the second write target in play the action is refused, and the
        // refusal names the short volume.
        match gate.evaluate_for_volumes(&[cwd.clone(), elsewhere.clone()], ActionClass::Heavy) {
            GateDecision::Refuse {
                reason,
                volumes,
                tier,
                ..
            } => {
                assert_eq!(tier, ResourceTier::ReclaimDirect);
                assert!(reason.contains("free"), "{reason}");
                // 0af：只有卷余量短 ⇒ 定案句标「储存」轴。
                assert!(
                    reason.contains("宿主机储存资源即将耗尽"),
                    "storage-axis headline missing: {reason}"
                );
                assert_eq!(volumes.len(), 2, "every probed volume is reported");
                assert_eq!(volumes[1].path, elsewhere.display().to_string());
                assert!(volumes[1].to_json()["readings"]["volume_free_bytes"] == 3 * GIB);
            }
            other => panic!("expected refusal, got {other:?}"),
        }
        // And the light path still passes on the same volumes.
        assert!(
            gate.evaluate_for_volumes(&[cwd, elsewhere], ActionClass::Light)
                .is_allowed()
        );
    }

    // ── payload bodies (review F-7) ─────────────────────────────────────

    #[test]
    fn here_string_bodies_are_not_command_syntax() {
        let command = "cd D:\\CLI; @'\nimport json\nx = 1 > 0\n'@ | python -";
        assert_eq!(
            classify_command(command),
            ActionClass::Light,
            "a comparison inside a here-string body is payload, not a redirection"
        );
        // The header line still counts: a real redirection next to the opener
        // keeps the command heavy.
        assert_eq!(
            classify_command("cat > D:\\out.txt <<EOF\nhello\nEOF"),
            ActionClass::Heavy
        );
    }

    #[test]
    fn heredoc_bodies_are_not_command_syntax() {
        let command = "python - <<'PY'\nprint(1 > 0)\nPY\necho done";
        assert_eq!(classify_command(command), ActionClass::Light);
        // The body carries the heavy program, not the command line.
        assert_eq!(
            classify_command("sh -c 'true' <<EOF\ncargo build\nEOF"),
            ActionClass::Light
        );
    }

    #[test]
    fn unterminated_payload_markers_do_not_swallow_commands() {
        // `<<` without a terminator must leave the text alone, so the cargo
        // line after it is still classified (false negatives are the dangerous
        // direction).
        assert_eq!(
            classify_command("echo a << b;\ncargo build"),
            ActionClass::Heavy
        );
    }
}

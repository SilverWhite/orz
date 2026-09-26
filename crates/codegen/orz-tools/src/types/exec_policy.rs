//! L2 命令面机械审查（0bw S2② v1，2026-09-26）：`run_terminal_cmd` 的
//! best-effort 风险闸（block／warn／allow 三分类）与留痕面文案。
//!
//! 设计权威：[`docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`] §4（规则表）
//! 与 §9（边界）。匹配＝「程序位词元精确」＋「短语」两法；`cmd /c`／
//! `powershell -Command` 等内容位**递归展开一层**。**不解析完整 shell 语法**
//! ——变量拼接、别名、`.NET` 直调、脚本内命令、`git rm` 式子命令等不保证覆盖
//! （best-effort，声明边界；命中即拒、绝不因解析失败放行）。
//!
//! 命令内路径 token 经 [`crate::types::write_control`] 的同一绑定归一化后过同一
//! deny 表（单一源）：系统核心命中＝`system-core-write`；`.gsa`／安装目录命中＝
//! `carrier-write`；安全机制翻转类命令＝`safety-mechanism-flip`（block）；
//! 根级/通配删除与提权＝`broad-destructive`／`elevation`（warn）。

use std::path::{Path, PathBuf};

use crate::types::write_control;

/// 命中事实（block／warn 共用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandFinding {
    /// 规则身份（稳定字符串：`safety-mechanism-flip` / `system-core-write` /
    /// `carrier-write` / `broad-destructive` / `elevation`）。
    pub rule: &'static str,
    /// 机械细节（含命中词元/目标路径与根）。
    pub detail: String,
}

/// 三分类审查结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandReview {
    /// 表外——零改动放行。
    Allow,
    /// 高危形态：留痕（结果头部行）但**不阻断**。
    Warn(CommandFinding),
    /// 锁死面命中：命令**不执行**，返回机械拒绝文案。
    Block(CommandFinding),
}

// ─── 规则表（v1；设计 §4）───────────────────────────────────────────────

/// 安全机制翻转类程序（程序位词元精确；`.exe`/`.com` 后缀剥离后比对）。
const FLIP_PROGRAMS: &[&str] = &[
    "bcdedit",
    "diskpart",
    "format",
    "set-executionpolicy",
    "set-mppreference",
    "add-mppreference",
    "remove-mppreference",
    "set-mpcomputerstatus",
    "set-netfirewallprofile",
];

/// 程序位前缀（`mkfs`、`mkfs.ext4`…）。
const FLIP_PROGRAM_PREFIXES: &[&str] = &["mkfs"];

/// 短语（程序词元＋紧随参数拼接，前缀匹配）。
const FLIP_PHRASES: &[&str] = &[
    "netsh advfirewall set",
    "netsh firewall set",
    "wevtutil cl",
    "clear-eventlog",
    "fltmc unload",
    "vssadmin delete",
    "wbadmin delete",
    "sc stop windefend",
    "sc config windefend",
    "sc delete windefend",
    "sc stop mpssvc",
    "sc config mpssvc",
    "sc delete mpssvc",
    "net stop windefend",
    "net stop mpssvc",
    "stop-service windefend",
    "stop-service mpssvc",
    "stop-service -name windefend",
    "stop-service -name mpssvc",
];

/// 破坏/修改动词（程序位匹配；命中 deny 根目标即 block）。
const DESTRUCTIVE_VERBS: &[&str] = &[
    "rm", "rmdir", "rd", "del", "erase", "remove-item", "ri", "rmtree", "mv", "move", "move-item",
    "mi", "rename-item", "ren", "rename", "cp", "copy", "copy-item", "cpi", "xcopy", "robocopy",
    "set-content", "add-content", "out-file", "new-item", "ni", "mkdir", "md", "touch", "tee",
    "icacls", "takeown", "attrib", "cacls", "set-acl", "chmod", "chown",
];

/// 删除类动词（broad-destructive 的前置）。
const DELETE_VERBS: &[&str] = &["rm", "rmdir", "rd", "del", "erase", "remove-item", "ri", "rmtree"];

/// `reg` 修改子命令与受保护蜂巢。
const REG_MODIFY_SUBCOMMANDS: &[&str] =
    &["add", "delete", "import", "copy", "restore", "load", "unload"];
const REG_PROTECTED_HIVES: &[&str] =
    &["hklm", "hkey_local_machine", "hkcr", "hkey_classes_root", "hku", "hkey_users"];

/// 递归/强制旗（broad-destructive 的递归腿）。
const RECURSIVE_FLAGS: &[&str] = &["-r", "-rf", "-fr", "-recurse", "-force", "/s", "/q"];

/// 提权程序（elevation warn）。
const ELEVATION_PROGRAMS: &[&str] = &["sudo", "doas", "gsudo", "runas"];

/// wrapper/前缀词（找程序位时跳过；内容位 `-command` 等递归展开）。
const WRAPPER_WORDS: &[&str] = &[
    "sudo", "doas", "gsudo", "env", "nohup", "time", "exec", "command", "xargs", "cmd",
    "cmd.exe", "/c", "/k", "powershell", "powershell.exe", "pwsh", "pwsh.exe", "-command", "-c",
    "-lc", "-l", "bash", "bash.exe", "sh", "sh.exe", "zsh", "zsh.exe", "runas", "start-process",
    "-noprofile", "-nologo", "-verb",
];

/// 内容位词元（其后的内容词按子命令递归展开）。
const CONTENT_WORDS: &[&str] = &["-command", "-c", "-lc", "/c", "/k"];

// ─── 公开入口 ───────────────────────────────────────────────────────────

/// 审查一条 `run_terminal_cmd` 命令（生产入口：宿主平台系统根＋当前安装目录）。
pub fn review_command(cwd: &Path, command: &str) -> CommandReview {
    let install_dir = write_control::current_install_dir();
    review_command_with(
        cwd,
        command,
        install_dir.as_deref(),
        &write_control::system_core_roots(),
    )
}

/// 审查（注入式；测试与跨平台场景）。
pub fn review_command_with(
    cwd: &Path,
    command: &str,
    install_dir: Option<&Path>,
    system_roots: &[PathBuf],
) -> CommandReview {
    let scan = crate::util::unicode_confusables::normalize_confusables(command);
    let toks = tokenize(&scan);
    let words: Vec<Word> = toks
        .iter()
        .filter_map(|t| match t {
            Tok::Word(w) => Some(w.clone()),
            Tok::Sep => None,
        })
        .collect();
    let segments = segments(&toks);

    let mut entries: Vec<ProgEntry> = Vec::new();
    for seg in &segments {
        program_entries(seg, 0, &mut entries);
    }

    // ① safety-mechanism-flip（程序位精确／前缀／短语＋dd of=／设备路径）。
    for e in &entries {
        if FLIP_PROGRAMS.contains(&e.prog.as_str())
            || FLIP_PROGRAM_PREFIXES.iter().any(|p| e.prog.starts_with(p))
        {
            return CommandReview::Block(CommandFinding {
                rule: "safety-mechanism-flip",
                detail: format!("matched the safety-mechanism-flip rule (`{}`)", e.prog),
            });
        }
        if FLIP_PHRASES
            .iter()
            .any(|ph| e.phrase == *ph || e.phrase.starts_with(&format!("{ph} ")))
        {
            return CommandReview::Block(CommandFinding {
                rule: "safety-mechanism-flip",
                detail: format!("matched the safety-mechanism-flip rule (`{}`)", e.phrase),
            });
        }
        if e.words.iter().any(|w| w.contains("physicaldrive")) {
            return CommandReview::Block(CommandFinding {
                rule: "safety-mechanism-flip",
                detail: "raw device path (`physicaldrive`)".to_owned(),
            });
        }
    }

    // ② reg 修改受保护蜂巢（HKLM/HKCR/HKU）。
    for e in &entries {
        if e.prog == "reg" {
            let sub = e.words.get(1).map(String::as_str).unwrap_or("");
            let hive = e
                .words
                .iter()
                .any(|w| REG_PROTECTED_HIVES.iter().any(|h| w.starts_with(h)));
            if REG_MODIFY_SUBCOMMANDS.contains(&sub) && hive {
                return CommandReview::Block(CommandFinding {
                    rule: "system-core-write",
                    detail: "registry modification on HKLM/HKCR/HKU".to_owned(),
                });
            }
        }
    }

    // ③ 写动词 ＋ deny 根目标（命令内路径 token 过同一 deny 表）。
    let has_write_verb = entries
        .iter()
        .any(|e| DESTRUCTIVE_VERBS.contains(&e.prog.as_str()) || e.prog == "dd")
        || words.iter().any(|w| w.text == ">" || w.text == ">>");
    if has_write_verb {
        for w in &words {
            for raw in path_candidates(&w.text) {
                if let Some(finding) = deny_target_finding(cwd, &raw, install_dir, system_roots) {
                    return CommandReview::Block(finding);
                }
            }
        }
    }

    // ④ broad-destructive（warn）。
    let delete_present = entries
        .iter()
        .any(|e| DELETE_VERBS.contains(&e.prog.as_str()));
    if delete_present {
        for w in &words {
            if w.text == ">" || w.text == ">>" {
                continue;
            }
            let expanded = expand_word(cwd, &w.text);
            if is_rootish(&expanded) || w.text.contains('*') || w.text.contains('?') {
                return CommandReview::Warn(CommandFinding {
                    rule: "broad-destructive",
                    detail: format!("broad/root-level delete target (`{}`)", w.text),
                });
            }
        }
        let recursive = words
            .iter()
            .any(|w| RECURSIVE_FLAGS.contains(&norm_word(&w.text).as_str()));
        if recursive {
            for w in &words {
                for raw in path_candidates(&w.text) {
                    let forms = path_forms(cwd, &raw);
                    if forms.iter().all(|f| !is_within(cwd, f)) {
                        return CommandReview::Warn(CommandFinding {
                            rule: "broad-destructive",
                            detail: format!(
                                "recursive-force delete outside the workspace (`{}`)",
                                w.text
                            ),
                        });
                    }
                }
            }
        }
    }

    // ⑤ elevation（warn）。
    for seg in &segments {
        if let Some(first) = seg.iter().find(|w| !is_assignment(&w.text)) {
            let n = norm_word(&first.text);
            if ELEVATION_PROGRAMS.contains(&n.as_str()) {
                return CommandReview::Warn(CommandFinding {
                    rule: "elevation",
                    detail: format!("privilege elevation (`{}`)", n),
                });
            }
        }
    }
    if words.windows(2).any(|pair| {
        norm_word(&pair[0].text) == "-verb" && norm_word(&pair[1].text) == "runas"
    }) {
        return CommandReview::Warn(CommandFinding {
            rule: "elevation",
            detail: "privilege elevation (`-Verb RunAs`)".to_owned(),
        });
    }

    CommandReview::Allow
}

/// block 面向模型的机械拒绝文案（经 `ToolError` 返回；随 tool 结果入 journal）。
pub fn block_message(finding: &CommandFinding) -> String {
    format!(
        "Error: command blocked by the mechanical write control (rule: {}). {}. \
         The command was not executed; this is a best-effort risk gate (L2), not a complete sandbox.",
        finding.rule, finding.detail
    )
}

/// warn 结果头部行（随 tool 结果入 journal；不阻断）。
pub fn warn_line(finding: &CommandFinding) -> String {
    format!(
        "[写入管控·提示] {}（rule: {}；动作照常执行；best-effort 机械闸）",
        finding.detail, finding.rule
    )
}

// ─── 目标解析与 deny 表比对 ─────────────────────────────────────────────

/// 目标解析：展开 → 绝对化 → 词法/近祖先 canonical → 过 deny 表。
fn deny_target_finding(
    cwd: &Path,
    raw: &str,
    install_dir: Option<&Path>,
    system_roots: &[PathBuf],
) -> Option<CommandFinding> {
    let expanded = expand_word(cwd, raw);
    if expanded.starts_with("/dev/")
        || expanded.starts_with("/proc/")
        || expanded.starts_with("/sys/")
    {
        return Some(CommandFinding {
            rule: "system-core-write",
            detail: format!("target `{raw}` is a device/kernel path (`{expanded}`)"),
        });
    }
    let forms = path_forms(cwd, raw);
    for root in system_roots {
        if forms.iter().any(|f| write_control::path_hits_root(root, f)) {
            return Some(CommandFinding {
                rule: "system-core-write",
                detail: format!(
                    "target `{raw}` resolves inside the locked system-core set (`{}`)",
                    root.to_string_lossy()
                ),
            });
        }
    }
    let gsa = cwd.join(".gsa");
    let gsa_canonical = crate::types::resources::session_volume_canonical_root(cwd);
    for form in &forms {
        if write_control::path_hits_root(&gsa, form)
            || write_control::path_hits_root(&gsa_canonical, form)
        {
            return Some(CommandFinding {
                rule: "carrier-write",
                detail: format!("target `{raw}` is inside the `.gsa` session volume"),
            });
        }
    }
    if let Some(install) = install_dir
        && let Some(hit) = write_control::install_dir_hit(install, cwd, &forms)
    {
        return Some(CommandFinding {
            rule: "carrier-write",
            detail: format!(
                "target `{raw}` is inside the carrier self-protection set (`{}`)",
                hit.root
            ),
        });
    }
    None
}

/// 词元路径形态（相对路径按 cwd 拼接；词内空白拆片兜底）。
fn path_forms(cwd: &Path, raw: &str) -> Vec<PathBuf> {
    let expanded = expand_word(cwd, raw);
    let p = PathBuf::from(&expanded);
    let abs = if p.is_absolute() { p } else { cwd.join(&p) };
    let lexical = orz_paths::normalize_lexically(&abs);
    write_control::candidate_forms(&lexical, None)
}

/// 从词元提取路径候选（`of=…`／`-path=…` 取等号右值；引号剥除；含空白词拆片兜底）。
fn path_candidates(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let consider = |out: &mut Vec<String>, value: &str| {
        push_candidate(out, value);
        if value.contains(char::is_whitespace) {
            for piece in value.split_whitespace() {
                push_candidate(out, piece);
            }
        }
    };
    if let Some((key, value)) = text.split_once('=') {
        if key.is_empty() || key.starts_with('-') || key.eq_ignore_ascii_case("of") {
            consider(&mut out, value);
            return out;
        }
    }
    consider(&mut out, text);
    out
}

fn push_candidate(out: &mut Vec<String>, value: &str) {
    let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
    if looks_like_path(value) && !out.iter().any(|v| v == value) {
        out.push(value.to_owned());
    }
}

/// 近似路径形态判定（不追求完备：选项/赋值左值不取，含分隔符或特殊开头即候选）。
fn looks_like_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let b = s.as_bytes();
    if b[0] == b'-' {
        return false;
    }
    let has_sep = s.contains('\\') || s.contains('/');
    let drive = b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic();
    let special = s.starts_with('~')
        || s.starts_with('%')
        || s.starts_with("$env:")
        || s.starts_with("${")
        || s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with(".\\")
        || s.starts_with("..\\")
        || s.starts_with('/')
        || s.starts_with('\\');
    drive || special || has_sep
}

// ─── 词元/环境展开 ──────────────────────────────────────────────────────

/// 环境引用展开（`%VAR%`／`$env:VAR`／`${VAR}`；值未知保持原样）。
fn expand_word(cwd: &Path, raw: &str) -> String {
    let mut s = raw.to_owned();
    if s == "~" || s.starts_with("~/") || s.starts_with("~\\") {
        if let Some(home) = home_dir() {
            s = format!("{home}{}", &s[1..]);
        }
    }
    for (name, value) in known_vars(cwd) {
        s = replace_ci(&s, &format!("%{name}%"), &value);
        s = replace_ci(&s, &format!("$env:{name}"), &value);
        s = replace_ci(&s, &format!("${{{name}}}"), &value);
        s = replace_ci(&s, &format!("${name}"), &value);
    }
    s
}

fn home_dir() -> Option<String> {
    std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .filter(|s| !s.is_empty())
}

fn known_vars(cwd: &Path) -> Vec<(String, String)> {
    let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    vec![
        (
            "systemroot".to_owned(),
            get("SystemRoot").unwrap_or_else(|| r"C:\Windows".to_owned()),
        ),
        (
            "windir".to_owned(),
            get("SystemRoot").unwrap_or_else(|| r"C:\Windows".to_owned()),
        ),
        (
            "programfiles".to_owned(),
            get("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".to_owned()),
        ),
        (
            "programfiles(x86)".to_owned(),
            get("ProgramFiles(x86)").unwrap_or_else(|| r"C:\Program Files (x86)".to_owned()),
        ),
        (
            "programdata".to_owned(),
            get("ProgramData").unwrap_or_else(|| r"C:\ProgramData".to_owned()),
        ),
        (
            "systemdrive".to_owned(),
            get("SystemDrive").unwrap_or_else(|| "C:".to_owned()),
        ),
        (
            "userprofile".to_owned(),
            get("USERPROFILE").or_else(|| get("HOME")).unwrap_or_default(),
        ),
        (
            "temp".to_owned(),
            get("TEMP").or_else(|| get("TMP")).unwrap_or_default(),
        ),
        ("cd".to_owned(), cwd.to_string_lossy().into_owned()),
        ("pwd".to_owned(), cwd.to_string_lossy().into_owned()),
    ]
}

/// ASCII 大小写不敏感替换（仅替字面量；逐位匹配）。
fn replace_ci(haystack: &str, needle: &str, value: &str) -> String {
    if needle.is_empty() || value.is_empty() {
        return haystack.to_owned();
    }
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if n.len() > h.len() {
        return haystack.to_owned();
    }
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < h.len() {
        if i + n.len() <= h.len() && h[i..i + n.len()].eq_ignore_ascii_case(n) {
            out.push_str(value);
            i += n.len();
        } else {
            // 保持 UTF-8 边界：单字节推入不安全，按字符推。
            let ch = haystack[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// 根级目标判定（`/`、`C:\`、`~`、`$HOME`、通配符整体）。
fn is_rootish(expanded: &str) -> bool {
    let t = expanded.trim().trim_matches('"').trim_matches('\'');
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    if lower == "/" || lower == "\\" || lower == "*" || lower == "/*" || lower == "\\*" {
        return true;
    }
    if matches!(lower.as_str(), "~" | "$home" | "$env:userprofile" | "%userprofile%") {
        return true;
    }
    // 盘根（`C:`、`C:\`、`C:/`）
    let b = lower.as_bytes();
    if b.len() == 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return true;
    }
    if b.len() == 3 && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && b[0].is_ascii_alphabetic()
    {
        return true;
    }
    let p = Path::new(&lower);
    p.parent().is_none() && p.has_root()
}

fn is_within(base: &Path, candidate: &Path) -> bool {
    crate::types::resources::candidate_is_under(base, candidate)
}

fn is_assignment(text: &str) -> bool {
    match text.split_once('=') {
        Some((key, _)) => {
            !key.is_empty()
                && !key.starts_with('-')
                && !key.contains('\\')
                && !key.contains('/')
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        None => false,
    }
}

// ─── 词法：token 化与程序位 ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct Word {
    text: String,
    /// 整词由引号包裹（`-Command "…"` 内容递归的判据）。
    fully_quoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(Word),
    /// 段分隔（`;` `&&` `||` `|` `&` `(` `)` 换行）。
    Sep,
}

/// 程序位条目（`prog` 归一化词元；`phrase`＝prog 起最多 4 词拼接；`words`＝段内归一化词表）。
#[derive(Debug, Clone)]
struct ProgEntry {
    prog: String,
    phrase: String,
    words: Vec<String>,
}

fn tokenize(command: &str) -> Vec<Tok> {
    fn flush(toks: &mut Vec<Tok>, cur: &mut String, quoted: &mut bool) {
        if !cur.is_empty() || *quoted {
            toks.push(Tok::Word(Word {
                text: std::mem::take(cur),
                fully_quoted: *quoted,
            }));
            *quoted = false;
        }
    }
    let mut toks = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                if cur.is_empty() {
                    quoted = true;
                }
                quote = Some(c);
            }
            c if c.is_whitespace() => flush(&mut toks, &mut cur, &mut quoted),
            ';' | '\n' => {
                flush(&mut toks, &mut cur, &mut quoted);
                toks.push(Tok::Sep);
            }
            '&' => {
                flush(&mut toks, &mut cur, &mut quoted);
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                toks.push(Tok::Sep);
            }
            '|' => {
                flush(&mut toks, &mut cur, &mut quoted);
                if chars.peek() == Some(&'|') {
                    chars.next();
                }
                toks.push(Tok::Sep);
            }
            '(' | ')' => {
                flush(&mut toks, &mut cur, &mut quoted);
                toks.push(Tok::Sep);
            }
            '>' => {
                flush(&mut toks, &mut cur, &mut quoted);
                let mut text = String::from(">");
                if chars.peek() == Some(&'>') {
                    chars.next();
                    text.push('>');
                }
                toks.push(Tok::Word(Word {
                    text,
                    fully_quoted: false,
                }));
            }
            _ => cur.push(c),
        }
    }
    flush(&mut toks, &mut cur, &mut quoted);
    toks
}

fn segments(toks: &[Tok]) -> Vec<Vec<Word>> {
    let mut out = Vec::new();
    let mut current: Vec<Word> = Vec::new();
    for tok in toks {
        match tok {
            Tok::Word(w) => current.push(w.clone()),
            Tok::Sep => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn program_entries(seg: &[Word], depth: u8, out: &mut Vec<ProgEntry>) {
    if depth > 2 {
        return;
    }
    let mut idx = 0;
    while idx < seg.len() {
        let w = &seg[idx];
        let n = norm_word(&w.text);
        if is_assignment(&w.text) || WRAPPER_WORDS.contains(&n.as_str()) {
            if CONTENT_WORDS.contains(&n.as_str())
                && let Some(next) = seg.get(idx + 1)
            {
                push_content_entry(next, seg, idx + 1, depth, out);
                idx += 2;
                continue;
            }
            idx += 1;
            continue;
        }
        break;
    }
    if idx >= seg.len() {
        return;
    }
    let words: Vec<String> = seg[idx..].iter().map(|w| norm_word(&w.text)).collect();
    let phrase = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
    out.push(ProgEntry {
        prog: norm_prog(&seg[idx].text),
        phrase,
        words,
    });
}

fn push_content_entry(w: &Word, seg: &[Word], idx: usize, depth: u8, out: &mut Vec<ProgEntry>) {
    if w.fully_quoted && w.text.contains(|c: char| c.is_whitespace() || c == ';' || c == '|') {
        let inner = tokenize(&w.text);
        for inner_seg in segments(&inner) {
            program_entries(&inner_seg, depth + 1, out);
        }
        return;
    }
    let words: Vec<String> = seg[idx..].iter().map(|w| norm_word(&w.text)).collect();
    let phrase = words.iter().take(4).cloned().collect::<Vec<_>>().join(" ");
    out.push(ProgEntry {
        prog: norm_prog(&w.text),
        phrase,
        words,
    });
}

/// 词元归一（小写；短语匹配用）。
fn norm_word(text: &str) -> String {
    text.trim().to_ascii_lowercase()
}

/// 程序名归一（小写＋剥离脚本/可执行后缀）。
fn norm_prog(text: &str) -> String {
    let lower = norm_word(text);
    for suffix in [".exe", ".com", ".bat", ".cmd", ".ps1"] {
        if let Some(stripped) = lower.strip_suffix(suffix) {
            return stripped.to_owned();
        }
    }
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> Vec<PathBuf> {
        write_control::windows_system_core_roots_with(
            Some(r"C:\Windows"),
            Some(r"C:\Program Files"),
            Some(r"C:\Program Files (x86)"),
            Some(r"C:\ProgramData"),
        )
    }

    fn review(cmd: &str) -> CommandReview {
        review_command_with(
            Path::new(r"D:\proj"),
            cmd,
            Some(Path::new(r"D:\app\orz")),
            &roots(),
        )
    }

    fn rule_of(review: &CommandReview) -> Option<&'static str> {
        match review {
            CommandReview::Block(f) | CommandReview::Warn(f) => Some(f.rule),
            CommandReview::Allow => None,
        }
    }

    #[test]
    fn dev_routine_commands_stay_allowed() {
        for cmd in [
            "cargo test -p orz-tools --lib",
            "git status --short",
            "rg --format json x src",
            "python scripts/check_repository.py",
            "echo hello",
            // 读系统文件：无写动词 → 放行（只读面不受影响）。
            "Get-Content C:\\Windows\\win.ini",
            "type C:\\Windows\\win.ini",
            "grep -n foo C:\\Windows\\System32\\drivers\\etc\\hosts",
            // 工作区内清理与复制：放行。
            "Remove-Item .tmp-0bw-check -Recurse -Force",
            "copy src\\a.rs src\\b.rs",
            // 命中短语但只是检索参数（程序位不是翻转类）→ 不误伤。
            "rg \"set-mppreference\" docs",
        ] {
            assert_eq!(review(cmd), CommandReview::Allow, "must allow: {cmd}");
        }
    }

    #[test]
    fn safety_flip_commands_block() {
        for cmd in [
            "Set-MpPreference -EnableControlledFolderAccess Disabled",
            "powershell -Command \"Set-MpPreference -EnableControlledFolderAccess AuditMode\"",
            "Add-MpPreference -ControlledFolderAccessProtectedFolders \"D:\\x\"",
            "netsh advfirewall set allprofiles state off",
            "wevtutil cl Security",
            "Clear-EventLog -LogName System",
            "bcdedit /set testsigning on",
            "diskpart",
            "format D: /y",
            "mkfs.ext4 /dev/sdb1",
            "vssadmin delete shadows /all",
            "sc stop WinDefend",
            "net stop mpssvc",
            "Stop-Service -Name WinDefend",
            "fltmc unload X",
            "wbadmin delete catalog",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some("safety-mechanism-flip"), "rule for {cmd}");
        }
    }

    #[test]
    fn system_core_and_carrier_writes_block() {
        for (cmd, expected) in [
            ("del \"C:\\Windows\\Temp\\x.txt\"", "system-core-write"),
            (
                "Remove-Item -Recurse -Force C:\\Windows\\System32\\drivers",
                "system-core-write",
            ),
            (
                "cmd /c del \"C:\\Program Files\\App\\x\"",
                "system-core-write",
            ),
            (
                "Set-Content C:\\ProgramData\\app\\cfg.json hi",
                "system-core-write",
            ),
            ("echo x > C:\\Windows\\Temp\\y", "system-core-write"),
            ("reg delete HKLM\\Software\\Foo /f", "system-core-write"),
            ("dd if=/dev/zero of=/dev/sda", "system-core-write"),
            (
                "powershell -Command \"Remove-Item 'C:\\Windows\\Temp\\z' -Force\"",
                "system-core-write",
            ),
            ("Remove-Item D:\\app\\orz\\orz.exe", "carrier-write"),
            ("Set-Content .gsa\\journal\\x.txt hi", "carrier-write"),
            ("rm -rf D:\\proj\\.gsa\\*", "carrier-write"),
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Block(_)),
                "must block: {cmd} -> {r:?}"
            );
            assert_eq!(rule_of(&r), Some(expected), "rule for {cmd}");
        }
    }

    #[test]
    fn warns_fire_for_broad_and_elevation_shapes() {
        for cmd in [
            "rm -rf /",
            "Remove-Item -Recurse -Force D:\\other\\x",
            "del /s /q *",
            "sudo apt-get install foo",
            "Start-Process cmd -Verb RunAs",
        ] {
            let r = review(cmd);
            assert!(
                matches!(r, CommandReview::Warn(_)),
                "must warn: {cmd} -> {r:?}"
            );
        }
        assert_eq!(rule_of(&review("rm -rf /")), Some("broad-destructive"));
        assert_eq!(
            rule_of(&review("sudo apt-get install foo")),
            Some("elevation")
        );
    }

    #[test]
    fn block_message_and_warn_line_carry_rule_identity() {
        let CommandReview::Block(f) =
            review("Set-MpPreference -EnableControlledFolderAccess Disabled")
        else {
            panic!("expected block");
        };
        let msg = block_message(&f);
        assert!(msg.contains("safety-mechanism-flip"));
        assert!(msg.contains("blocked by the mechanical write control"));
        assert!(msg.contains("was not executed"));

        let CommandReview::Warn(f) = review("rm -rf /") else {
            panic!("expected warn");
        };
        let line = warn_line(&f);
        assert!(line.starts_with("[写入管控·提示]"));
        assert!(line.contains("broad-destructive"));
    }
}

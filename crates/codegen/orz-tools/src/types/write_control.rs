//! 写入管控（0bw S2 v1，2026-09-26）：deny 单一源表＋写目标检查。
//!
//! 设计权威：[`docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`]（S1）。
//! 本模块＝「deny 单一源」：**系统核心**（Windows 表 A：systemroot／program
//! files ×2／programdata；Linux 表 B：`/boot` `/etc` `/usr` `/lib*` `/bin`
//! `/sbin` `/dev` `/proc` `/sys`）＋**载体自保护**（`.gsa` 会话卷／orz 安装
//! 目录／三件套文件／`grok-home`）。三处消费（工具面写入路径、`run_terminal_cmd`
//! 命令面、进程面随 L3）共用本表——落地点不得各自复制表项。
//!
//! 语义（设计 §2.2）：命中即拒；表项解析失败回退字面默认（**绝不因解析失败
//! 放行**）；`\\?\`／`\\.\` 前缀比对前剥除；目标存在走 canonical、不存在走近
//! 祖先 canonical；Windows 走读面同族的字节级 ASCII 大小写折叠（FR-N01 语义）。
//!
//! 已知边界（设计 §9）：8.3 短名／subst／junction 的**未存在面**不保证拦截；
//! UNC 不在 v1 表内；本模块不构成沙箱——锁死面之外一切照旧（allowlist 不做）。

use std::path::{Path, PathBuf};

/// Windows 系统核心（表 A）env 缺失时的字面回退。
pub const WINDOWS_SYSTEM_CORE_FALLBACKS: [&str; 4] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
];

/// Linux 系统核心（表 B）。
pub const LINUX_SYSTEM_CORE: [&str; 12] = [
    "/boot", "/etc", "/usr", "/lib", "/lib32", "/lib64", "/libx32", "/bin", "/sbin", "/dev",
    "/proc", "/sys",
];

/// 载体三件套文件名（C3；含 `.exe` 变体）。
pub const CARRIER_BINARY_NAMES: [&str; 6] = [
    "orz",
    "orz.exe",
    "orz-signer",
    "orz-signer.exe",
    "orz-acaf-provision",
    "orz-acaf-provision.exe",
];

/// 安装目录内随载体保护的子目录（C3）。
pub const CARRIER_PROTECTED_SUBDIRS: [&str; 1] = ["grok-home"];

/// deny 表命中结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyHit {
    /// 规则身份：`system-core`／`carrier:session-volume`／`carrier:install-dir`／`carrier:install-file`。
    pub rule: &'static str,
    /// 命中的表项（展示形态）。
    pub root: String,
    /// 被拒的写目标（展示形态）。
    pub target: String,
}

/// 写目标上下文（工具面／命令面共用；`install_dir` 由调用方推导注入）。
pub struct WriteTargetCtx<'a> {
    pub cwd: &'a Path,
    /// 模型面拼写形态（含 `~` 展开后的 resolved 亦可）。
    pub joined: &'a Path,
    /// canonical（或近祖先 canonical）形态；与 `joined` 相同可传 `None`。
    pub resolved: Option<&'a Path>,
    /// orz 安装目录（`current_exe()` 父目录）；未知传 `None`。
    pub install_dir: Option<&'a Path>,
}

/// 当前进程的安装目录（`current_exe()` 父目录；`grok_home.rs::install_dir` 同义）。
pub fn current_install_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

/// 剥除 Windows 谓词前缀（`\\?\`／`\\.\`），防比对绕过。
pub fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    for prefix in [r"\\?\", r"\\.\"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            return PathBuf::from(rest);
        }
    }
    path.to_path_buf()
}

/// `candidate` 落在 `root` 之下（含相等）。
///
/// 委派读面同族 [`crate::types::resources::candidate_is_under`]（词法 `..` 归
/// 一＋Windows 字节级 ASCII 大小写折叠）；两端先剥谓词前缀。
pub fn path_hits_root(root: &Path, candidate: &Path) -> bool {
    let root = strip_verbatim_prefix(root);
    let candidate = strip_verbatim_prefix(candidate);
    crate::types::resources::candidate_is_under(&root, &candidate)
}

/// 近祖先 canonical：目标不存在时上溯至首个存在祖先做 canonical 解析，其余段
/// 词法拼接（覆盖 8.3 短名等只在存在面可解的形态；`parent()` 链终止即回退原样）。
pub fn best_effort_canonical(path: &Path) -> PathBuf {
    let mut probe = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if let Ok(canon) = dunce::canonicalize(&probe) {
            let mut out = canon;
            for seg in tail.iter().rev() {
                out.push(seg);
            }
            return out;
        }
        match probe.parent() {
            Some(parent) if parent != probe => {
                if let Some(name) = probe.file_name() {
                    tail.push(name.to_os_string());
                }
                probe = parent.to_path_buf();
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Windows 系统核心根构造（注入式；`None` 走字面回退）。
pub fn windows_system_core_roots_with(
    system_root: Option<&str>,
    program_files: Option<&str>,
    program_files_x86: Option<&str>,
    program_data: Option<&str>,
) -> Vec<PathBuf> {
    let pick = |value: Option<&str>, fallback: &str| -> PathBuf {
        match value.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => PathBuf::from(fallback),
        }
    };
    vec![
        pick(system_root, WINDOWS_SYSTEM_CORE_FALLBACKS[0]),
        pick(program_files, WINDOWS_SYSTEM_CORE_FALLBACKS[1]),
        pick(program_files_x86, WINDOWS_SYSTEM_CORE_FALLBACKS[2]),
        pick(program_data, WINDOWS_SYSTEM_CORE_FALLBACKS[3]),
    ]
}

/// Windows 系统核心根（env 读取；缺失回退字面默认）。
pub fn windows_system_core_roots() -> Vec<PathBuf> {
    let get = |key: &str| std::env::var(key).ok();
    windows_system_core_roots_with(
        get("SystemRoot").as_deref(),
        get("ProgramFiles").as_deref(),
        get("ProgramFiles(x86)").as_deref(),
        get("ProgramData").as_deref(),
    )
}

/// Linux 系统核心根（表 B）。
pub fn linux_system_core_roots() -> Vec<PathBuf> {
    LINUX_SYSTEM_CORE.iter().map(PathBuf::from).collect()
}

/// 宿主平台的系统核心根（进程面／命令面共用入口）。
pub fn system_core_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows_system_core_roots()
    }
    #[cfg(not(windows))]
    {
        linux_system_core_roots()
    }
}

/// 工具面（`search_replace`）机械拒绝文案。
///
/// 会话卷形态保持既有文案（0p S2 语义原文；既有测试断言 `not model-writable`）。
pub fn write_block_message(model_path: &str, hit: &DenyHit) -> String {
    match hit.rule {
        "carrier:session-volume" => format!(
            "Error: {model_path} is inside the runtime-owned `.gsa` session volume, which is \
             not model-writable."
        ),
        "carrier:install-dir" | "carrier:install-file" => format!(
            "Error: {model_path} is inside the orz installation carrier ({root}), protected by \
             the write control (rule: {rule}); writes here are not permitted.",
            root = hit.root,
            rule = hit.rule,
        ),
        _ => format!(
            "Error: {model_path} is inside the locked system-core set ({root}), protected by \
             the write control (rule: system-core); writes here are not permitted.",
            root = hit.root,
        ),
    }
}

/// 候选形态集：词法形态＋resolved 形态＋近祖先 canonical。
pub fn candidate_forms(joined: &Path, resolved: Option<&Path>) -> Vec<PathBuf> {
    let joined = strip_verbatim_prefix(joined);
    let mut forms: Vec<PathBuf> = vec![joined.clone()];
    if let Some(r) = resolved {
        let r = strip_verbatim_prefix(r);
        if !forms.contains(&r) {
            forms.push(r);
        }
    }
    let best = best_effort_canonical(&joined);
    if !forms.contains(&best) {
        forms.push(best);
    }
    forms
}

/// 写目标机械门（设计 §3.1；工具面唯一入口）。
///
/// 顺序：C1 会话卷域（委派既有单一源判定，语义不重写）→ C2/C3 安装目录
/// （含整树／降级两形态）→ A/B 系统核心。命中返回 [`DenyHit`]。
pub fn check_write_target(ctx: &WriteTargetCtx<'_>) -> Option<DenyHit> {
    let forms = candidate_forms(ctx.joined, ctx.resolved);
    let display = forms[0].to_string_lossy().into_owned();

    // C1：`.gsa` 会话卷域（既有窗口契约的单源判定）。
    let gsa_root = crate::types::resources::session_volume_canonical_root(ctx.cwd);
    if crate::types::resources::is_path_in_session_volume_domain(
        &gsa_root,
        ctx.cwd,
        ctx.joined,
        ctx.resolved,
    ) {
        return Some(DenyHit {
            rule: "carrier:session-volume",
            root: ".gsa session volume".to_owned(),
            target: display,
        });
    }

    // C2/C3：安装目录（整树；install_dir ⊆ cwd 时降级为文件级）。
    if let Some(install) = ctx.install_dir
        && let Some(hit) = install_dir_hit(install, ctx.cwd, &forms)
    {
        return Some(hit);
    }

    // A/B：系统核心。
    let roots = system_core_roots();
    if let Some(root) = first_root_hit(&roots, &forms) {
        return Some(DenyHit {
            rule: "system-core",
            root: root.to_string_lossy().into_owned(),
            target: display,
        });
    }
    None
}

/// 安装目录命中检查（命令面复用；`forms` 为候选形态集）。
pub(crate) fn install_dir_hit(
    install: &Path,
    cwd: &Path,
    forms: &[PathBuf],
) -> Option<DenyHit> {
    let install_norm = strip_verbatim_prefix(install);
    let root_display = install_norm.to_string_lossy().into_owned();

    // 降级判定（设计 §2.1 C2）：install_dir ⊆ cwd ⇒ 文件级（避免封锁工作区）。
    let degraded = crate::types::resources::candidate_is_under(cwd, &install_norm);

    if !degraded {
        return first_form_hit(&install_norm, forms).map(|target| DenyHit {
            rule: "carrier:install-dir",
            root: root_display,
            target: target.to_string_lossy().into_owned(),
        });
    }

    // 降级：三件套文件名／`grok-home` 子树／安装目录本体。
    for form in forms {
        let Some(rel) = relative_under(&install_norm, form) else {
            continue;
        };
        if rel.as_os_str().is_empty() {
            return Some(DenyHit {
                rule: "carrier:install-dir",
                root: root_display,
                target: form.to_string_lossy().into_owned(),
            });
        }
        let mut comps = rel.components();
        let first = comps.next()?;
        let first_s = first.as_os_str().to_string_lossy();
        let single_component = comps.next().is_none();
        let is_trio = single_component
            && CARRIER_BINARY_NAMES
                .iter()
                .any(|n| first_s.eq_ignore_ascii_case(n));
        let is_subdir = CARRIER_PROTECTED_SUBDIRS
            .iter()
            .any(|d| first_s.eq_ignore_ascii_case(d));
        if is_trio || is_subdir {
            return Some(DenyHit {
                rule: "carrier:install-file",
                root: root_display,
                target: form.to_string_lossy().into_owned(),
            });
        }
    }
    None
}

/// 首个命中 `roots` 的形态（返回命中的根）。
fn first_root_hit<'r>(roots: &'r [PathBuf], forms: &[PathBuf]) -> Option<&'r PathBuf> {
    roots
        .iter()
        .find(|root| forms.iter().any(|form| path_hits_root(root, form)))
}

/// `forms` 中落在 `root` 之下的首个形态。
fn first_form_hit<'f>(root: &Path, forms: &'f [PathBuf]) -> Option<&'f PathBuf> {
    forms.iter().find(|form| path_hits_root(root, form))
}

/// `candidate` 相对 `base` 的余段（Windows 按组件大小写折叠；不落 base 返回 `None`）。
pub(crate) fn relative_under(base: &Path, candidate: &Path) -> Option<PathBuf> {
    let base_n = strip_verbatim_prefix(base);
    let cand_n = strip_verbatim_prefix(candidate);
    if let Ok(rest) = cand_n.strip_prefix(&base_n) {
        return Some(rest.to_path_buf());
    }
    #[cfg(windows)]
    {
        let b: Vec<_> = base_n.components().collect();
        let c: Vec<_> = cand_n.components().collect();
        if c.len() >= b.len()
            && b.iter().zip(c.iter()).all(|(x, y)| {
                x.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&y.as_os_str().to_string_lossy())
            })
        {
            let mut rest = PathBuf::new();
            for comp in &c[b.len()..] {
                rest.push(comp.as_os_str());
            }
            return Some(rest);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_roots_hit_and_boundaries() {
        let roots = windows_system_core_roots_with(None, None, None, None);
        let windows = &roots[0];
        assert!(path_hits_root(windows, &windows.join("Temp").join("x.txt")));
        assert!(path_hits_root(windows, windows));
        assert!(!path_hits_root(windows, Path::new(r"C:\WindowsX\f.txt")));
        assert!(path_hits_root(
            &roots[1],
            Path::new(r"C:\Program Files\App\x")
        ));
        // `C:\Program Files (x86)` 不是 `C:\Program Files` 的落点（边界字节非分隔符）。
        assert!(!path_hits_root(
            &roots[1],
            Path::new(r"C:\Program Files (x86)\App\x")
        ));
        assert!(path_hits_root(
            &roots[2],
            Path::new(r"C:\Program Files (x86)\App\x")
        ));
        assert!(path_hits_root(
            &roots[3],
            Path::new(r"C:\ProgramData\App\x")
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_case_folding_and_verbatim_prefix_stripping() {
        let roots = windows_system_core_roots_with(None, None, None, None);
        let windows = &roots[0];
        assert!(path_hits_root(windows, Path::new(r"c:\windows\Temp\x")));
        assert!(path_hits_root(windows, Path::new(r"\\?\C:\Windows\Temp\x")));
        assert!(path_hits_root(windows, Path::new(r"\\.\C:\WINDOWS\x")));
        assert!(path_hits_root(
            Path::new(r"\\?\C:\Windows"),
            Path::new(r"C:\windows\x")
        ));
    }

    #[test]
    fn linux_roots_hit_and_boundaries() {
        assert!(path_hits_root(Path::new("/etc"), Path::new("/etc/shadow")));
        assert!(path_hits_root(Path::new("/etc"), Path::new("/etc")));
        assert!(!path_hits_root(Path::new("/etc"), Path::new("/etcx")));
        assert!(path_hits_root(
            Path::new("/usr"),
            Path::new("/usr/lib/x.so")
        ));
        assert!(!path_hits_root(Path::new("/usr"), Path::new("/user/x")));
        assert!(path_hits_root(Path::new("/boot"), Path::new("/boot/grub/x")));
    }

    #[test]
    fn system_core_roots_env_fallback_binds_literals() {
        let fallback = windows_system_core_roots_with(None, None, None, None);
        assert_eq!(fallback[0], PathBuf::from(r"C:\Windows"));
        assert_eq!(fallback[3], PathBuf::from(r"C:\ProgramData"));
        let overridden =
            windows_system_core_roots_with(Some(r"D:\Win"), None, Some("  "), Some(r"E:\PD"));
        assert_eq!(overridden[0], PathBuf::from(r"D:\Win"));
        assert_eq!(overridden[2], PathBuf::from(r"C:\Program Files (x86)"));
        assert_eq!(overridden[3], PathBuf::from(r"E:\PD"));
        assert_eq!(linux_system_core_roots().len(), LINUX_SYSTEM_CORE.len());
    }

    #[test]
    fn install_dir_full_tree_when_outside_cwd() {
        let cwd = Path::new(r"D:\proj");
        let install = Path::new(r"D:\app\rel");
        let forms = candidate_forms(&install.join("data").join("x.txt"), None);
        let hit = install_dir_hit(install, cwd, &forms).expect("install-dir hit");
        assert_eq!(hit.rule, "carrier:install-dir");
        let elsewhere = candidate_forms(Path::new(r"D:\app\other\x.txt"), None);
        assert!(install_dir_hit(install, cwd, &elsewhere).is_none());
    }

    #[test]
    fn install_dir_degrades_to_file_level_when_inside_cwd() {
        let cwd = Path::new(r"D:\CLI");
        let install = Path::new(r"D:\CLI\orz\target\release");
        let trio = candidate_forms(&install.join("orz.exe"), None);
        let hit = install_dir_hit(install, cwd, &trio).expect("trio hit");
        assert_eq!(hit.rule, "carrier:install-file");
        let home = candidate_forms(&install.join("grok-home").join("creds.json"), None);
        assert!(install_dir_hit(install, cwd, &home).is_some());
        let other = candidate_forms(&install.join("notes.txt"), None);
        assert!(install_dir_hit(install, cwd, &other).is_none());
        let dir_itself = candidate_forms(install, None);
        assert!(install_dir_hit(install, cwd, &dir_itself).is_some());
    }

    #[test]
    fn session_volume_domain_is_rejected_and_workspace_allowed() {
        let tmp = std::env::temp_dir().join("0bw-write-control-gsa");
        let _ = std::fs::create_dir_all(tmp.join(".gsa"));
        let hit = check_write_target(&WriteTargetCtx {
            cwd: &tmp,
            joined: &tmp.join(".gsa").join("journal").join("x.md"),
            resolved: None,
            install_dir: None,
        })
        .expect("session volume hit");
        assert_eq!(hit.rule, "carrier:session-volume");
        assert!(
            check_write_target(&WriteTargetCtx {
                cwd: &tmp,
                joined: &tmp.join("src").join("x.md"),
                resolved: None,
                install_dir: None,
            })
            .is_none()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn check_write_target_reports_system_core_rule() {
        let roots = system_core_roots();
        let target = roots[0].join("Temp").join("0bw-write-control-test.txt");
        let cwd = std::env::temp_dir();
        let hit = check_write_target(&WriteTargetCtx {
            cwd: &cwd,
            joined: &target,
            resolved: None,
            install_dir: None,
        })
        .expect("system core hit");
        assert_eq!(hit.rule, "system-core");
    }

    #[test]
    fn best_effort_canonical_resolves_nearest_existing_ancestor() {
        let tmp = std::env::temp_dir().join("0bw-best-effort-canon");
        let _ = std::fs::create_dir_all(tmp.join("sub"));
        let probe = tmp.join("sub").join("not-yet.txt");
        let canon = best_effort_canonical(&probe);
        assert_eq!(canon.file_name().unwrap(), "not-yet.txt");
        assert_eq!(canon.parent().unwrap(), dunce::canonicalize(tmp.join("sub")).unwrap());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[cfg(windows)]
    #[test]
    fn relative_under_folds_case() {
        let rel = relative_under(
            Path::new(r"D:\App\Rel"),
            Path::new(r"d:\app\rel\grok-home\x"),
        )
        .expect("relative");
        assert_eq!(rel, PathBuf::from("grok-home\\x"));
        assert!(relative_under(Path::new(r"D:\App"), Path::new(r"D:\Other\x")).is_none());
    }
}

//! 写入管控（0bw v1 → 0cb v2 → **0cc v3 宿主机灾难保底收窄**，2026-09-29；
//! **v3.1 审查处理批**，2026-09-30）：宿主状态两窄目标＋根本树根常量（仅作
//! 递归删除目标）。
//!
//! 设计权威：[`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`]（v3.1，
//! 修订 [`WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`]（v1）的 deny 表范围；
//! 单一源／三落地点／留痕／补偿架构不变）。**v3 收窄**（用户裁决「把灾难保底
//! 纯粹变成宿主机灾难保底……只要不重建，orz 实际上不会被即时破坏」）：工具面
//! [`check_write_target`] 只查**宿主状态两窄目标**——C1 `.gsa` 会话卷＋C2′
//! ACAF keystore 根（keystore 目录子树＋signer manifest 本体，装配同源 env
//! 调用点解析，见 [`HostStateTargets`]）；v2 的安装目录／三件套／`grok-home`／
//! ⊆cwd 降级**双面全退役**（设计 §2 条 5 四点论证：守卫自保循环论证／恢复成本
//! 分类／Windows OS 文件锁／keystore 信任锚例外）——载体面写交回审批组件。
//! **v3.1 补强（审查 P2）**：keystore／manifest 的**祖先链臂** [`HostStateTargets::hit_ancestor`]
//! ——宿主原生 keystore 实况位于已退役的载体安装目录内，扫荡删除／搬移祖先
//! 不得因载体面退役而连带放行信任锚。v1 表
//! A/B「根本性树根」仅以 [`LINUX_DISASTER_TREE_ROOTS`]／[`windows_disaster_tree_roots`]
//! 的身份保留给命令面规则 1（`catastrophic-recursive-delete`：删除动词＋递归旗＋
//! 目标**恰为**树根／卷根）的递归删除目标比对（[`path_equals_root`]——子目录级
//! 精准删除放行）。
//!
//! 语义（v1 §2.2 沿用）：命中即拒；表项解析失败回退字面默认（**绝不因解析失败
//! 放行**）；`\\?\`／`\\.\` 前缀比对前剥除；目标存在走 canonical、不存在走近
//! 祖先 canonical；Windows 走读面同族的字节级 ASCII 大小写折叠（FR-N01 语义）。
//!
//! 已知边界（v1 §9 沿用）：8.3 短名／subst／junction 的**未存在面**不保证拦截；
//! UNC 不在表内；本模块不构成沙箱——宿主状态两目标之外一切照旧（allowlist 不做）。
//!
//! **L3 边界（0cc S2 收窄——S4 成败项）**：Landlock 排除集随 v3 同步收窄为
//! **灾难防护所需最小核** [`LINUX_DISASTER_KERNEL_FACES`]（`/boot` `/dev`
//! `/proc` `/sys`——规则 2 raw 设备与规则 3 引导/内核机制翻转的内核面）；
//! 载体面系统树（`/etc` `/usr` `/lib*` `/bin` `/sbin`）放行——否则 L2 放行的
//! `/usr/local/bin` 安装仍死于内核 EPERM（run `RUN-CLI-6abbb013` 实证 L3 在
//! 容器在役）。消费方＝`computer/local/terminal.rs` 与 orz-sandbox，变更须
//! 多处同步。
//!
//! **L3 粒度精准化（0ch v4，2026-10-01）**：目录排除集恰 4 项零变更之外，
//! 新增设备面安全节点封闭表 [`DEVICE_SAFE_NODES`]（恰 7 项——文件级
//! `WRITE_FILE`(＋TRUNCATE) 授权，消除「`>/dev/null` 被 L3 拒而 L1/L2 豁免」
//! 的两层粒度差）；`/` 本体 make 族子集授权（`ROOT_MAKE_GRANT`，机制侧
//! orz-sandbox 常量——消除 `mkdir /git` 恒拒）由 S1 设计档 §7.2 定档。
//! L3 粒度不得比 L1/L2 粗；设计权威＝[`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`]
//! §7。

use std::path::{Path, PathBuf};

/// Windows 根本性树根（v2 规则 1 递归删除目标；v1 表 A 同值）env 缺失时的字面回退。
pub const WINDOWS_DISASTER_TREE_ROOT_FALLBACKS: [&str; 4] = [
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\Program Files (x86)",
    "C:\\ProgramData",
];

/// Linux 根本性树根（v2 规则 1 递归删除目标；设计 §1——较 v1 表 B 增 `/var`、
/// 去 `/libx32`；`/dev` `/proc` `/sys` 在此仅指「递归删除树根本体」形态，
/// 一般性路径写/读不涉本表）。
pub const LINUX_DISASTER_TREE_ROOTS: [&str; 12] = [
    "/boot", "/etc", "/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/var", "/dev", "/proc",
    "/sys",
];

/// L3 Landlock 排除集（**0cc v3 收窄＝灾难防护所需最小核**；原 v1 表 B 十二树
/// 中的载体面系统树 `/etc` `/usr` `/lib*` `/bin` `/sbin` 放行）。恰四项＝规则 2
/// （raw 设备——`/dev`）与规则 3（引导固件与内核机制翻转——`/boot`、`/proc`
/// （sysctl/sysrq 机制位）、`/sys`（efivars/securityfs 机制位））的内核面；
/// 规则 1 递归删除与规则 5 宿主状态非顶层目录形态，Landlock 顶层粒度不可表达，
/// 由 L1/L2 承载。消费方＝`computer/local/terminal.rs` 与 orz-sandbox 测试，
/// 变更须多处同步。
pub const LINUX_DISASTER_KERNEL_FACES: [&str; 4] = ["/boot", "/dev", "/proc", "/sys"];

/// L3 设备面安全节点表（**0ch v4 子项①**；封闭表——Landlock 文件级
/// `WRITE_FILE`(＋TRUNCATE) 授权，恰好 7 项）。`/dev` 目录树维持整树不授权
/// （块设备／`/dev/mem` 等写、节点删除、`mknod` 仍内核拒），本表只对**单个
/// 安全节点**放写——L1/L2 规则 2 对 `/dev/null` 显式豁免、块设备形态集仅
/// `sd*`/`vd*`/`nvme*`/`mmcblk*`/`mapper*`，本表与该形态集零交集（一致性钉
/// 在 `exec_policy.rs` 测试）。缺席节点/symlink 由装挂面跳过＝默认拒
/// （fail-closed）。行数钉＋危险形态负向钉见本模块测试，新增表项必改测试。
pub const DEVICE_SAFE_NODES: [&str; 7] = [
    "/dev/null",
    "/dev/zero",
    "/dev/full",
    "/dev/tty",
    "/dev/random",
    "/dev/urandom",
    "/dev/ptmx",
];

/// deny 表命中结果（v3：宿主状态两族；v3.1 增祖先链两 face）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyHit {
    /// 规则身份：`carrier:session-volume`／`carrier:keystore-root`／
    /// `carrier:signer-manifest`（直接命中）／`carrier:keystore-ancestor`／
    /// `carrier:signer-manifest-ancestor`（v3.1 祖先链臂——命令面由扫荡动词
    /// 门控，见 [`HostStateTargets::hit_ancestor`]）。
    pub rule: &'static str,
    /// 命中的表项（展示形态）。
    pub root: String,
    /// 被拒的写目标（展示形态）。
    pub target: String,
}

/// v3 规则 5 宿主状态两窄目标（0cc；工具面／命令面／回退面共用单一源）。
///
/// 装配同源 env 的调用点解析：keystore 根与 signer manifest 取自
/// `ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST`——与 `orz-bin` AcafClient 装配
/// 同源的键；各消费点调用 [`Self::from_env`] 时读取（子进程 env 变更不回传
/// 父进程，运行期内与装配期常量等价）。容器面 keystore 随会话卷落 `.gsa` 域
/// 时由 C1 先行覆盖——[`check_write_target`] 先查 C1，**双覆盖时报告
/// `carrier:session-volume`**；宿主原生面走本结构窄目标。缺席（env 未设）＝
/// 该窄目标不设防——与 AcafClient「未配置 ⇒ 无票据」同形；命中即拒
/// （fail-closed 于命中）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostStateTargets {
    /// ACAF keystore 根（目录子树整体保护——keystore 目录＋key 两件）。
    pub keystore_root: Option<PathBuf>,
    /// 签名器 manifest（恰为本体文件；宿主原生面在 keystore 目录外）。
    pub signer_manifest: Option<PathBuf>,
}

impl HostStateTargets {
    /// 装配同源 env 键解析（调用点读取；`ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST`；
    /// 空白值视为缺席）。
    pub fn from_env() -> Self {
        let read = |key: &str| -> Option<PathBuf> {
            std::env::var(key)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Self {
            keystore_root: read("ORZ_ACAF_KEYSTORE"),
            signer_manifest: read("ORZ_ACAF_MANIFEST"),
        }
    }

    /// 候选形态集比对（工具面／命令面／回退面共用；`forms` 为 [`candidate_forms`]
    /// 等候选形态集）：keystore 根＝子树包含（含本体）；manifest＝恰为本体。
    /// keystore 根优先。祖先形态不进本判定——见 [`Self::hit_ancestor`]（命令面
    /// 扫荡动词门控下的专用臂；工具面/回退面目标恒为文件，不可能为目录祖先）。
    pub fn hit(&self, forms: &[PathBuf]) -> Option<DenyHit> {
        if let Some(root) = &self.keystore_root {
            let root_norm = strip_verbatim_prefix(root);
            if let Some(target) = forms.iter().find(|f| path_hits_root(&root_norm, f)) {
                return Some(DenyHit {
                    rule: "carrier:keystore-root",
                    root: root_norm.to_string_lossy().into_owned(),
                    target: target.to_string_lossy().into_owned(),
                });
            }
        }
        if let Some(manifest) = &self.signer_manifest {
            let manifest_norm = strip_verbatim_prefix(manifest);
            if let Some(target) = forms.iter().find(|f| path_equals_root(&manifest_norm, f)) {
                return Some(DenyHit {
                    rule: "carrier:signer-manifest",
                    root: manifest_norm.to_string_lossy().into_owned(),
                    target: target.to_string_lossy().into_owned(),
                });
            }
        }
        None
    }

    /// v3.1 祖先链臂（审查 P2 补强；命令面专用——扫荡动词门控在调用方，
    /// 见 `exec_policy::ANCESTOR_SWEEP_VERBS`）：目标为受护目标的**严格祖先**
    /// （直接命中已由 [`Self::hit`] 排除，相等形态不会到达本臂）。
    ///
    /// 动机：宿主原生 keystore／manifest 实况位于已退役的载体安装目录内
    /// （`<install>\acaf\keystore`）——载体面退役后，`rm -rf <install>` 一类
    /// 扫荡删除／搬移不触任何规则却连带摧毁信任锚（设计 §2 条 5 定类
    /// 「非平凡可恢复」的本体）。本臂使祖先形态同落 `carrier-write` block；
    /// `cp`/`mkdir` 等入位写不毁祖先，不入动词门。
    ///
    /// 比对方向与 [`Self::hit`] 相反（形态为 base、受护目标为其下）；keystore
    /// 优先于 manifest。卷根／盘根亦为祖先（命令面规则 1 恰本体比对先行，故
    /// 卷根递归删除仍报 `catastrophic-recursive-delete`）。
    pub fn hit_ancestor(&self, forms: &[PathBuf]) -> Option<DenyHit> {
        if let Some(root) = &self.keystore_root {
            let root_norm = strip_verbatim_prefix(root);
            if let Some(target) = forms.iter().find(|f| path_hits_root(f, &root_norm)) {
                return Some(DenyHit {
                    rule: "carrier:keystore-ancestor",
                    root: root_norm.to_string_lossy().into_owned(),
                    target: target.to_string_lossy().into_owned(),
                });
            }
        }
        if let Some(manifest) = &self.signer_manifest {
            let manifest_norm = strip_verbatim_prefix(manifest);
            if let Some(target) = forms.iter().find(|f| path_hits_root(f, &manifest_norm)) {
                return Some(DenyHit {
                    rule: "carrier:signer-manifest-ancestor",
                    root: manifest_norm.to_string_lossy().into_owned(),
                    target: target.to_string_lossy().into_owned(),
                });
            }
        }
        None
    }
}

/// 写目标上下文（工具面／命令面共用；`host_state` 由调用方装配注入）。
pub struct WriteTargetCtx<'a> {
    pub cwd: &'a Path,
    /// 模型面拼写形态（含 `~` 展开后的 resolved 亦可）。
    pub joined: &'a Path,
    /// canonical（或近祖先 canonical）形态；与 `joined` 相同可传 `None`。
    pub resolved: Option<&'a Path>,
    /// 宿主状态两窄目标（v3；`HostStateTargets::from_env()` 或注入）。
    pub host_state: &'a HostStateTargets,
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

/// Windows 根本性树根构造（注入式；`None` 走字面回退）。
pub fn windows_disaster_tree_roots_with(
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
        pick(system_root, WINDOWS_DISASTER_TREE_ROOT_FALLBACKS[0]),
        pick(program_files, WINDOWS_DISASTER_TREE_ROOT_FALLBACKS[1]),
        pick(program_files_x86, WINDOWS_DISASTER_TREE_ROOT_FALLBACKS[2]),
        pick(program_data, WINDOWS_DISASTER_TREE_ROOT_FALLBACKS[3]),
    ]
}

/// Windows 根本性树根（env 读取；缺失回退字面默认）。
pub fn windows_disaster_tree_roots() -> Vec<PathBuf> {
    let get = |key: &str| std::env::var(key).ok();
    windows_disaster_tree_roots_with(
        get("SystemRoot").as_deref(),
        get("ProgramFiles").as_deref(),
        get("ProgramFiles(x86)").as_deref(),
        get("ProgramData").as_deref(),
    )
}

/// Linux 根本性树根（规则 1 递归删除目标）。
pub fn linux_disaster_tree_roots() -> Vec<PathBuf> {
    LINUX_DISASTER_TREE_ROOTS
        .iter()
        .map(PathBuf::from)
        .collect()
}

/// 宿平台的根本性树根（命令面规则 1 共用入口）。
pub fn disaster_tree_roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        windows_disaster_tree_roots()
    }
    #[cfg(not(windows))]
    {
        linux_disaster_tree_roots()
    }
}

/// 卷根判定（`/`、`C:\`、`D:\`…；词法 `..` 归一后判——`/usr/../..` 即 `/`）。
pub fn is_volume_root(path: &Path) -> bool {
    let norm = orz_paths::normalize_lexically(&strip_verbatim_prefix(path));
    norm.has_root() && norm.parent().is_none()
}

/// 目标解析后**恰为** `root` 本体（双向包含＝相等；Windows 折叠语义同
/// [`path_hits_root`]）。v2 规则 1 的比对形状——子目录级目标放行（「需精准
/// 删除」硬边界；无路径前缀宽扫）。
pub fn path_equals_root(root: &Path, candidate: &Path) -> bool {
    path_hits_root(root, candidate) && path_hits_root(candidate, root)
}

/// 工具面（`search_replace`／回退面）机械拒绝文案（v3＝宿主状态两族；系统树
/// 与载体面写不再拒）。
///
/// 会话卷形态保持既有文案（0p S2 语义原文；既有测试断言 `not model-writable`）；
/// keystore／manifest 形态各带规则 id（反馈面核证：三层拒绝面文案带可区分
/// 来源标识——L2 文案已带规则 id，L3 为 shell EPERM 既有形状）。v3.1 祖先链
/// 两 face 仅由命令面产出（扫荡动词门控），此处文案臂保持函数对命中空间全盖
/// （防御性完备，非生产路径）。
pub fn write_block_message(model_path: &str, hit: &DenyHit) -> String {
    match hit.rule {
        "carrier:session-volume" => format!(
            "Error: {model_path} is inside the runtime-owned `.gsa` session volume, which is \
             not model-writable."
        ),
        "carrier:keystore-root" => format!(
            "Error: {model_path} is inside the ACAF keystore root ({root}), protected by \
             the write control (rule: {rule}); writes here are not permitted.",
            root = hit.root,
            rule = hit.rule,
        ),
        "carrier:signer-manifest" => format!(
            "Error: {model_path} is the ACAF signer manifest ({root}), protected by \
             the write control (rule: {rule}); writes here are not permitted.",
            root = hit.root,
            rule = hit.rule,
        ),
        "carrier:keystore-ancestor" => format!(
            "Error: {model_path} is an ancestor of the ACAF keystore root ({root}); deleting \
             or moving it would destroy the protected host state (rule: {rule}).",
            root = hit.root,
            rule = hit.rule,
        ),
        "carrier:signer-manifest-ancestor" => format!(
            "Error: {model_path} is an ancestor of the ACAF signer manifest ({root}); deleting \
             or moving it would destroy the protected host state (rule: {rule}).",
            root = hit.root,
            rule = hit.rule,
        ),
        _ => format!(
            "Error: {model_path} is protected by the write control (rule: {rule}); writes \
             here are not permitted.",
            rule = hit.rule,
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

/// 写目标机械门（v3；工具面唯一入口）——**宿主状态两窄目标专用**：C1 会话卷
/// 域（委派既有单一源判定，语义不重写）→ C2′ keystore 根＋signer manifest。
/// v2 的安装目录／三件套／`grok-home` 拒绝面随 0cc 双面退役（设计 §2 条 5：
/// 载体面写交回审批组件）；v1 的系统核心臂随整树位置锁退役（`0cb` §2/§4）。
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

    // C2′：ACAF keystore 根＋signer manifest（v3 宿主状态窄目标）。
    ctx.host_state.hit(&forms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disaster_tree_roots_hit_and_boundaries() {
        let roots = windows_disaster_tree_roots_with(None, None, None, None);
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
    fn case_folding_and_verbatim_prefix_stripping() {
        let roots = windows_disaster_tree_roots_with(None, None, None, None);
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
    fn path_equals_root_matches_only_the_root_itself() {
        // 规则 1 比对形状：目标**恰为**树根本体；子目录级放行（精准删除）。
        assert!(path_equals_root(Path::new("/usr"), Path::new("/usr")));
        assert!(path_equals_root(Path::new("/usr"), Path::new("/usr/")));
        assert!(!path_equals_root(
            Path::new("/usr"),
            Path::new("/usr/local")
        ));
        assert!(!path_equals_root(
            Path::new("/usr"),
            Path::new("/usr/local/bin/tool")
        ));
        assert!(!path_equals_root(Path::new("/usr"), Path::new("/usrx")));
        assert!(path_equals_root(Path::new("/"), Path::new("/usr/..")));
        assert!(is_volume_root(Path::new("/")));
        assert!(is_volume_root(Path::new("C:\\")));
        assert!(is_volume_root(Path::new("D:/")));
        assert!(!is_volume_root(Path::new("/usr")));
        assert!(!is_volume_root(Path::new("relative/x")));
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
        assert!(path_hits_root(
            Path::new("/boot"),
            Path::new("/boot/grub/x")
        ));
    }

    #[test]
    fn disaster_roots_env_fallback_binds_literals_and_closed_sets_hold() {
        let fallback = windows_disaster_tree_roots_with(None, None, None, None);
        assert_eq!(fallback[0], PathBuf::from(r"C:\Windows"));
        assert_eq!(fallback[3], PathBuf::from(r"C:\ProgramData"));
        let overridden =
            windows_disaster_tree_roots_with(Some(r"D:\Win"), None, Some("  "), Some(r"E:\PD"));
        assert_eq!(overridden[0], PathBuf::from(r"D:\Win"));
        assert_eq!(overridden[2], PathBuf::from(r"C:\Program Files (x86)"));
        assert_eq!(overridden[3], PathBuf::from(r"E:\PD"));
        assert_eq!(
            linux_disaster_tree_roots().len(),
            LINUX_DISASTER_TREE_ROOTS.len()
        );
        // 封闭集钉（0cb 防膨胀）：根本树根恰 12 项（设计 §1 规则 1 目标集）。
        assert_eq!(LINUX_DISASTER_TREE_ROOTS.len(), 12);
        assert!(LINUX_DISASTER_TREE_ROOTS.contains(&"/var"));
        assert!(!LINUX_DISASTER_TREE_ROOTS.contains(&"/libx32"));
        // 0cb 审查处理批（钉覆盖补全）：根本树根字面回退表行数钉。
        assert_eq!(WINDOWS_DISASTER_TREE_ROOT_FALLBACKS.len(), 4);
        // 0cc v3（S2 落码）：L3 排除集收窄＝灾难防护最小核恰 4 项——载体面
        // 系统树（/etc /usr /lib* /bin /sbin）已放行（S4 成败项），旧
        // `LINUX_SYSTEM_CORE` 十二树表退役。
        assert_eq!(LINUX_DISASTER_KERNEL_FACES.len(), 4);
        for face in ["/boot", "/dev", "/proc", "/sys"] {
            assert!(LINUX_DISASTER_KERNEL_FACES.contains(&face));
        }
        for retired in [
            "/etc", "/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32",
        ] {
            assert!(
                !LINUX_DISASTER_KERNEL_FACES.contains(&retired),
                "{retired} must be retired from the L3 exclusion set"
            );
        }
    }

    /// 0ch v4 子项①封闭表钉：恰 7 项、全为 `/dev/` 前缀单节点、危险节点
    /// 形态不入表（新增表项必改本测试）。
    #[test]
    fn device_safe_nodes_closed_set_holds() {
        assert_eq!(DEVICE_SAFE_NODES.len(), 7);
        for node in [
            "/dev/null",
            "/dev/zero",
            "/dev/full",
            "/dev/tty",
            "/dev/random",
            "/dev/urandom",
            "/dev/ptmx",
        ] {
            assert!(
                DEVICE_SAFE_NODES.contains(&node),
                "{node} must stay in DEVICE_SAFE_NODES"
            );
        }
        for node in DEVICE_SAFE_NODES {
            assert!(
                node.starts_with("/dev/") && !node[5..].contains('/'),
                "table entries must be single /dev nodes: {node}"
            );
        }
        // 危险形态负向集：块设备族、原始内存/端口族、帧缓冲/回环不入表
        //（这些写面维持 L3 内核拒——规则 2 灾难面）。
        for bad in [
            "/dev/sd",
            "/dev/vd",
            "/dev/nvme",
            "/dev/mmcblk",
            "/dev/mapper",
            "/dev/mem",
            "/dev/kmem",
            "/dev/port",
            "/dev/loop",
            "/dev/fb",
            "/dev/console",
        ] {
            assert!(
                !DEVICE_SAFE_NODES.contains(&bad),
                "{bad} must NOT be in DEVICE_SAFE_NODES"
            );
            assert!(
                !DEVICE_SAFE_NODES.iter().any(|n| n.starts_with(bad)),
                "no {bad}* form may enter DEVICE_SAFE_NODES"
            );
        }
    }

    #[test]
    fn keystore_root_subtree_hits_and_siblings_stay_allowed() {
        let keystore = PathBuf::from(r"D:\acaf\keystore");
        let targets = HostStateTargets {
            keystore_root: Some(keystore.clone()),
            signer_manifest: None,
        };
        let forms = candidate_forms(&keystore.join("installation-key.json"), None);
        let hit = targets.hit(&forms).expect("keystore subtree hit");
        assert_eq!(hit.rule, "carrier:keystore-root");
        // 根本体同拒（包含语义）。
        let root_forms = candidate_forms(&keystore, None);
        assert!(targets.hit(&root_forms).is_some());
        // 兄弟目录（含旧安装目录形态）不落 keystore 根 ⇒ 不拦。
        let sibling = candidate_forms(&keystore.parent().unwrap().join("orz.exe"), None);
        assert!(targets.hit(&sibling).is_none());
    }

    #[test]
    fn signer_manifest_hits_only_the_file_itself() {
        let manifest = PathBuf::from(r"D:\acaf\signer-manifest.json");
        let targets = HostStateTargets {
            keystore_root: None,
            signer_manifest: Some(manifest.clone()),
        };
        let file_forms = candidate_forms(&manifest, None);
        let hit = targets.hit(&file_forms).expect("manifest hit");
        assert_eq!(hit.rule, "carrier:signer-manifest");
        // manifest 所在目录的其他文件不拦（窄目标＝恰为本体）。
        let sibling = candidate_forms(&manifest.parent().unwrap().join("other.txt"), None);
        assert!(targets.hit(&sibling).is_none());
    }

    #[test]
    fn keystore_root_takes_precedence_over_manifest() {
        let keystore = PathBuf::from(r"D:\acaf\keystore");
        let manifest = keystore.parent().unwrap().join("signer-manifest.json");
        let targets = HostStateTargets {
            keystore_root: Some(keystore.clone()),
            signer_manifest: Some(manifest),
        };
        // keystore 内目标：两表都可命中形状下报 keystore 根（优先序钉）。
        let forms = candidate_forms(&keystore.join("x.bin"), None);
        assert_eq!(targets.hit(&forms).unwrap().rule, "carrier:keystore-root");
    }

    /// 0cc v3.1 审查处理批（P2 祖先链臂）：扫荡删除/搬移受护目标的祖先即命中；
    /// 非祖先不命中；直接命中优先于祖先命中；[`check_write_target`]（工具面）
    /// 不含祖先臂。
    #[test]
    fn ancestor_sweep_hits_and_non_ancestors_stay_allowed() {
        let keystore = PathBuf::from(r"D:\app\orz\acaf\keystore");
        let manifest = PathBuf::from(r"D:\app\orz\acaf\signer-manifest.json");
        let targets = HostStateTargets {
            keystore_root: Some(keystore.clone()),
            signer_manifest: Some(manifest.clone()),
        };
        // 安装目录＝两受护目标的共同祖先（实况形态：keystore 在已退役的载体
        // 安装目录内）——命中且报 keystore 祖先（keystore 优先于 manifest）。
        let install = PathBuf::from(r"D:\app\orz");
        let hit = targets
            .hit_ancestor(&candidate_forms(&install, None))
            .expect("ancestor hit");
        assert_eq!(hit.rule, "carrier:keystore-ancestor");
        assert_eq!(hit.root, keystore.to_string_lossy());
        // 中间父目录（acaf）同命中；manifest 专属祖先形态（keystore 未武装时）
        // 报 manifest 祖先。
        let acaf = install.join("acaf");
        assert_eq!(
            targets
                .hit_ancestor(&candidate_forms(&acaf, None))
                .unwrap()
                .rule,
            "carrier:keystore-ancestor"
        );
        let manifest_only = HostStateTargets {
            keystore_root: None,
            signer_manifest: Some(manifest.clone()),
        };
        assert_eq!(
            manifest_only
                .hit_ancestor(&candidate_forms(&acaf, None))
                .unwrap()
                .rule,
            "carrier:signer-manifest-ancestor"
        );
        // 非祖先：兄弟子树与无关节——放行。
        for unrelated in [r"D:\app\other", r"D:\app\orz\bin"] {
            assert!(
                targets
                    .hit_ancestor(&candidate_forms(Path::new(unrelated), None))
                    .is_none(),
                "non-ancestor must stay allowed: {unrelated}"
            );
        }
        // 直接命中不被祖先臂抢报：keystore 子树形态 hit_ancestor 判 None、
        // hit 报 keystore-root（调用方先 hit 后 hit_ancestor 的优先序基础）。
        let direct_forms = candidate_forms(&keystore.join("k.json"), None);
        assert!(targets.hit_ancestor(&direct_forms).is_none());
        assert_eq!(
            targets.hit(&direct_forms).unwrap().rule,
            "carrier:keystore-root"
        );
        // 工具面唯一入口不含祖先臂：祖先目录形态在 check_write_target 判 None
        // （工具面目标恒为文件；祖先拒面由命令面扫荡动词门控承载）。
        let cwd = std::env::temp_dir();
        assert!(
            check_write_target(&WriteTargetCtx {
                cwd: &cwd,
                joined: &install,
                resolved: None,
                host_state: &targets,
            })
            .is_none(),
            "tool face has no ancestor arm"
        );
        // 祖先命中文案带规则 id 与「ancestor」语义（可区分来源标识钉）。
        let msg = write_block_message("x", &hit);
        assert!(msg.contains("ancestor of the ACAF keystore root"));
        assert!(msg.contains("rule: carrier:keystore-ancestor"));
    }

    #[test]
    fn host_state_targets_env_parsing_skips_blank() {
        let key = "ORZ_ACAF_KEYSTORE";
        let prev = std::env::var(key).ok();
        // SAFETY: 测试单线程段内改装配 env（值即时还原）；edition 2024 起
        // set_var/remove_var 为 unsafe。
        unsafe {
            std::env::set_var(key, "  ");
        }
        assert!(HostStateTargets::from_env().keystore_root.is_none());
        unsafe {
            std::env::set_var(key, r"D:\acaf\keystore-env-probe");
        }
        assert_eq!(
            HostStateTargets::from_env().keystore_root,
            Some(PathBuf::from(r"D:\acaf\keystore-env-probe"))
        );
        unsafe {
            match prev {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
    }

    #[test]
    fn session_volume_domain_is_rejected_and_workspace_allowed() {
        let tmp = std::env::temp_dir().join("0bw-write-control-gsa");
        let _ = std::fs::create_dir_all(tmp.join(".gsa"));
        let targets = HostStateTargets::default();
        let hit = check_write_target(&WriteTargetCtx {
            cwd: &tmp,
            joined: &tmp.join(".gsa").join("journal").join("x.md"),
            resolved: None,
            host_state: &targets,
        })
        .expect("session volume hit");
        assert_eq!(hit.rule, "carrier:session-volume");
        assert!(
            check_write_target(&WriteTargetCtx {
                cwd: &tmp,
                joined: &tmp.join("src").join("x.md"),
                resolved: None,
                host_state: &targets,
            })
            .is_none()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn check_write_target_is_host_state_only_since_v3() {
        // v3 收窄钉：① 系统树写不拒（0cb 退役面保持）；② 载体安装目录／三件套
        // ／`grok-home` 写不拒（0cc 退役面——宿主机灾难保底不再含载体本体）；
        // ③ 宿主状态窄目标仍拒。
        let cwd = std::env::temp_dir();
        let targets = HostStateTargets::default();
        let roots = disaster_tree_roots();
        let system_target = roots[0].join("Temp").join("0cc-host-state-only-probe.txt");
        assert!(
            check_write_target(&WriteTargetCtx {
                cwd: &cwd,
                joined: &system_target,
                resolved: None,
                host_state: &targets,
            })
            .is_none(),
            "system-tree write must not be denied ({})",
            system_target.display()
        );
        // 载体面退役放行（一行回归）：安装目录／三件套／grok-home 形态全放行。
        for retired in [
            r"D:\app\orz\orz.exe",
            r"D:\app\orz\orz-signer.exe",
            r"D:\app\orz\grok-home\creds.json",
            r"D:\app\orz\_bgprobe.exe",
        ] {
            assert!(
                check_write_target(&WriteTargetCtx {
                    cwd: &cwd,
                    joined: Path::new(retired),
                    resolved: None,
                    host_state: &targets,
                })
                .is_none(),
                "carrier-face write must be retired to allow: {retired}"
            );
        }
        // 宿主状态仍拒。
        let keystore = PathBuf::from(r"D:\acaf\keystore");
        let armed = HostStateTargets {
            keystore_root: Some(keystore.clone()),
            signer_manifest: None,
        };
        let tmp = std::env::temp_dir().join("0cc-write-control-gsa");
        let _ = std::fs::create_dir_all(tmp.join(".gsa"));
        let hit = check_write_target(&WriteTargetCtx {
            cwd: &tmp,
            joined: &tmp.join(".gsa").join("journal").join("x.md"),
            resolved: None,
            host_state: &armed,
        })
        .expect("session volume hit");
        assert_eq!(hit.rule, "carrier:session-volume");
        let ks_hit = check_write_target(&WriteTargetCtx {
            cwd: &tmp,
            joined: &keystore.join("installation-key.json"),
            resolved: None,
            host_state: &armed,
        })
        .expect("keystore hit");
        assert_eq!(ks_hit.rule, "carrier:keystore-root");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn block_message_carries_rule_identity_per_face() {
        let session = DenyHit {
            rule: "carrier:session-volume",
            root: ".gsa session volume".to_owned(),
            target: "x".to_owned(),
        };
        assert!(write_block_message("x", &session).contains("not model-writable"));
        let keystore = DenyHit {
            rule: "carrier:keystore-root",
            root: r"D:\acaf\keystore".to_owned(),
            target: "x".to_owned(),
        };
        let msg = write_block_message("x", &keystore);
        assert!(msg.contains("ACAF keystore root"));
        assert!(msg.contains("rule: carrier:keystore-root"));
        let manifest = DenyHit {
            rule: "carrier:signer-manifest",
            root: r"D:\acaf\signer-manifest.json".to_owned(),
            target: "x".to_owned(),
        };
        let msg = write_block_message("x", &manifest);
        assert!(msg.contains("signer manifest"));
        assert!(msg.contains("rule: carrier:signer-manifest"));
    }

    #[test]
    fn best_effort_canonical_resolves_nearest_existing_ancestor() {
        let tmp = std::env::temp_dir().join("0bw-best-effort-canon");
        let _ = std::fs::create_dir_all(tmp.join("sub"));
        let probe = tmp.join("sub").join("not-yet.txt");
        let canon = best_effort_canonical(&probe);
        assert_eq!(canon.file_name().unwrap(), "not-yet.txt");
        assert_eq!(
            canon.parent().unwrap(),
            dunce::canonicalize(tmp.join("sub")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

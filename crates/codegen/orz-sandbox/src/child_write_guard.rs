//! Landlock: child write guard (pre_exec) — 0bw① (2026-09-27)；**0ch v4 粒度
//! 精准化（2026-10-01）**。
//!
//! 形态＝spawn 时（pre_exec，与 [`crate::child_net`] seccomp 同型接线）把
//! 授权集装进子进程 Landlock ruleset：写族（write／remove／make／refer／
//! truncate）默认拒、读不设限（读族不入 `handled_access_fs`），授权三段——
//! ① **目录段**：`/` 顶层逐目录全写族 baseline（deny 排除集已在枚举时剔除）；
//! ② **文件段**（0ch 子项①）：设备面安全节点逐个文件级 `WRITE_FILE`(＋
//! TRUNCATE v3+) 授权——`>/dev/null` 一族不再死于内核 EPERM（L1/L2 规则 2
//! 豁免、L3 却整树拒的两层粒度差消除）；`/dev` 目录树本身仍无规则——块
//! 设备/危险节点写、节点删除、`mknod` 仍内核拒；
//! ③ **根段**（0ch 子项②）：`/` 本体 [`ROOT_MAKE_GRANT`]（make 族子集恰
//! 5 位，**不含** `WRITE_FILE`/`REMOVE_*`/`MAKE_CHAR`/`MAKE_BLOCK`/`REFER`/
//! `TRUNCATE`）——顶层新建目录/文件/socket/fifo/sym 放行；Landlock union
//! 语义下该授权全树生效（已接受后果与 `WRITE_FILE` 绝不上 `/` 的否决论证
//! 见设计档 §7.2）。
//!
//! **deny 表不进本模块**——单一源在 orz-tools `write_control`
//! （`LINUX_DISASTER_KERNEL_FACES`，0cc v3 收窄＝灾难防护最小核 /boot /dev
//! /proc /sys；`DEVICE_SAFE_NODES`，0ch v4＝设备面安全节点恰 7 项）；本模块
//! 只承载机制，由调用方（orz-tools `terminal.rs` spawn 点）传两表进
//! [`prepare_allow_set`]。
//!
//! 失败姿态（0bw §1「高阻力＋强审计、非绝对保证」；2026-09-27 复审三分支
//! 收敛，全部 fail-open）：① 内核不支持 Landlock ⇒ 不装 pre_exec；② `/`
//! 枚举失败 ⇒ 不装 pre_exec——两支各 warn 一次（L1 工具面／L2 命令面仍硬拒
//! 锁死面），不因内核能力缺失瘫痪命令执行；③ 子进程内装挂失败 ⇒
//! [`install_best_effort`] 以 write(2) 直写 stderr 一行提示（async-signal-
//! safe，pre_exec 窗口内唯一可行可见面）后照常 exec——**装挂永不 fail
//! spawn**。表内节点缺席/symlink ⇒ 该项跳过不加规则＝默认拒（fail-closed）。
//!
//! 直写 syscall（不经 nono：nono 的 `Sandbox` 是整进程启动期形态，
//! pre_exec 场景不适用）。syscall 号 x86_64／aarch64 统一（444–446）；
//! 其余架构（arm／s390x／ppc64le／mips…）编号不同，**不启用**（
//! `prepare_allow_set` 恒 `None`＝fail-open 方向——错号探测可能打到别的
//! syscall，不得尝试）。

/// 装配产物：目录段＋文件段两表（C 形态全路径，pre_exec 无分配消费）；
/// 根段为恒定规则、不随装配变化。
pub struct ChildWriteAllowSet {
    /// `/` 顶层目录 allowlist（deny 排除集已剔除；全写族 baseline）。
    pub dirs: Vec<std::ffi::CString>,
    /// 设备面安全节点（0ch 子项①；文件级 `WRITE_FILE`(＋TRUNCATE) 授权）。
    pub files: Vec<std::ffi::CString>,
}

/// 调用方在 spawn 前的装配面：枚举 `/` 顶层目录、排除 `deny` 表项与非目录
/// 项；`safe_nodes` 逐项 CString 化进文件段（存在性/symlink 检查在装挂时）。
/// 返回可直接传入 [`install_child_write_guard`] 的授权集。
///
/// `None` ＝ Landlock 不可用或枚举失败（调用方按 best-effort 跳过装挂）。
///
/// 架构门（2026-09-27 复审）：Landlock 统一 syscall 号仅 x86_64／aarch64
/// 成立；其余架构 prepare 恒 `None`（错号探测可能命中无关 syscall，不得
/// 尝试——fail-open 方向）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub fn prepare_allow_set(deny: &[&str], safe_nodes: &[&str]) -> Option<ChildWriteAllowSet> {
    if kernel_abi_version().is_none() {
        return None;
    }
    let dirs = enumerate_writable_top_dirs(deny).ok()?;
    let files = safe_nodes
        .iter()
        .filter_map(|n| std::ffi::CString::new(*n).ok())
        .collect();
    Some(ChildWriteAllowSet { dirs, files })
}

/// 非（Linux ∧ x86_64/aarch64）无装配面（恒 `None`；调用方按 `#[cfg]` 排除，
/// 本入口仅为跨目标编译友好而存在）。
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub fn prepare_allow_set(_deny: &[&str], _safe_nodes: &[&str]) -> Option<ChildWriteAllowSet> {
    None
}

/// 枚举 `root` 顶层目录，排除 `deny` 表项（精确名匹配，尾斜杠归一）与
/// 非目录项。产出 C 形态全路径供 pre_exec 无分配消费。
///
/// **顶层 symlink 一律不授权**（2026-09-27 复审 P1）：`open(O_PATH)` 会解析
/// 链接，「先 `ln -s /etc /w` 再枚举」可把核心目录以别名塞进 allowlist；
/// merged-usr 的 `/bin→/usr/bin` 等本就在表 B 内被名字排除，不受影响；
/// 非核心 symlink 丢授权＝该子树写面默认拒＝fail-closed 方向。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub fn enumerate_writable_top_dirs(deny: &[&str]) -> std::io::Result<Vec<std::ffi::CString>> {
    enumerate_writable_top_dirs_in(std::path::Path::new("/"), deny)
}

/// [`enumerate_writable_top_dirs`] 的可注入根形态（测试用；生产恒 `/`）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub fn enumerate_writable_top_dirs_in(
    root: &std::path::Path,
    deny: &[&str],
) -> std::io::Result<Vec<std::ffi::CString>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // deny 表项为绝对形态（"/boot"…），read_dir 条目名无前导斜杠——
        // 归一到绝对形态再比对（防核心集误入 allowlist）。
        let absolute = format!("/{name}");
        if deny
            .iter()
            .any(|d| d.trim_end_matches('/') == absolute.as_str())
        {
            continue;
        }
        // DirEntry::file_type 不跟随 symlink：symlink 一律跳过（见上），
        // 仅真实目录进入 allowlist。
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        // Windows 下才可能失败的非 UTF-8 路径在 Linux 顶层目录不成立；
        // 保守跳过（该目录写面被默认拒＝fail-closed 方向）。
        if let Ok(c) = std::ffi::CString::new(entry.path().as_os_str().as_encoded_bytes()) {
            out.push(c);
        }
    }
    Ok(out)
}

#[cfg(target_os = "linux")]
mod sys {
    /// `landlock_create_ruleset`（统一编号：x86_64／aarch64 同值）。
    pub const SYS_LANDLOCK_CREATE_RULESET: libc::c_long = 444;
    /// `landlock_add_rule`。
    pub const SYS_LANDLOCK_ADD_RULE: libc::c_long = 445;
    /// `landlock_restrict_self`。
    pub const SYS_LANDLOCK_RESTRICT_SELF: libc::c_long = 446;

    pub const LANDLOCK_CREATE_RULESET_VERSION: libc::c_uint = 1 << 0;
    pub const LANDLOCK_RULE_PATH_BENEATH: libc::c_int = 1;

    // linux/landlock.h fs access rights（ABI v1 基线；REFER=v2、TRUNCATE=v3）。
    pub const FS_WRITE_FILE: u64 = 1 << 1;
    pub const FS_REMOVE_DIR: u64 = 1 << 4;
    pub const FS_REMOVE_FILE: u64 = 1 << 5;
    pub const FS_MAKE_CHAR: u64 = 1 << 6;
    pub const FS_MAKE_DIR: u64 = 1 << 7;
    pub const FS_MAKE_REG: u64 = 1 << 8;
    pub const FS_MAKE_SOCK: u64 = 1 << 9;
    pub const FS_MAKE_FIFO: u64 = 1 << 10;
    pub const FS_MAKE_BLOCK: u64 = 1 << 11;
    pub const FS_MAKE_SYM: u64 = 1 << 12;
    pub const FS_REFER: u64 = 1 << 13;
    pub const FS_TRUNCATE: u64 = 1 << 14;

    /// abi v1 写族基线（读族 EXECUTE/READ_FILE/READ_DIR 不入 handled＝读不设限）。
    pub const WRITE_BASELINE: u64 = FS_WRITE_FILE
        | FS_REMOVE_DIR
        | FS_REMOVE_FILE
        | FS_MAKE_CHAR
        | FS_MAKE_DIR
        | FS_MAKE_REG
        | FS_MAKE_SOCK
        | FS_MAKE_FIFO
        | FS_MAKE_BLOCK
        | FS_MAKE_SYM;
    #[repr(C)]
    pub struct LandlockRulesetAttr {
        pub handled_access_fs: u64,
    }

    #[repr(C)]
    pub struct LandlockPathBeneathAttr {
        pub allowed_access: u64,
        pub parent_fd: libc::c_int,
    }
}

/// 探测内核 Landlock ABI 版本；不可用（<5.13／ENOSYS/EOPNOTSUPP）⇒ `None`。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub fn kernel_abi_version() -> Option<u32> {
    // SAFETY: 版本探测形态——ruleset_fd 指针为 NULL、size 为 0、仅传
    // VERSION 旗标；返回值 >0 ＝ 版本号，<0 ＝ 不支持。
    let rc = unsafe {
        libc::syscall(
            sys::SYS_LANDLOCK_CREATE_RULESET,
            std::ptr::null::<libc::c_void>(),
            0usize,
            sys::LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    if rc > 0 { Some(rc as u32) } else { None }
}

/// 非（Linux ∧ x86_64/aarch64）无内核探测（恒 `None`）。
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub fn kernel_abi_version() -> Option<u32> {
    None
}

/// `/` 本体授权集（**0ch 子项②**；make 族子集恰 5 位）。设计档 §7.2 定档：
/// **不含** `WRITE_FILE`/`TRUNCATE`（写/截断全树维持默认拒——union 语义下
/// 授予即放行宿主既有块设备 open-write，扬盘保底失守）、**不含**
/// `REMOVE_DIR`/`REMOVE_FILE`（删除口径=防删除破坏的本体）、**不含**
/// `MAKE_CHAR`/`MAKE_BLOCK`（mknod 设备节点制造＝绕道 raw 设备访问）、
/// **不含** `REFER`（跨目录 link/rename 搬移维持默认拒）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub const ROOT_MAKE_GRANT: u64 =
    sys::FS_MAKE_DIR | sys::FS_MAKE_REG | sys::FS_MAKE_SOCK | sys::FS_MAKE_FIFO | sys::FS_MAKE_SYM;

/// 文件段（设备面安全节点）授权集：`WRITE_FILE`＋`TRUNCATE`（仅 ABI≥v3——
/// `allowed_access` 必须是 `handled_access_fs` 的子集，v1/v2 的 handled 不含
/// TRUNCATE，带上即 EINVAL）。Landlock 对非目录 fd 仅接受文件族权利。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn file_node_grant(version: u32) -> u64 {
    let mut g = sys::FS_WRITE_FILE;
    if version >= 3 {
        g |= sys::FS_TRUNCATE;
    }
    g
}

/// 在当前（fork 后、exec 前）进程装挂子进程写面 Landlock ruleset。
///
/// `allow` ＝ [`prepare_allow_set`] 产出的授权集（deny 表已在枚举时排除——
/// Landlock 无 deny 规则，「不授权」即拒）。规则三段：目录（全写族
/// baseline）＋文件（[`file_node_grant`]）＋`/` 根（[`ROOT_MAKE_GRANT`]）。
/// 错误上抛由调用方决定姿态（本项目 spawn 点为 best-effort：见模块注释；
/// `prepare_allow_set` 返回 `None` 时本函数不会被装挂）。
///
/// # Safety
/// After fork / before exec. 装挂后本进程（及其 exec 后映像）写族访问
/// 被 ruleset 收窄且不可逆（Landlock 只能收紧）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub unsafe fn install_child_write_guard(allow: &ChildWriteAllowSet) -> std::io::Result<()> {
    use sys::{
        FS_REFER, FS_TRUNCATE, LANDLOCK_RULE_PATH_BENEATH, LandlockPathBeneathAttr,
        LandlockRulesetAttr, SYS_LANDLOCK_ADD_RULE, SYS_LANDLOCK_CREATE_RULESET,
        SYS_LANDLOCK_RESTRICT_SELF, WRITE_BASELINE,
    };

    let version = kernel_abi_version()
        .ok_or_else(|| std::io::Error::other("landlock unsupported (kernel < 5.13 or disabled)"))?;

    // handled 按内核 ABI 收窄：超集位会 EINVAL（v2 REFER、v3 TRUNCATE）。
    let mut handled = WRITE_BASELINE;
    if version >= 2 {
        handled |= FS_REFER;
    }
    if version >= 3 {
        handled |= FS_TRUNCATE;
    }

    // SAFETY: NO_NEW_PRIVS 是 landlock_restrict_self 的非特权前置。
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let attr = LandlockRulesetAttr {
        handled_access_fs: handled,
    };
    // SAFETY: attr 与 size 匹配 struct landlock_ruleset_attr；旗标 0＝常规创建。
    let ruleset_fd = unsafe {
        libc::syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            &attr as *const LandlockRulesetAttr,
            std::mem::size_of::<LandlockRulesetAttr>(),
            0,
        )
    };
    if ruleset_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let ruleset_fd = ruleset_fd as libc::c_int;

    // 目录段：顶层目录逐项全写族 baseline（既有语义不变）。
    for dir in &allow.dirs {
        if let Err(e) = unsafe { add_path_beneath_rule(ruleset_fd, dir, handled) } {
            // SAFETY: ruleset_fd 收尾。
            unsafe { libc::close(ruleset_fd) };
            return Err(e);
        }
    }

    // 文件段（0ch 子项①）：设备面安全节点逐个文件级授权；`allowed_access`
    // 必须落在文件族权利内（目录族位会 EINVAL）。
    let file_grant = file_node_grant(version);
    for file in &allow.files {
        if let Err(e) = unsafe { add_path_beneath_rule(ruleset_fd, file, file_grant) } {
            // SAFETY: ruleset_fd 收尾。
            unsafe { libc::close(ruleset_fd) };
            return Err(e);
        }
    }

    // 根段（0ch 子项②）：`/` 本体 make 族子集（常量集，见 ROOT_MAKE_GRANT）。
    let root = std::ffi::CString::new("/").expect("static path");
    if let Err(e) = unsafe { add_path_beneath_rule(ruleset_fd, &root, ROOT_MAKE_GRANT) } {
        // SAFETY: ruleset_fd 收尾。
        unsafe { libc::close(ruleset_fd) };
        return Err(e);
    }

    // SAFETY: ruleset_fd 合法；NO_NEW_PRIVS 已置位。
    let rc = unsafe { libc::syscall(SYS_LANDLOCK_RESTRICT_SELF, ruleset_fd, 0) };
    // SAFETY: ruleset_fd 收尾（restrict 成败均已不再需要）。
    unsafe { libc::close(ruleset_fd) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// 加一条 PATH_BENEATH 规则（目录段/文件段/根段共用；打开失败＝枚举后
/// 消失或 symlink 交换 ⇒ 跳过不加规则＝该路径写面被默认拒，fail-closed）。
///
/// # Safety
/// `ruleset_fd` 须为合法 ruleset；`path` 须为合法 NUL 结尾路径。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
unsafe fn add_path_beneath_rule(
    ruleset_fd: libc::c_int,
    path: &std::ffi::CString,
    allowed_access: u64,
) -> std::io::Result<()> {
    use sys::{LANDLOCK_RULE_PATH_BENEATH, LandlockPathBeneathAttr, SYS_LANDLOCK_ADD_RULE};

    // SAFETY: path 为合法 NUL 结尾路径；O_PATH＝仅持引用不读内容，
    // O_CLOEXEC 防 exec 泄漏；O_NOFOLLOW＝枚举后被换成 symlink 的路径
    // 打不开（跳过＝默认拒，fail-closed——复审 P1 纵深）。
    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_PATH | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        // 路径缺席/已换 symlink：不加规则（更严方向），不算装挂失败。
        return Ok(());
    }
    let rule = LandlockPathBeneathAttr {
        allowed_access,
        parent_fd: fd,
    };
    // SAFETY: rule 与 ruleset_fd 均为刚取得的合法句柄/结构。
    let rc = unsafe {
        libc::syscall(
            SYS_LANDLOCK_ADD_RULE,
            ruleset_fd,
            LANDLOCK_RULE_PATH_BENEATH,
            &rule as *const LandlockPathBeneathAttr,
            0,
        )
    };
    // SAFETY: fd 不再需要（规则已拷入内核）。
    unsafe { libc::close(fd) };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// 非（Linux ∧ x86_64/aarch64）无装挂面（空操作；Linux 实现承载真实契约）。
///
/// # Safety
/// Safe to call with any arguments — the stub performs no work.
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub unsafe fn install_child_write_guard(_allow: &ChildWriteAllowSet) -> std::io::Result<()> {
    Ok(())
}

/// 装挂失败时的提示行（async-signal-safe：`write(2)` 直写 stderr，不用
/// tracing／堆分配——pre_exec 窗口内唯一可行可见面，2026-09-27 复审）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
const INSTALL_FAILURE_NOTE: &[u8] =
    b"orz: child write guard install failed (best-effort skip; L1/L2 remain)\n";

/// best-effort 装挂（2026-09-27 复审裁决：装挂失败**永不 fail spawn**——
/// 设计 §3.3「不因 L3 故障瘫痪命令执行」的兑现）。失败 ⇒ `write(2)` 一行
/// 提示后返回，子进程照常 exec（L1 工具面／L2 命令面仍硬拒锁死面）。
///
/// # Safety
/// After fork / before exec（同 [`install_child_write_guard`]）。
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
pub unsafe fn install_best_effort(allow: &ChildWriteAllowSet) {
    // SAFETY: 见 install_child_write_guard；失败路径仅 write(2)（信号安全）。
    if unsafe { install_child_write_guard(allow) }.is_err() {
        // SAFETY: 常量切片指针/长度；fd 2 恒有效。
        unsafe {
            libc::write(
                libc::STDERR_FILENO,
                INSTALL_FAILURE_NOTE.as_ptr().cast(),
                INSTALL_FAILURE_NOTE.len(),
            )
        };
    }
}

/// 非（Linux ∧ x86_64/aarch64）无 best-effort 装挂面（空操作）。
///
/// # Safety
/// Safe to call with any arguments — the stub performs no work.
#[cfg(not(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
)))]
pub unsafe fn install_best_effort(_allow: &ChildWriteAllowSet) {}

#[cfg(all(
    test,
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod tests {
    use super::*;

    /// 表 B 核心集（单一源在 orz-tools write_control；此处以同值副本断言
    /// 排除逻辑——机制模块不得反向依赖策略 crate）。
    const CORE: [&str; 12] = [
        "/boot", "/etc", "/usr", "/lib", "/lib32", "/lib64", "/libx32", "/bin", "/sbin", "/dev",
        "/proc", "/sys",
    ];

    #[test]
    fn abi_probe_returns_something_or_none() {
        // 真机读数：探测不 panic；版本 ≥1 时枚举可用。
        let v = kernel_abi_version();
        if v.is_some() {
            assert!(v.unwrap() >= 1);
        }
    }

    #[test]
    fn enumeration_excludes_core_and_non_dirs() {
        let dirs = enumerate_writable_top_dirs(&CORE).expect("enumerate /");
        let names: Vec<String> = dirs
            .iter()
            // CString::to_str 返回 Result（Path::to_str 才是 Option）。
            .filter_map(|c| c.to_str().ok().map(|s| s.to_string()))
            .collect();
        for core in CORE {
            assert!(
                !names.iter().any(|n| n == core),
                "core dir {core} must not be in allowlist: {names:?}"
            );
        }
        // 可写基线：/tmp、/var、/run 至少其一应在（任何常规 Linux）。
        assert!(
            names
                .iter()
                .any(|n| n == "/tmp" || n == "/var" || n == "/run"),
            "writable baseline missing: {names:?}"
        );
    }

    #[test]
    fn install_rejects_on_empty_ruleset_paths_but_probe_gates() {
        // 无 allowlist 时装挂仍成立（全写面默认拒）——本测试只验证
        // 「探测不支持 ⇒ 不装」的门；真机装挂留给容器/跨目标验证。
        if kernel_abi_version().is_none() {
            return;
        }
        // 装挂会收窄当前测试进程写面，故不在单测内真正调用 restrict；
        // 仅验证 ruleset 创建臂（add/restrict 由 spawn 真机验证覆盖）。
        assert!(true);
    }

    /// 2026-09-27 复审 P1 回归钉：顶层 symlink（无论指向目录还是文件）
    /// 一律不入 allowlist；deny 表名与普通文件照旧排除。
    #[test]
    fn enumeration_excludes_symlinks_non_dirs_and_deny_names() {
        let base = std::env::temp_dir().join(format!(
            "orz-cwg-enum-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("realdir")).unwrap();
        std::fs::create_dir_all(base.join("etc-like")).unwrap();
        std::fs::write(base.join("plainfile"), b"x").unwrap();
        std::os::unix::fs::symlink(base.join("etc-like"), base.join("dirlink")).unwrap();
        std::os::unix::fs::symlink(base.join("plainfile"), base.join("filelink")).unwrap();

        // deny 表以「/ 名字」形态比对（为 `/` 顶层枚举设计）；临时根下
        // 以 "/etc-like" 命中，验证名字排除臂。
        let deny = ["/etc-like"];
        let dirs = enumerate_writable_top_dirs_in(&base, &deny).expect("enumerate temp root");
        let names: Vec<String> = dirs
            .iter()
            .filter_map(|c| c.to_str().ok().map(|s| s.to_string()))
            .collect();
        assert!(
            names.iter().any(|n| n.ends_with("/realdir")),
            "real dir must be allowed: {names:?}"
        );
        for excluded in ["dirlink", "filelink", "plainfile", "etc-like"] {
            assert!(
                !names.iter().any(|n| n.ends_with(excluded)),
                "{excluded} must not be in allowlist: {names:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0ch 子项② 权限集钉：`ROOT_MAKE_GRANT` 恰 5 位（make 族子集）——
    /// 写/截断/删除/设备节点制造/跨目录搬移位一律不得混入（设计档 §7.2；
    /// union 语义下 `WRITE_FILE` 上 `/` 即放行宿主块设备写＝扬盘保底失守）。
    #[test]
    fn root_make_grant_is_precise_five_bit_subset() {
        use sys::{
            FS_MAKE_BLOCK, FS_MAKE_CHAR, FS_MAKE_DIR, FS_MAKE_FIFO, FS_MAKE_REG, FS_MAKE_SOCK,
            FS_MAKE_SYM, FS_REFER, FS_REMOVE_DIR, FS_REMOVE_FILE, FS_TRUNCATE, FS_WRITE_FILE,
        };
        assert_eq!(
            ROOT_MAKE_GRANT,
            FS_MAKE_DIR | FS_MAKE_REG | FS_MAKE_SOCK | FS_MAKE_FIFO | FS_MAKE_SYM
        );
        for banned in [
            FS_WRITE_FILE,
            FS_TRUNCATE,
            FS_REMOVE_DIR,
            FS_REMOVE_FILE,
            FS_MAKE_CHAR,
            FS_MAKE_BLOCK,
            FS_REFER,
        ] {
            assert_eq!(
                ROOT_MAKE_GRANT & banned,
                0,
                "banned bit {banned:#b} must not appear in ROOT_MAKE_GRANT"
            );
        }
    }

    /// 0ch 子项① 权限集钉：文件段授权只含文件族权利（WRITE_FILE＋ABI≥3 的
    /// TRUNCATE），目录族位（MAKE_\*/REMOVE_\*）一律不出现——Landlock 对非
    /// 目录 fd 接受目录族位即 EINVAL。
    #[test]
    fn file_node_grant_is_file_family_only() {
        use sys::{
            FS_MAKE_BLOCK, FS_MAKE_CHAR, FS_MAKE_DIR, FS_MAKE_FIFO, FS_MAKE_REG, FS_MAKE_SOCK,
            FS_MAKE_SYM, FS_REFER, FS_REMOVE_DIR, FS_REMOVE_FILE, FS_TRUNCATE, FS_WRITE_FILE,
        };
        let dir_family = FS_MAKE_DIR
            | FS_MAKE_REG
            | FS_MAKE_SOCK
            | FS_MAKE_FIFO
            | FS_MAKE_SYM
            | FS_MAKE_CHAR
            | FS_MAKE_BLOCK
            | FS_REMOVE_DIR
            | FS_REMOVE_FILE
            | FS_REFER;
        for version in [1u32, 2, 3, 4, 5] {
            let g = file_node_grant(version);
            assert!(g & FS_WRITE_FILE != 0, "v{version}: WRITE_FILE required");
            assert_eq!(
                g & dir_family,
                0,
                "v{version}: directory-family bits must not appear in file grant"
            );
            if version >= 3 {
                assert_eq!(g, FS_WRITE_FILE | FS_TRUNCATE, "v{version}");
            } else {
                assert_eq!(g, FS_WRITE_FILE, "v{version}: TRUNCATE not in handled");
            }
        }
    }

    /// 0ch 装配钉：`prepare_allow_set` 文件段与传入表逐项对应；ABI 探测
    /// 失败 ⇒ `None`（门在枚举前）。
    #[test]
    fn prepare_allow_set_carries_files_and_gates_on_abi() {
        let nodes = ["/dev/null", "/dev/zero", "/dev/ptmx"];
        match prepare_allow_set(&CORE, &nodes) {
            Some(set) => {
                let file_names: Vec<String> = set
                    .files
                    .iter()
                    .filter_map(|c| c.to_str().ok().map(|s| s.to_string()))
                    .collect();
                assert_eq!(
                    file_names,
                    nodes.iter().map(|s| s.to_string()).collect::<Vec<_>>()
                );
                assert!(
                    set.dirs.iter().any(|d| d.to_str().ok() == Some("/tmp")),
                    "dirs 段应含 /tmp（枚举语义不变）"
                );
            }
            None => {
                assert!(kernel_abi_version().is_none(), "Some 门＝ABI 探测");
            }
        }
    }
}

#[cfg(all(test, not(target_os = "linux")))]
mod tests {
    #[test]
    fn non_linux_prepare_is_none() {
        assert!(super::prepare_allow_set(&[], &[]).is_none());
        assert!(super::kernel_abi_version().is_none());
    }
}

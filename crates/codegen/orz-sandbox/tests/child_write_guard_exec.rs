//! Landlock 装挂 exec 后真机读数（0bw①，2026-09-27 重建批；**0cc v3 收窄
//! 后形态**，2026-09-29；**0ch v4 粒度精准化后形态**，2026-10-01）。
//!
//! `install_rejects_on_empty_ruleset_paths_but_probe_gates`（lib 单测）因
//! 不能在测试进程内 self-restrict 而只验探测门；本集成测试把装挂放进
//! **子进程**（pre_exec 同生产接线），对内核取真实读数：
//! - 目录段（0bw①/0cc v3）：写灾难防护最小核外的 `/tmp` 照常、载体面系统树
//!   （`/usr/local`——S4 成败项）放行；
//! - 文件段（0ch①）：`>/dev/null` 等安全节点写放行；`/dev/console`（表外
//!   既有节点）写仍拒——封闭表的「表外恒拒」面；
//! - 表外块设备（0ch① 审查处理补钉）：既有块设备节点（sd*/vd*/nvme*/
//!   mmcblk*/mapper* 形态）写仍拒——root 门＋块设备形态门归因、**仅守卫臂**
//!   （对照臂 `echo x > /dev/sda` 会真写盘面＝灾难，不设）；无块设备节点
//!   （评测容器常态）⇒ 显式跳过（读数如实，不假绿）；
//! - 根段（0ch②）：顶层 `mkdir`/`touch` 放行；`mknod`（设备节点制造）仍拒；
//!   `rm /dev/null`（安全节点删除）仍拒；新建顶层目录内**同 spawn** 写文件
//!   仍拒（§7.4 枚举期快照边界钉）。
//!
//! 容器/CI 内核 <5.13 或 seccomp 封 landlock syscall 时探测为 `None` ⇒
//! 显式跳过（读数如实，不假绿）；对照臂（无守卫）同败 ⇒ 环境不成立跳过
//! （不假红）。

#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

use std::os::unix::fs::FileTypeExt;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use orz_sandbox::child_write_guard::{
    ChildWriteAllowSet, install_best_effort, kernel_abi_version, prepare_allow_set,
};

/// 灾难防护最小核同值副本（机制 crate 不反向依赖策略 crate；单一源
/// `orz-tools::write_control::LINUX_DISASTER_KERNEL_FACES`，变更须两处同步；
/// 0cc v3——v1 表 B 十二树中的 /etc /usr /lib* /bin /sbin 载体面已放行）。
const CORE: [&str; 4] = ["/boot", "/dev", "/proc", "/sys"];

/// 设备面安全节点同值副本（0ch①；单一源
/// `orz-tools::write_control::DEVICE_SAFE_NODES`，恰 7 项）。
const SAFE_NODES: [&str; 7] = [
    "/dev/null",
    "/dev/zero",
    "/dev/full",
    "/dev/tty",
    "/dev/random",
    "/dev/urandom",
    "/dev/ptmx",
];

/// 本文件**不用** `Command::output()`／`Stdio::null()`：两者把子进程 stdin
/// 落在 open("/dev/null")——`rm /dev/null` 探针的对照臂窗口内该 open 会
/// ENOENT（spawn 面溺死，实测），且那正是 0ch 要修的摩擦本身。统一
/// `stdin(Stdio::piped())`（pipe2，零 /dev/null 依赖）＋`wait_with_output`
/// （stdin 写半即时关闭＝子进程读 stdin 得 EOF）。
fn run_sh(cmd: &mut Command, script: &str) -> Result<std::process::Output, std::io::Error> {
    cmd.arg("-c")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
}

fn spawn_with_guard(script: &str) -> Result<std::process::Output, std::io::Error> {
    let Some(set) = prepare_allow_set(&CORE, &SAFE_NODES) else {
        panic!("prepare_allow_set returned None despite ABI probe success");
    };
    run_prepared(script, set)
}

/// 既有测试实现抽出：装挂闭包对 `ChildWriteAllowSet` 的所有权消费形态与
/// 生产 pre_exec 接线一致（`move` 进闭包）。
fn run_prepared(
    script: &str,
    set: ChildWriteAllowSet,
) -> Result<std::process::Output, std::io::Error> {
    let mut cmd = Command::new("sh");
    // SAFETY: pre_exec 注册钩子（fork 后 exec 前，与生产 spawn 点同型；
    // 闭包体处于外层 unsafe 上下文，install_best_effort 不再嵌套 unsafe 块）。
    unsafe {
        cmd.pre_exec(move || {
            install_best_effort(&set);
            Ok(())
        });
    }
    run_sh(&mut cmd, script)
}

/// 无守卫对照臂（真实权限基线——非 root 环境下区分 Landlock EPERM 与
/// 真 EACCES；对照臂同败 ⇒ 环境不成立，显式跳过读数）。
fn spawn_without_guard(script: &str) -> Result<std::process::Output, std::io::Error> {
    run_sh(&mut Command::new("sh"), script)
}

#[test]
fn exec_guard_denies_kernel_faces_and_allows_tmp_and_carrier_trees() {
    let Some(version) = kernel_abi_version() else {
        eprintln!("SKIP: kernel Landlock unavailable (ABI probe None) — reading not taken");
        return;
    };
    eprintln!("kernel landlock ABI = {version}");

    // 非表内基线：/tmp 写照常成功（allowlist 补集语义的正面）。
    let ok = spawn_with_guard(
        "echo ok > /tmp/orz-landlock-allow-probe && cat /tmp/orz-landlock-allow-probe",
    )
    .expect("spawn /tmp probe");
    assert!(ok.status.success(), "/tmp write must succeed: {ok:?}");
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "ok");
    let _ = std::fs::remove_file("/tmp/orz-landlock-allow-probe");

    // 0cc v3 强约束（S4 成败项）：载体面系统树 /usr/local 写放行——L2 放行的
    // 安装形态不得死于内核 EPERM。对照臂同败（非 root 等真实权限）⇒ 环境
    // 不成立，跳过该臂（不假红）。
    let control = spawn_without_guard(
        "mkdir -p /usr/local/libexec && touch /usr/local/libexec/orz-0cc-allow-probe",
    )
    .expect("spawn /usr/local control probe");
    let _ = std::fs::remove_file("/usr/local/libexec/orz-0cc-allow-probe");
    if control.status.success() {
        let ok = spawn_with_guard(
            "mkdir -p /usr/local/libexec && touch /usr/local/libexec/orz-0cc-allow-probe",
        )
        .expect("spawn /usr/local probe");
        assert!(
            ok.status.success(),
            "/usr/local (carrier tree) write must NOT be denied under the guard: {ok:?}"
        );
        let _ = std::fs::remove_file("/usr/local/libexec/orz-0cc-allow-probe");
    } else {
        eprintln!(
            "SKIP: /usr/local control probe failed without guard — no root perms; reading not taken"
        );
    }

    // 0ch① 修正探针：`/dev` 整树新建随根段 make 授权放行（设计档 §7.2 已
    // 接受后果①），旧「/dev 内不得落盘」负探针退役；表外**既有节点**
    // （/dev/console，不在 DEVICE_SAFE_NODES）open-write 仍内核拒——封闭表
    // 的表外恒拒面。对照臂同败（非 root／console 缺席）⇒ 跳过（不假红）。
    let control =
        spawn_without_guard("echo control > /dev/console").expect("spawn /dev/console control");
    if control.status.success() {
        let denied = spawn_with_guard("echo blocked > /dev/console").expect("spawn console probe");
        assert!(
            !denied.status.success(),
            "/dev/console (not in DEVICE_SAFE_NODES) write must be denied: {denied:?}"
        );
        let stderr = String::from_utf8_lossy(&denied.stderr);
        assert!(
            stderr.contains("Permission denied") || stderr.contains("Operation not permitted"),
            "expected EACCES/EPERM, got: {stderr}"
        );
    } else {
        eprintln!(
            "SKIP: /dev/console control probe failed without guard — no root perms; reading not taken"
        );
    }
}

/// 0ch v4 探针集（文件段＋根段；真机内核读数）。
#[test]
fn exec_0ch_safe_nodes_root_make_and_closed_set_probes() {
    let Some(version) = kernel_abi_version() else {
        eprintln!("SKIP: kernel Landlock unavailable (ABI probe None) — reading not taken");
        return;
    };
    eprintln!("kernel landlock ABI = {version}");

    // 0ch① 正探针：`>/dev/null`（O_WRONLY|O_CREAT|O_TRUNC → WRITE_FILE
    // 〔+TRUNCATE v3+〕）放行——apt/dpkg/git/sshd 摩擦族的第一步。
    let ok = spawn_with_guard("echo ok > /dev/null && echo done").expect("spawn /dev/null probe");
    assert!(
        ok.status.success(),
        "/dev/null write must NOT be denied under the guard: {ok:?}"
    );
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "done");

    // 0ch② 正探针：顶层新建（`/` 本体 make 授权）——`mkdir /git` 一族。
    let ok = spawn_with_guard("mkdir /orz0ch-root-probe && touch /orz0ch-file-probe && echo made")
        .expect("spawn root-make probe");
    assert!(
        ok.status.success(),
        "top-level mkdir/touch must NOT be denied under the guard: {ok:?}"
    );
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "made");
    let _ = std::fs::remove_dir_all("/orz0ch-root-probe");
    let _ = std::fs::remove_file("/orz0ch-file-probe");

    // §7.4 边界钉：新建顶层目录内**同 spawn** 写文件仍拒（新条目不在枚举
    // 快照内、无 WRITE_FILE covering rule；下一 spawn 枚举后获全权）。
    let denied = spawn_with_guard("mkdir /orz0ch-gap-probe && echo x > /orz0ch-gap-probe/f")
        .expect("spawn snapshot-gap probe");
    assert!(
        !denied.status.success(),
        "same-spawn write into a freshly made top dir must stay denied (§7.4 boundary): {denied:?}"
    );
    let _ = std::fs::remove_dir_all("/orz0ch-gap-probe");

    // 0ch① 保底面：设备节点制造（mknod，MAKE_BLOCK 不在根授权集）仍拒。
    let denied = spawn_with_guard("mknod /dev/orz0ch-bprobe b 8 0").expect("spawn mknod probe");
    assert!(
        !denied.status.success(),
        "mknod (device node minting) must be denied under the guard: {denied:?}"
    );
    assert!(
        !std::path::Path::new("/dev/orz0ch-bprobe").exists(),
        "no device node may be minted under the guard"
    );

    // 0ch① 保底面（审查处理 P3-3 补钉，S2 立项口径「/dev/sd* 类仍不可写」的
    // 字面落钉）：表外**既有块设备**写仍拒。只做守卫臂、**不做对照臂**——
    // root 下 `echo x > <块设备>` 会真写盘面，对照臂本身就是灾难；归因改由
    // 「root 门＋块设备形态门」确立（root 对可写块设备本应成功，守卫下
    // EACCES/EPERM 即守卫所拒）。无块设备节点（评测容器常态）或非 root
    // ⇒ 显式跳过（读数如实，不假绿/不假红）。
    let root_probe =
        spawn_without_guard("[ \"$(id -u)\" = 0 ] && echo root").expect("spawn uid probe");
    let is_root = String::from_utf8_lossy(&root_probe.stdout).trim() == "root";
    let block_node = std::fs::read_dir("/dev").ok().and_then(|entries| {
        entries.filter_map(|e| e.ok()).find_map(|e| {
            let name = e.file_name();
            let path = format!("/dev/{}", name.to_string_lossy());
            let on_form = [
                "/dev/sd",
                "/dev/vd",
                "/dev/nvme",
                "/dev/mmcblk",
                "/dev/mapper",
            ]
            .iter()
            .any(|p| path.starts_with(p));
            // file_type 不跟随 symlink：别名/软链不冒充块设备（fail-safe）。
            let is_blk = e.file_type().map(|t| t.is_block_device()).unwrap_or(false);
            (on_form && is_blk).then_some(path)
        })
    });
    match (is_root, block_node) {
        (true, Some(node)) => {
            let denied = spawn_with_guard(&format!("echo x > {node}"))
                .expect("spawn off-table block device probe");
            assert!(
                !denied.status.success(),
                "off-table block device {node} write must stay denied under the guard: {denied:?}"
            );
            let stderr = String::from_utf8_lossy(&denied.stderr);
            assert!(
                stderr.contains("Permission denied") || stderr.contains("Operation not permitted"),
                "expected EACCES/EPERM for {node}, got: {stderr}"
            );
        }
        (true, None) => {
            eprintln!("SKIP: no block device node under /dev — sd* probe reading not taken");
        }
        (false, _) => {
            eprintln!("SKIP: non-root env — sd* probe reading not attributable");
        }
    }

    // 0ch① 保底面：安全节点的**删除**仍拒（REMOVE 不在文件级授权集，`/dev`
    // 树无目录规则）。顺序＝对照臂删除 ⇒ **先恢复** ⇒ 守卫臂删除：`sh`
    // spawn 面自身依赖 /dev/null（`Command::output()` 的 stdin＝Stdio::null
    // ＝open("/dev/null")——对照臂删除后立即做守卫臂会以 ENOENT 溺死在
    // spawn，本探针集实测）。恢复臂及其断言收敛进 root 分支（审查处理
    // P3-4：非 root 且 /dev/null 缺席的畸形环境下 mknod 恢复必败、无
    // CAP_MKNOD 同败——无条件断言会假红；非 root 分支仅 best-effort 恢复，
    // 后续探针无 /dev/null 依赖）；对照臂同败 ⇒ 跳过断言（不假红）。
    let control =
        spawn_without_guard("rm /dev/null && echo removed").expect("spawn rm control probe");
    if control.status.success() {
        eprintln!("control arm: root deleted /dev/null (env holds root)");
        let restore = spawn_without_guard(
            "[ -e /dev/null ] || { mknod /dev/null c 1 3 && chmod 666 /dev/null; }",
        )
        .expect("spawn /dev/null restore");
        assert!(
            restore.status.success(),
            "/dev/null must be restored before the guarded rm probe: {restore:?}"
        );
        let denied = spawn_with_guard("rm /dev/null").expect("spawn rm probe");
        assert!(
            !denied.status.success(),
            "rm /dev/null (REMOVE family) must be denied under the guard: {denied:?}"
        );
        // 守卫臂被拒后 /dev/null 必在；若守卫失效被删，此处兜底恢复并让
        // 断言失败已先行报红。
        let _ = spawn_without_guard(
            "[ -e /dev/null ] || { mknod /dev/null c 1 3 && chmod 666 /dev/null; }",
        )
        .expect("spawn /dev/null post-probe guard restore");
    } else {
        eprintln!("SKIP: rm control probe failed without guard — no root perms; reading not taken");
        // 非 root 下 best-effort 恢复（/dev/null 在场时为 no-op）；失败不
        // 构成假红——本探针的读数已按对照臂同败跳过。
        let _ = spawn_without_guard(
            "[ -e /dev/null ] || { mknod /dev/null c 1 3 && chmod 666 /dev/null; }",
        )
        .expect("spawn /dev/null best-effort restore");
    }

    // /proc 实质不变（§7.3）：procfs 本身不支持常规创建，守卫内外同败——
    // 对照臂同败 ⇒ 断言环境成立下的「守卫后仍拒」（不区分失败来源）。
    let control = spawn_without_guard("mkdir /proc/orz0ch-probe").expect("spawn /proc control");
    if !control.status.success() {
        let denied = spawn_with_guard("mkdir /proc/orz0ch-probe").expect("spawn /proc probe");
        assert!(
            !denied.status.success(),
            "/proc creation must stay unavailable under the guard: {denied:?}"
        );
        let _ = std::fs::remove_dir("/proc/orz0ch-probe");
    } else {
        eprintln!("SKIP: /proc control probe unexpectedly succeeded; reading not taken");
    }
}

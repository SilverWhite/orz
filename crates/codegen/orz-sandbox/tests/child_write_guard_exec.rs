//! Landlock 装挂 exec 后真机读数（0bw①，2026-09-27 重建批；**0cc v3 收窄
//! 后形态**，2026-09-29）。
//!
//! `install_rejects_on_empty_ruleset_paths_but_probe_gates`（lib 单测）因
//! 不能在测试进程内 self-restrict 而只验探测门；本集成测试把装挂放进
//! **子进程**（pre_exec 同生产接线），对内核取真实读数：写灾难防护最小核
//! （`/dev`——规则 2 内核面）被拒、写 `/tmp` 照常、写载体面系统树
//! （`/usr/local`——S4 成败项）放行——deny 表补集语义的 exec 后实证。
//! 容器/CI 内核 <5.13 或 seccomp 封 landlock syscall 时探测为 `None` ⇒
//! 显式跳过（读数如实，不假绿）。

#![cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

use std::os::unix::process::CommandExt;
use std::process::Command;

use orz_sandbox::child_write_guard::{install_best_effort, kernel_abi_version, prepare_allow_dirs};

/// 灾难防护最小核同值副本（机制 crate 不反向依赖策略 crate；单一源
/// `orz-tools::write_control::LINUX_DISASTER_KERNEL_FACES`，变更须两处同步；
/// 0cc v3——v1 表 B 十二树中的 /etc /usr /lib* /bin /sbin 载体面已放行）。
const CORE: [&str; 4] = ["/boot", "/dev", "/proc", "/sys"];

fn spawn_with_guard(script: &str) -> Result<std::process::Output, std::io::Error> {
    let Some(dirs) = prepare_allow_dirs(&CORE) else {
        panic!("prepare_allow_dirs returned None despite ABI probe success");
    };
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script);
    // SAFETY: pre_exec 注册钩子；闭包内 install_best_effort 亦为 unsafe
    // （fork 后 exec 前，与生产 spawn 点同型）。
    unsafe {
        cmd.pre_exec(move || {
            unsafe { install_best_effort(&dirs) };
            Ok(())
        });
    }
    cmd.output()
}

/// 无守卫对照臂（真实权限基线——非 root 环境下区分 Landlock EPERM 与
/// 真 EACCES；对照臂同败 ⇒ 环境不成立，显式跳过读数）。
fn spawn_without_guard(script: &str) -> Result<std::process::Output, std::io::Error> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script);
    cmd.output()
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

    // 最小核：/dev 写被内核拒（Permission denied），文件不得落盘。对照臂
    // 同败（非 root）⇒ 环境 不成立，跳过读数（不假红）。
    let control = spawn_without_guard("echo blocked > /dev/orz-landlock-deny-probe")
        .expect("spawn /dev control probe");
    let _ = std::fs::remove_file("/dev/orz-landlock-deny-probe");
    if !control.status.success() {
        eprintln!(
            "SKIP: /dev control probe failed without guard — no root perms; reading not taken"
        );
        return;
    }
    let denied =
        spawn_with_guard("echo blocked > /dev/orz-landlock-deny-probe").expect("spawn /dev probe");
    assert!(
        !denied.status.success(),
        "/dev write must be denied under the guard: {denied:?}"
    );
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(
        stderr.contains("Permission denied"),
        "expected Permission denied, got: {stderr}"
    );
    assert!(
        !std::path::Path::new("/dev/orz-landlock-deny-probe").exists(),
        "no file may land in /dev under the guard"
    );
}

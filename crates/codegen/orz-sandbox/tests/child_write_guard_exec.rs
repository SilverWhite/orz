//! Landlock 装挂 exec 后真机读数（0bw①，2026-09-27 重建批）。
//!
//! `install_rejects_on_empty_ruleset_paths_but_probe_gates`（lib 单测）因
//! 不能在测试进程内 self-restrict 而只验探测门；本集成测试把装挂放进
//! **子进程**（pre_exec 同生产接线），对内核取真实读数：
//! 写 `/etc`（表 B）被拒、写 `/tmp`（非表 B）照常——deny 表补集语义的
//! exec 后实证。容器/CI 内核 <5.13 或 seccomp 封 landlock syscall 时
//! 探测为 `None` ⇒ 显式跳过（读数如实，不假绿）。

#![cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")))]

use std::os::unix::process::CommandExt;
use std::process::Command;

use orz_sandbox::child_write_guard::{
    install_best_effort, kernel_abi_version, prepare_allow_dirs,
};

/// 表 B 同值副本（机制 crate 不反向依赖策略 crate；单一源
/// `orz-tools::write_control::LINUX_SYSTEM_CORE`，变更须两处同步）。
const CORE: [&str; 12] = [
    "/boot", "/etc", "/usr", "/lib", "/lib32", "/lib64", "/libx32", "/bin", "/sbin", "/dev",
    "/proc", "/sys",
];

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

#[test]
fn exec_guard_denies_core_write_and_allows_tmp() {
    let Some(version) = kernel_abi_version() else {
        eprintln!("SKIP: kernel Landlock unavailable (ABI probe None) — reading not taken");
        return;
    };
    eprintln!("kernel landlock ABI = {version}");

    // 非表 B 基线：/tmp 写照常成功（allowlist 补集语义的正面）。
    let ok = spawn_with_guard("echo ok > /tmp/orz-landlock-allow-probe && cat /tmp/orz-landlock-allow-probe")
        .expect("spawn /tmp probe");
    assert!(ok.status.success(), "/tmp write must succeed: {ok:?}");
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "ok");
    let _ = std::fs::remove_file("/tmp/orz-landlock-allow-probe");

    // 表 B：/etc 写被内核拒（Permission denied），文件不得落盘。
    let denied = spawn_with_guard("echo blocked > /etc/orz-landlock-deny-probe")
        .expect("spawn /etc probe");
    assert!(
        !denied.status.success(),
        "/etc write must be denied under the guard: {denied:?}"
    );
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(
        stderr.contains("Permission denied"),
        "expected Permission denied, got: {stderr}"
    );
    assert!(
        !std::path::Path::new("/etc/orz-landlock-deny-probe").exists(),
        "no file may land in /etc under the guard"
    );
}

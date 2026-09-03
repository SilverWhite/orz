//! TER T1.12 (W-F11, 2026-09-04)：机械层代码工具环境快照（PULL 白名单
//! 面，供黑板 `section=env`）。范围 = 工具/语言/包/版本的存在性与版本、
//! 关键输入在场判定；单项版本探测 ≤1s 超时、整快照 ≤5s；连通性判定由
//! W-F12（M2 T2.2）快速确定性失败闭环后接入（本模块暂不产生
//! connectivity 行——fail-closed 不伪造）。
//! 纪律：绝不输出任务专属结论 / allowlist 内容 / “缺工具链”压测提示。

use orz_loop::host::EnvSnapshotFact;
use std::path::Path;
use std::time::Duration;

/// 探测注册表（key, kind）。kind 落在 orz-loop `env` 渲染白名单内。
const PROBES: &[(&str, &str)] = &[
    ("python3", "language"),
    ("python", "language"),
    ("node", "language"),
    ("cargo", "tool"),
    ("rustc", "language"),
    ("git", "tool"),
    ("go", "language"),
    ("gcc", "language"),
    ("clang", "language"),
    ("make", "tool"),
    ("cmake", "tool"),
    ("java", "language"),
    ("npm", "package"),
    ("pip3", "package"),
    ("pip", "package"),
    ("pwsh", "tool"),
];

const VERSION_TIMEOUT: Duration = Duration::from_millis(1_000);
const VALUE_CAP: usize = 80;

fn path_search_dirs() -> Vec<std::path::PathBuf> {
    let raw = std::env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ';' } else { ':' };
    raw.split(sep)
        .filter(|s| !s.is_empty())
        .map(Into::into)
        .collect()
}

fn executable_candidates(name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    if cfg!(windows) {
        let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".EXE;.CMD;.BAT".to_string());
        for ext in exts.split(';').filter(|e| !e.is_empty()) {
            out.push(format!("{name}{}", ext.to_lowercase()));
        }
    }
    out
}

fn find_in_path(name: &str) -> Option<std::path::PathBuf> {
    for dir in path_search_dirs() {
        for candidate in executable_candidates(name) {
            let p = dir.join(&candidate);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn trim_version(s: &str) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    let capped: String = line.chars().take(VALUE_CAP).collect();
    if capped != line {
        format!("{capped}…")
    } else {
        capped.to_string()
    }
}

async fn probe_version(bin: &Path) -> Option<String> {
    let Ok(child) = tokio::process::Command::new(bin)
        .arg("--version")
        .output()
        .await
    else {
        return None;
    };
    if !child.status.success() {
        return None;
    }
    // 版本行通常为 ASCII；此处走 lossy 快速路径即可（完整输出检索才需要
    // 固定解码链）。
    let trimmed = trim_version(&String::from_utf8_lossy(&child.stdout));
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn input_present(cwd: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(cwd) else {
        return false;
    };
    entries.flatten().any(|e| {
        let name = e.file_name().to_string_lossy().into_owned();
        !name.starts_with('.') && name != ".git" && name != ".gsa"
    })
}

/// 生成 env 快照事实（≤5s）。
pub(crate) async fn snapshot_env(cwd: &Path) -> Vec<EnvSnapshotFact> {
    // 存在性探测（PATH 扫描）顺序且极快；版本探测并发执行——墙钟 ≈
    // 单个 `--version` 超时上限（1s），整快照稳定 ≤5s（实测 ≈1s）。
    let found: Vec<(&str, &str, std::path::PathBuf)> = PROBES
        .iter()
        .filter_map(|(name, kind)| find_in_path(name).map(|bin| (*name, *kind, bin)))
        .collect();
    let mut set = tokio::task::JoinSet::new();
    for (name, kind, bin) in found {
        set.spawn(async move {
            let value = match tokio::time::timeout(VERSION_TIMEOUT, probe_version(&bin)).await {
                Ok(Some(version)) => format!("present ({version})"),
                Ok(None) => "present (no version line)".to_string(),
                Err(_) => "present (version timeout)".to_string(),
            };
            (name, kind, value)
        });
    }
    let mut facts = Vec::with_capacity(PROBES.len() + 1);
    while let Some(res) = set.join_next().await {
        let (name, kind, value) = res.expect("version probe task");
        facts.push(EnvSnapshotFact {
            kind: kind.to_string(),
            key: name.to_string(),
            value,
        });
    }
    for (name, kind) in PROBES {
        if !facts.iter().any(|f| f.key == *name) {
            facts.push(EnvSnapshotFact {
                kind: (*kind).to_string(),
                key: (*name).to_string(),
                value: "missing".to_string(),
            });
        }
    }
    facts.push(EnvSnapshotFact {
        kind: "input".to_string(),
        key: "workspace_input_present".to_string(),
        value: if input_present(cwd) { "true" } else { "false" }.to_string(),
    });
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_value_is_capped_to_one_line() {
        let text = format!("v1.2.3\n{}", "x".repeat(300));
        assert_eq!(trim_version(&text), "v1.2.3");
        let long = "x".repeat(200);
        let capped = trim_version(&long);
        assert!(capped.chars().count() <= VALUE_CAP + 1);
    }

    #[tokio::test]
    async fn snapshot_is_bounded_and_structured() {
        let dir = std::env::temp_dir().join(format!("orz-host-env-snap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("input.txt"), b"x").unwrap();
        let started = std::time::Instant::now();
        let facts = snapshot_env(&dir).await;
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "snapshot must finish within 5s"
        );
        let kinds: std::collections::HashSet<&str> =
            facts.iter().map(|f| f.kind.as_str()).collect();
        assert!(
            kinds
                .iter()
                .all(|k| orz_loop::env::ENV_ALLOWED_KINDS.contains(k)),
            "{kinds:?}"
        );
        assert!(
            facts
                .iter()
                .any(|f| f.key == "workspace_input_present" && f.value == "true")
        );
        let mut keys: Vec<_> = facts.iter().map(|f| &f.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), facts.len(), "keys must be unique");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

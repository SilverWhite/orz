//! 回退窗口机械 undo 面（0bw④，2026-09-27；`WRITE_CONTROL_MECHANICAL_
//! DESIGN_2026-09-26.md` §6「git 检查点/journal undo」的可实施半边）。
//!
//! 0bm⑦ 编辑前快照（`.gsa/rollback/<key8>/<millis>-<call8>.bak`＋meta 侧车）
//! 的消费端此前只有「模型 read_file 读回再写回」的提示语文案；本模块提供
//! **机械 undo**：`list`（枚举快照＋目标路径）与 `restore`（快照字节原样
//! 写回目标文件，覆写前把目标当前内容再存一格回退快照——undo 自身可逆）。
//!
//! **边界（如实）**：
//! - 本面**不涉 git**——「git 自动检查点」的容器/触发/回收（避免污染用户
//!   仓库历史）在设计中显式留待用户裁决，维持设计登记不落码。
//! - 快照窗口每文件保留 [`orz_tools::util::write_face::ROLLBACK_RETENTION`]
//!   条（0bm⑦ 语义不变）；被挤出窗口的快照不可恢复。
//! - restore 的**目标域收敛**（2026-09-27 复审 P0 裁决，设计 §3.4 登记）：
//!   目标必须归一化后仍在本工作区（cwd）之内——拒绝绝对路径／盘符形态／
//!   `..` 越界；`.gsa` 域（C1）在归一后判定（`./.gsa/…`、`x/../.gsa/…`、
//!   `.GSA` 大小写变体均不可绕）；并复用 L1 单一源
//!   [`orz_tools::types::write_control::check_write_target`]——回退 CLI 是
//!   deny 表的**第四消费点**（工具面／命令面／进程面之外，模型可经终端
//!   调用本 CLI，故同表约束）。
//! - 写回**原子化**（2026-09-27 复审 P2）：先写目标同目录临时文件再
//!   rename——中断不留截断目标。

use std::path::{Path, PathBuf};

/// 回退快照清单行（`orz rollback list`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackListRow {
    /// 相对 cwd 的快照指针（restore 的定位键）。
    pub pointer: String,
    /// 目标文件相对路径（meta 侧车；旧快照无 meta ⇒ `None`）。
    pub target: Option<String>,
    /// 快照字节数。
    pub size_bytes: u64,
    /// 快照时间戳（文件名毫秒位）。
    pub millis: u128,
}

/// 枚举 `{cwd}/.gsa/rollback/` 全部快照（按 key 目录序＋文件名时间序）。
pub fn list_rollback(cwd: &Path) -> Result<Vec<RollbackListRow>, String> {
    let root = cwd.join(".gsa").join("rollback");
    let mut rows = Vec::new();
    let mut keys: Vec<PathBuf> = match std::fs::read_dir(&root) {
        Ok(entries) => entries.flatten().map(|e| e.path()).collect(),
        Err(err) => return Err(format!("读取 {}: {err}", root.display())),
    };
    keys.sort();
    for key_dir in keys {
        if !key_dir.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&key_dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| n.ends_with(".bak") && !n.ends_with(".bak.meta"))
            .collect();
        names.sort();
        for name in names {
            let path = key_dir.join(&name);
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let millis = name
                .split('-')
                .next()
                .and_then(|m| m.parse::<u128>().ok())
                .unwrap_or(0);
            let key = key_dir
                .file_name()
                .map(|k| k.to_string_lossy().to_string())
                .unwrap_or_default();
            let target = std::fs::read_to_string(key_dir.join(format!("{name}.meta")))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            rows.push(RollbackListRow {
                pointer: format!(".gsa/rollback/{key}/{name}"),
                target,
                size_bytes,
                millis,
            });
        }
    }
    Ok(rows)
}

/// 快照字节原样写回目标文件。
///
/// `target` 缺省取快照 meta 侧车记录；显式传参覆盖（用于 meta 缺席的
/// 旧快照）。目标域＝**cwd 内归一化相对路径**（模块注释「目标域收敛」）；
/// 覆写前目标当前内容再存一格回退快照（undo 自身可逆，同一 0bm⑦ 窗口）；
/// 写回经临时文件＋rename 原子化。
pub fn restore_rollback(cwd: &Path, pointer: &str, target: Option<&str>) -> Result<String, String> {
    // 指针形态校验：只接受本面产出的相对形态（防穿越/绝对路径）。
    if !pointer.starts_with(".gsa/rollback/")
        || pointer.contains("..")
        || pointer.contains('\\')
        || pointer.ends_with(".meta")
    {
        return Err(format!(
            "非法回退指针：{pointer}（须为 .gsa/rollback/ 相对形态）"
        ));
    }
    let snapshot = cwd.join(pointer);
    let bytes = std::fs::read(&snapshot).map_err(|e| format!("读取快照失败：{e}"))?;

    // 目标：显式参数 > meta 侧车；随后归一化＋目标域收敛（见下）。
    let requested = match target {
        Some(t) => t.trim().to_string(),
        None => std::fs::read_to_string(snapshot.with_file_name(format!(
            "{}.meta",
            snapshot
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        )))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "快照无 meta 侧车且未显式指定目标文件".to_string())?,
    };
    let resolved = normalize_relative_target(&requested)?;

    // C1（2026-09-27 复审 P0）：`.gsa` 域在归一后判定——`./.gsa/…`、
    // `x/../.gsa/…` 与 `.GSA` 大小写变体（Windows 路径大小写不敏感）均不
    // 可绕；状态链面不做 undo 目标。
    let first = resolved.split('/').next().unwrap_or_default();
    if first.eq_ignore_ascii_case(".gsa") {
        return Err(format!("拒绝以 .gsa 域为回退目标：{resolved}"));
    }
    // deny 单一源第四消费点（复审 P0 裁决；0cb v2 保底化 2026-09-29 收窄；
    // **0cc v3 宿主状态收窄同日**）：回退写面与工具面同表——v3 起写门＝
    // **宿主状态两窄目标**（C1 `.gsa` 由上方独立判定拒；C2′ keystore 根／
    // signer manifest 经装配 env 解析注入），v2 的安装目录／三件套载体面与
    // v1 系统核心臂均退役，cwd 边界不变。
    let host_state = orz_tools::types::write_control::HostStateTargets::from_env();
    let joined = cwd.join(&resolved);
    if let Some(hit) = orz_tools::types::write_control::check_write_target(
        &orz_tools::types::write_control::WriteTargetCtx {
            cwd,
            joined: &joined,
            resolved: None,
            host_state: &host_state,
        },
    ) {
        return Err(format!(
            "回退目标命中写入管控锁死面（{}：{}）：{}",
            hit.rule, hit.root, resolved
        ));
    }

    // 覆写前把目标当前内容存回退快照（undo 自身可逆；新建目标无前像则跳过）。
    let pre_image_note = match std::fs::read(&joined) {
        Ok(current) => {
            match orz_tools::util::write_face::store_rollback_snapshot(
                cwd, &resolved, &current, "rollback",
            ) {
                orz_tools::util::write_face::RollbackOutcome::Stored(pointer) => {
                    format!("；覆写前内容已存 {pointer}")
                }
                orz_tools::util::write_face::RollbackOutcome::Failed(reason) => {
                    format!("；覆写前快照存储失败（如实）：{reason}")
                }
                orz_tools::util::write_face::RollbackOutcome::Skipped => String::new(),
            }
        }
        Err(_) => String::new(),
    };

    // 写回（父目录缺失 ⇒ 显式报错；不静默创建——undo 面不做超范围动作）。
    if let Some(parent) = joined.parent() {
        if !parent.exists() {
            return Err(format!("目标目录不存在：{}", parent.display()));
        }
    }
    // 原子写回（复审 P2）：同目录临时文件 → rename（Windows 语义
    // MOVEFILE_REPLACE_EXISTING）；中断不留截断目标，rename 失败清理临时件。
    let temp_path = joined.with_file_name(format!(
        ".{}.orz-restore-tmp-{}-{}",
        joined
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "target".to_string()),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default(),
    ));
    std::fs::write(&temp_path, &bytes).map_err(|e| format!("写回失败（临时件）：{e}"))?;
    if let Err(e) = std::fs::rename(&temp_path, &joined) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!("写回失败：{e}"));
    }
    Ok(format!(
        "已回退 {resolved} ← {pointer}（{written} 字节）{pre_image_note}",
        written = bytes.len(),
    ))
}

/// 回退目标归一化（cwd 内相对形态；见模块注释「目标域收敛」）。
///
/// 反斜杠归一为 `/` 后按词法走组件：`..` 逐级弹出（弹出空栈＝越出
/// cwd，拒绝）；绝对形态（前导 `/`、Windows 盘符 `C:`/`C:x`）拒绝；
/// `.` 与空段折叠。合法文件名含 `..` 子串（`a..b.txt`）不受影响——
/// 按**组件**而非子串判定。
fn normalize_relative_target(target: &str) -> Result<String, String> {
    let unified = target.trim().replace('\\', "/");
    if unified.is_empty() {
        return Err("回退目标为空".to_string());
    }
    if unified.starts_with('/') {
        return Err(format!(
            "拒绝绝对路径回退目标：{target}（须为 cwd 内相对路径）"
        ));
    }
    let bytes = unified.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Err(format!(
            "拒绝盘符形态回退目标：{target}（须为 cwd 内相对路径）"
        ));
    }
    let mut parts: Vec<&str> = Vec::new();
    for component in unified.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(format!("拒绝越出工作区的回退目标：{target}"));
                }
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return Err(format!("回退目标归一后为空：{target}"));
    }
    Ok(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-rollback-maint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn list_and_restore_roundtrip_with_meta_target() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/a.txt"), b"old content").unwrap();
        // 模拟一次编辑：编辑前快照 old content，随后文件被改写。
        assert!(matches!(
            orz_tools::util::write_face::store_rollback_snapshot(
                &dir,
                "src/a.txt",
                b"old content",
                "call1"
            ),
            orz_tools::util::write_face::RollbackOutcome::Stored(_)
        ));
        std::fs::write(dir.join("src/a.txt"), b"broken edit").unwrap();

        // list：恰一条快照，meta 指向 src/a.txt。
        let rows = list_rollback(&dir).unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].target.as_deref(), Some("src/a.txt"));

        // restore：不带显式目标（meta 默认）。
        let pointer = rows[0].pointer.clone();
        let message = restore_rollback(&dir, &pointer, None).unwrap();
        assert!(message.contains("src/a.txt"), "{message}");
        assert_eq!(
            std::fs::read(dir.join("src/a.txt")).unwrap(),
            b"old content"
        );
        // undo 可逆：覆写前的 broken edit 进入回退窗口。
        let rows_after = list_rollback(&dir).unwrap();
        assert!(
            rows_after
                .iter()
                .any(|r| r.target.as_deref() == Some("src/a.txt") && r.pointer != pointer),
            "{rows_after:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_rejects_escape_and_gsa_targets() {
        let dir = tempdir();
        // 穿越/绝对指针拒绝。
        assert!(restore_rollback(&dir, ".gsa/rollback/../../etc", None).is_err());
        assert!(restore_rollback(&dir, "/abs/path.bak", None).is_err());
        // .gsa 域目标拒绝。
        assert!(restore_rollback(&dir, ".gsa/rollback/x/1.bak", Some(".gsa/x")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-09-27 复审 P0 回归钉：restore 目标域收敛——绝对/盘符/越界/
    /// `.gsa` 绕过形态（`./`、`x/../`、大小写变体）全部拒绝。
    #[test]
    fn restore_rejects_target_escapes_and_gsa_bypasses() {
        let dir = tempdir();
        let key_dir = dir.join(".gsa").join("rollback").join("00000001");
        std::fs::create_dir_all(&key_dir).unwrap();
        std::fs::write(key_dir.join("0000000000001-old.bak"), b"payload").unwrap();
        let pointer = ".gsa/rollback/00000001/0000000000001-old.bak";

        #[cfg(windows)]
        let absolutes = ["C:\\Windows\\x", "C:foo", "\\\\server\\share\\x"];
        #[cfg(not(windows))]
        let absolutes = ["/etc/passwd"];
        for abs in absolutes {
            assert!(
                restore_rollback(&dir, pointer, Some(abs)).is_err(),
                "absolute target must be rejected: {abs}"
            );
        }
        for escape in [
            "../outside.txt",
            "src/../../outside.txt",
            "a/b/../../../outside.txt",
        ] {
            assert!(
                restore_rollback(&dir, pointer, Some(escape)).is_err(),
                "traversal target must be rejected: {escape}"
            );
        }
        for bypass in ["./.gsa/x", "sub/../.gsa/x", ".GSA/x", ".gsa"] {
            assert!(
                restore_rollback(&dir, pointer, Some(bypass)).is_err(),
                ".gsa-domain bypass must be rejected: {bypass}"
            );
        }
        assert!(
            !dir.join("outside.txt").exists(),
            "no escaped file may be created"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `..` 按**组件**而非子串判定：合法文件名 `a..b.txt` 不受影响
    /// （2026-09-27 复审 P3 对齐）。
    #[test]
    fn restore_accepts_dotdot_inside_filename() {
        let dir = tempdir();
        let key_dir = dir.join(".gsa").join("rollback").join("00000001");
        std::fs::create_dir_all(&key_dir).unwrap();
        std::fs::write(key_dir.join("0000000000001-old.bak"), b"legacy").unwrap();
        std::fs::create_dir_all(dir.join("dir")).unwrap();
        let pointer = ".gsa/rollback/00000001/0000000000001-old.bak";
        let message = restore_rollback(&dir, pointer, Some("dir/a..b.txt")).unwrap();
        assert!(message.contains("dir/a..b.txt"), "{message}");
        assert_eq!(std::fs::read(dir.join("dir/a..b.txt")).unwrap(), b"legacy");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 原子写回（2026-09-27 复审 P2）：restore 后目标目录不留
    /// `.orz-restore-tmp-*` 临时件。
    #[test]
    fn restore_leaves_no_temp_files_behind() {
        let dir = tempdir();
        let key_dir = dir.join(".gsa").join("rollback").join("00000001");
        std::fs::create_dir_all(&key_dir).unwrap();
        std::fs::write(key_dir.join("0000000000001-old.bak"), b"content").unwrap();
        let _ = std::fs::create_dir_all(dir.join("src"));
        let pointer = ".gsa/rollback/00000001/0000000000001-old.bak";
        restore_rollback(&dir, pointer, Some("src/a.txt")).unwrap();
        let leftovers: Vec<String> = std::fs::read_dir(dir.join("src"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".orz-restore-tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "temp leftovers: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_without_meta_requires_explicit_target() {
        let dir = tempdir();
        let key_dir = dir.join(".gsa").join("rollback").join("00000001");
        std::fs::create_dir_all(&key_dir).unwrap();
        std::fs::write(key_dir.join("0000000000001-old.bak"), b"legacy").unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        // 无 meta ⇒ 未给目标时显式报错；给了目标则按目标回退。
        let pointer = ".gsa/rollback/00000001/0000000000001-old.bak";
        assert!(restore_rollback(&dir, pointer, None).is_err());
        let message = restore_rollback(&dir, pointer, Some("src/b.txt")).unwrap();
        assert!(message.contains("src/b.txt"), "{message}");
        assert_eq!(std::fs::read(dir.join("src/b.txt")).unwrap(), b"legacy");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

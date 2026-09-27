//! 写路径公共层（0bm ⑥⑦，2026-09-25）。
//!
//! 0bl 审查后，编辑族（`search_replace` 锚点／经典／新建覆盖、`hashline_edit`
//! 既有文件／新建覆盖）各自散写同一组机械动作：UTF-16 fail-closed 门、解码链、
//! BOM 形态、emoji 剥离告知、写入面告知行。0bm ⑥ 把这些动作收敛为**单一
//! 实现点**（本模块），编辑工具只经本层读写；0bm ⑦ 在同一层加**编辑前回退
//! 快照**（硬编辑预存回退窗口，告知行携带回退指针）。
//!
//! 权威边界：本模块只做机械动作与文案构造；编辑语义（行尾保真、锚点核证）
//! 留在各工具。

use std::path::Path;

/// 默认编辑上限：16 MiB（0bm ④）。
pub(crate) const DEFAULT_EDIT_MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// 上限逃生开关（合法正整数覆盖默认值；非法/0 回退默认）。
pub(crate) const EDIT_MAX_FILE_BYTES_ENV: &str = "ORZ_EDIT_MAX_FILE_BYTES";
/// 回退快照每文件保留条数（0bm ⑦ 回退窗口）。
pub(crate) const ROLLBACK_RETENTION: usize = 5;

/// env 锁（测试用；`ORZ_EDIT_MAX_FILE_BYTES` 为进程级）。
#[cfg(test)]
pub(crate) static WRITE_FACE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 编辑入口读入结果（解码文本 ＋ 原文件 BOM 形态）。
pub(crate) struct DecodedForEdit {
    pub(crate) text: String,
    pub(crate) had_bom: bool,
}

/// 编辑面读入单点：UTF-16 fail-closed 门 → 固定解码链 → BOM 形态。
///
/// `Err(message)` = 面向模型的拒绝文案（UTF-16 形态，编辑面不支持）。
pub(crate) fn decode_for_edit(bytes: &[u8], file_path: &str) -> Result<DecodedForEdit, String> {
    if crate::util::encoding::utf16_shaped_input(bytes) {
        return Err(crate::util::encoding::utf16_rejection_message(file_path));
    }
    let (text, label) = crate::util::encoding::decode_text(bytes);
    Ok(DecodedForEdit {
        text,
        had_bom: crate::util::encoding::label_had_bom(&label),
    })
}

/// 当前编辑上限：env 覆盖（合法正整数）→ 默认 16 MiB。
pub(crate) fn edit_max_file_bytes() -> u64 {
    match std::env::var(EDIT_MAX_FILE_BYTES_ENV) {
        Ok(raw) => raw
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_EDIT_MAX_FILE_BYTES),
        Err(_) => DEFAULT_EDIT_MAX_FILE_BYTES,
    }
}

/// 读入后的大小门：`Some(message)` = 超限拒绝文案（含实际大小、上限、逃生名）。
pub(crate) fn check_edit_size(file_path: &str, actual_bytes: u64) -> Option<String> {
    let limit = edit_max_file_bytes();
    (actual_bytes > limit).then(|| {
        format!(
            "Error: {file_path} is {actual_bytes} bytes, exceeding the edit-face size limit of \
             {limit} bytes ({} MiB). Raise the limit with {EDIT_MAX_FILE_BYTES_ENV} if this edit \
             is intended, or edit a smaller file.",
            limit / (1024 * 1024)
        )
    })
}

/// 回退快照存储结局（失败**不静默**：退化为告知行文案，如实随成功输出携带）。
pub enum RollbackOutcome {
    /// 已存储；携带相对 cwd 的回退指针（`.gsa/rollback/…`）。
    Stored(String),
    /// 存储失败；携带原因摘要。
    Failed(String),
    /// 无前内容（新建路径），不产出告知行。
    Skipped,
}

/// 编辑前回退快照（0bm ⑦）：`.gsa/rollback/<path-key8>/<millis>-<call8>.bak`。
///
/// - **原始字节**（BOM/行尾/编码形态保真——回退即原样写回）；
/// - 每目录保留最近 [`ROLLBACK_RETENTION`] 条（按文件名序＝时间序，超出清旧）；
/// - 存储失败不静默：返回 [`RollbackOutcome::Failed`]，调用方随告知行如实携带。
pub fn store_rollback_snapshot(
    cwd: &Path,
    target_display: &str,
    previous: &[u8],
    call_id: &str,
) -> RollbackOutcome {
    let key = format!(
        "{:08x}",
        crate::util::hash::fnv1a_32(target_display.as_bytes())
    );
    let dir = cwd.join(".gsa").join("rollback").join(&key);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let call8: String = call_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    let call8 = if call8.is_empty() {
        "nocall".to_string()
    } else {
        call8
    };
    if let Err(err) = std::fs::create_dir_all(&dir) {
        return RollbackOutcome::Failed(format!("create {}: {err}", dir.display()));
    }
    // 毫秒同刻（连发调用）时递增毫秒位避让：既保证单文件不互相覆盖，又保证
    // 文件名序＝时间序（后缀式避让会破坏同毫秒内的排序，由保留窗口测试钉住）。
    let mut stamp = millis;
    let mut name = format!("{stamp:013}-{call8}.bak");
    let mut attempt = 0u32;
    while dir.join(&name).exists() && attempt < 10_000 {
        stamp += 1;
        name = format!("{stamp:013}-{call8}.bak");
        attempt += 1;
    }
    let file = dir.join(&name);
    if let Err(err) = std::fs::write(&file, previous) {
        return RollbackOutcome::Failed(format!("write {}: {err}", file.display()));
    }
    // 0bw④（2026-09-27）：meta 侧车记录目标路径——`orz rollback list`
    // 据此展示、`restore` 据此默认目标。缺省回退＝旧快照形态（无 meta，
    // restore 需显式给目标）。meta 写失败不回滚快照本体（留痕面缺失
    // 如实，不影响 undo 字节源）。
    let _ = std::fs::write(dir.join(format!("{name}.meta")), target_display);
    prune_rollback_dir(&dir);
    RollbackOutcome::Stored(format!(".gsa/rollback/{key}/{name}"))
}

/// 保留窗口修剪：按文件名（时间序）保留最近 [`ROLLBACK_RETENTION`] 条 `.bak`。
/// 修剪失败静默——快照本体已落盘，修剪只影响存储占用，不产出误导性回退指针。
fn prune_rollback_dir(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter(|name| name.ends_with(".bak"))
        .collect();
    names.sort();
    if names.len() <= ROLLBACK_RETENTION {
        return;
    }
    for name in &names[..names.len() - ROLLBACK_RETENTION] {
        let _ = std::fs::remove_file(dir.join(name));
        // 0bw④：meta 侧车随快照一同修剪。
        let _ = std::fs::remove_file(dir.join(format!("{name}.meta")));
    }
}

/// 回退告知行：存储成功＝带指针；失败＝如实携带原因（不静默）；无前内容＝无行。
pub(crate) fn rollback_notice_line(outcome: &RollbackOutcome, file_path: &str) -> Option<String> {
    match outcome {
        RollbackOutcome::Stored(pointer) => Some(format!(
            "[回退窗口] 编辑前内容已存 {pointer}（回退＝read_file 读取该文件，\
             将内容原样写回 {file_path}）"
        )),
        RollbackOutcome::Failed(reason) => Some(format!(
            "[回退窗口] 编辑前内容快照存储失败：{reason}（本次编辑未受影响；\
             如需回退请依赖外部版本控制）"
        )),
        RollbackOutcome::Skipped => None,
    }
}

/// 告知行尾附单点（0bm ⑥ 收敛）：成功编辑的两个 prompt 字段各追加一行；
/// 失败／非 `EditsApplied` 面不改动。
pub(crate) fn attach_notice_line(
    result: crate::types::output::SearchReplaceOutput,
    line: &str,
) -> crate::types::output::SearchReplaceOutput {
    match result {
        crate::types::output::SearchReplaceOutput::EditsApplied(mut applied) => {
            // 0bm 复审补口（2026-09-24）口径：仅当输出不以换行收尾时补分隔
            // 换行，避免在已有尾换行的输出后产生一个空行（纯外观）。
            if !applied.tool_output_for_prompt.ends_with('\n') {
                applied.tool_output_for_prompt.push('\n');
            }
            applied.tool_output_for_prompt.push_str(line);
            if let Some(concise) = applied.tool_output_for_prompt_concise.as_mut() {
                if !concise.ends_with('\n') {
                    concise.push('\n');
                }
                concise.push_str(line);
            }
            crate::types::output::SearchReplaceOutput::EditsApplied(applied)
        }
        other => other,
    }
}

/// 回退告知尾附：`Stored`/`Failed` 附行（失败不静默），`Skipped` 原样返回。
pub(crate) fn attach_rollback_notice(
    result: crate::types::output::SearchReplaceOutput,
    outcome: &RollbackOutcome,
    file_path: &str,
) -> crate::types::output::SearchReplaceOutput {
    match rollback_notice_line(outcome, file_path) {
        Some(line) => attach_notice_line(result, &line),
        None => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_gate_default_and_oversized_message() {
        let _guard = WRITE_FACE_ENV_LOCK.lock().unwrap();
        // 默认档：16 MiB——17 MiB 的虚拟大小必须被拒，文案须含逃生名与上限。
        let message =
            check_edit_size("big.log", DEFAULT_EDIT_MAX_FILE_BYTES + 1).expect("oversized");
        assert!(message.contains(EDIT_MAX_FILE_BYTES_ENV), "{message}");
        assert!(message.contains("16 MiB"), "{message}");
        assert!(check_edit_size("big.log", DEFAULT_EDIT_MAX_FILE_BYTES).is_none());
    }

    #[test]
    fn size_gate_env_override() {
        let _guard = WRITE_FACE_ENV_LOCK.lock().unwrap();
        // SAFETY: 本测试与其它 env 读写经 WRITE_FACE_ENV_LOCK / EMOJI_ENV_LOCK
        // 串行化（测试进程内无并发 env 访问；Rust 2024 起 set_var 为 unsafe）。
        unsafe {
            std::env::set_var(EDIT_MAX_FILE_BYTES_ENV, "1048576");
        }
        assert_eq!(edit_max_file_bytes(), 1_048_576);
        assert!(check_edit_size("f.txt", 2_097_152).is_some());
        assert!(check_edit_size("f.txt", 1_048_576).is_none());
        // 非法值回退默认。
        unsafe {
            std::env::set_var(EDIT_MAX_FILE_BYTES_ENV, "0");
        }
        assert_eq!(edit_max_file_bytes(), DEFAULT_EDIT_MAX_FILE_BYTES);
        unsafe {
            std::env::set_var(EDIT_MAX_FILE_BYTES_ENV, "not-a-number");
        }
        assert_eq!(edit_max_file_bytes(), DEFAULT_EDIT_MAX_FILE_BYTES);
        unsafe {
            std::env::remove_var(EDIT_MAX_FILE_BYTES_ENV);
        }
    }

    #[test]
    fn rollback_snapshot_keeps_original_bytes_and_pointer() {
        let tmp = tempfile::tempdir().unwrap();
        let original: &[u8] = b"\xef\xbb\xbfold\r\ncontent\n";
        let outcome = store_rollback_snapshot(tmp.path(), "dir/file.txt", original, "call1234");
        let RollbackOutcome::Stored(pointer) = outcome else {
            panic!("snapshot must be stored");
        };
        assert!(pointer.starts_with(".gsa/rollback/"), "{pointer}");
        let absolute = tmp.path().join(&pointer);
        assert_eq!(std::fs::read(&absolute).unwrap(), original);
        // 告知行携带指针与回退用法。
        let line = rollback_notice_line(&RollbackOutcome::Stored(pointer.clone()), "dir/file.txt")
            .expect("stored ⇒ line");
        assert!(line.contains(&pointer), "{line}");
        assert!(line.contains("dir/file.txt"), "{line}");
    }

    #[test]
    fn rollback_retention_keeps_last_five() {
        let tmp = tempfile::tempdir().unwrap();
        for index in 0..7u8 {
            let outcome = store_rollback_snapshot(
                tmp.path(),
                "keep.txt",
                format!("v{index}").as_bytes(),
                "callkeep",
            );
            assert!(matches!(outcome, RollbackOutcome::Stored(_)));
        }
        let key = format!("{:08x}", crate::util::hash::fnv1a_32("keep.txt".as_bytes()));
        let dir = tmp.path().join(".gsa").join("rollback").join(&key);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
            // 0bw④：meta 侧车随行但不计快照窗口数。
            .filter(|name| name.ends_with(".bak") && !name.ends_with(".bak.meta"))
            .collect();
        names.sort();
        assert_eq!(names.len(), ROLLBACK_RETENTION, "{names:?}");
        // 每条快照恰有一条 meta 侧车，且记录目标路径。
        for name in &names {
            let meta = std::fs::read_to_string(dir.join(format!("{name}.meta"))).unwrap();
            assert_eq!(meta, "keep.txt");
        }
        // 保留的是最近 5 条：v2..v6（v0/v1 被清）。
        let contents: Vec<Vec<u8>> = names
            .iter()
            .map(|name| std::fs::read(dir.join(name)).unwrap())
            .collect();
        assert_eq!(contents[0], b"v2");
        assert_eq!(contents[ROLLBACK_RETENTION - 1], b"v6");
    }

    #[test]
    fn rollback_failure_is_not_silent() {
        // 用一个不可能创建目录的路径（父为文件）触发失败，断言 Failed 文案。
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let outcome = store_rollback_snapshot(&blocker, "file.txt", b"old", "c");
        let RollbackOutcome::Failed(reason) = outcome else {
            panic!("store under a file must fail");
        };
        let line = rollback_notice_line(&RollbackOutcome::Failed(reason), "file.txt")
            .expect("failed ⇒ line");
        assert!(line.contains("失败"), "{line}");
    }
}

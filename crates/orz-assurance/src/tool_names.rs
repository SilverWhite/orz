//! 工具名字面单一源（0ao 重名面全库收敛，2026-09-18；索引 `GAP-TOOLNAME-LITERAL-CONVERGENCE`）。
//!
//! 为什么在 orz-assurance：依赖方向 orz-loop → orz-assurance ← orz-host，
//! orz-assurance 是三者中唯一可被双方引用而**不倒挂依赖**的公共层
//! （0ao 立项批明令「禁引依赖方向倒挂」）。工具名字面的语义权威本就是
//! 契约侧——journal-conformance 判官的 `journal::toolsets::WORK_TOOLS`
//! 表就在本 crate；名字常量与其同源落位。
//!
//! 纪律（0ao 判据）：
//! - 生产判等面（注册表 / 分类 / 探针 / 权限桥 / 派发过滤）一律引用本表，
//!   禁止再写字面；
//! - 豁免面仅限：注释、`#[cfg(test)]` 测试模块、以及 [`self::tests`]
//!   钉子里 `LITERAL_EXEMPTS` 逐条具名的文案面；
//! - 机械扫描钉子（本模块测试）巡检全 crates 源树，字面出现在
//!   定义处 / 注释 / 测试 / 豁免清单之外即报红。

/// `blackboard_write`——模型写入面（0ae D0，2026-09-15，用户裁决 DP-6；
/// 8 工具面冻结的用户主导显式例外 +1）。原名定义在
/// `orz-loop/src/blackboard.rs`（2026-09-17 处理批 07405e61 去重），
/// 0ao 上移至本 crate 成全库单源，orz-loop 经再导出别名零字面引用。
pub const BLACKBOARD_WRITE_TOOL_NAME: &str = "blackboard_write";

/// `context_compress`——压缩交互第九工具（0ap，2026-09-18 用户定名；
/// `DESIGN-COMPRESSION-INTERACTION` §1）。知情发起 D3 模型参与压缩窗口
/// ＋响应自带滑块读数表；纯内存压缩状态操作（无文件/网络/黑板外部副作用，
/// ReadOnly 类）。名字自诞生即落本表——不产生第二份字面。
pub const CONTEXT_COMPRESS_TOOL_NAME: &str = "context_compress";

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// 豁免清单：字面允许出现的**非注释、非测试**位置（逐条具名＋理由）。
    /// 条目 = (文件路径〔相对 `crates/`〕，行内必须包含的片段，理由)。
    /// 0ao 落码后**当前为空**——生产判等面全部收敛到常量，生产文案面
    /// （format! 可拼接处）亦经隐式捕获收敛。新增豁免必须在此登记理由，
    /// 钉子即豁免清单本体（0ao 判据口径）；空表＝任何新生产字面直接报红。
    const LITERAL_EXEMPTS: &[(&str, &str, &str)] = &[];

    fn crates_root() -> PathBuf {
        // CARGO_MANIFEST_DIR = <orz>/crates/orz-assurance ⇒ 源树在 ../..。
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates")
    }

    fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// 逐行判定该行是否处于 `#[cfg(test)] mod` 内（花括号深度追踪）。
    /// 边界（如实登记）：只解析标准 `#[cfg(test)] mod … { … }` 形态；
    /// 行尾注释里的不配对花括号可能使深度失步——失步只会把生产行误判为
    /// 测试行从而**漏报**，故豁免清单与定义处断言仍独立兜底。
    fn test_region_lines(text: &str) -> Vec<bool> {
        let mut flags = Vec::new();
        let mut depth: usize = 0;
        let mut test_arms: Vec<usize> = Vec::new();
        let mut next_mod_is_test = false;
        for raw in text.lines() {
            let trimmed = raw.trim_start();
            let is_comment_only = trimmed.starts_with("//");
            let mut is_test = in_test_region(&test_arms, depth);
            if !is_comment_only {
                if next_mod_is_test
                    && (trimmed.starts_with("mod ")
                        || trimmed.starts_with("pub mod ")
                        || trimmed.starts_with("pub(crate) mod "))
                {
                    next_mod_is_test = false;
                    test_arms.push(depth);
                    is_test = true;
                }
                if trimmed.starts_with("#[cfg(test)]") {
                    next_mod_is_test = true;
                }
            }
            flags.push(is_test);
            if !is_comment_only {
                let opens = raw.matches('{').count();
                let closes = raw.matches('}').count();
                depth = (depth + opens).saturating_sub(closes);
            }
            while let Some(&arm) = test_arms.last() {
                if depth <= arm {
                    test_arms.pop();
                } else {
                    break;
                }
            }
        }
        flags
    }

    fn in_test_region(arms: &[usize], depth: usize) -> bool {
        arms.iter().any(|&arm| depth > arm)
    }

    /// 标识符级包含判定：命中处前后不得是 `[A-Za-z0-9_]`——防止
    /// `context_compress` 误命中事件族名 `context_compressed` 等更长标识。
    fn contains_ident(line: &str, name: &str) -> bool {
        let bytes = line.as_bytes();
        let mut from = 0usize;
        while let Some(pos) = line[from..].find(name) {
            let start = from + pos;
            let end = start + name.len();
            let before_ok = start == 0
                || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
            let after_ok =
                end >= bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
            if before_ok && after_ok {
                return true;
            }
            from = start + 1;
        }
        false
    }

    #[test]
    fn tool_name_literals_live_only_at_definition_comments_tests_and_exemptions() {
        let root = crates_root();
        assert!(root.is_dir(), "crates root missing: {}", root.display());
        let mut files = Vec::new();
        collect_rs_files(&root, &mut files);
        assert!(
            files.len() > 100,
            "source tree walk found too few files: {}",
            files.len()
        );

        // (常量标识, 字面值)——定义行判定按标识，字面巡检按值。
        for (ident, name) in [
            ("BLACKBOARD_WRITE_TOOL_NAME", BLACKBOARD_WRITE_TOOL_NAME),
            ("CONTEXT_COMPRESS_TOOL_NAME", CONTEXT_COMPRESS_TOOL_NAME),
        ] {
            let mut definition_sites = 0;
            for file in &files {
                let rel = file
                    .strip_prefix(&root)
                    .expect("under crates root")
                    .to_string_lossy()
                    .replace('\\', "/");
                let Ok(text) = std::fs::read_to_string(file) else {
                    panic!("unreadable source: {}", file.display());
                };
                let is_definition_file = rel == "orz-assurance/src/tool_names.rs";
                let test_flags = test_region_lines(&text);
                for (idx, line) in text.lines().enumerate() {
                    if !contains_ident(line, name) {
                        continue;
                    }
                    if is_definition_file {
                        // 定义文件只认 `pub const <IDENT>` 行为定义处；
                        // 其余（文档注释）按注释面跳过。
                        if line.contains(&format!("pub const {ident}")) {
                            definition_sites += 1;
                        }
                        continue;
                    }
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("//") {
                        continue; // 注释面
                    }
                    if test_flags[idx] {
                        continue; // 测试面
                    }
                    let exempt = LITERAL_EXEMPTS
                        .iter()
                        .any(|(file_frag, needle, _)| *file_frag == rel && line.contains(needle));
                    assert!(
                        exempt,
                        "0ao 机械扫描：`{name}` 字面出现在生产面 {rel}:{} —— \
                         判等面必须引用 tool_names 常量；文案面请在 \
                         tool_names::tests::LITERAL_EXEMPTS 具名登记\n  行：{trimmed}",
                        idx + 1
                    );
                }
            }
            assert_eq!(
                definition_sites, 1,
                "`{name}` 必须恰有一个定义处（orz-assurance/src/tool_names.rs）"
            );
        }
    }

    /// 再导出面零字面：orz-loop 的 `blackboard.rs` 别名必须是 `use` 形态
    /// （否则会出现第二个字面定义处，绕过单源）。
    #[test]
    fn orz_loop_alias_is_a_reexport_not_a_second_literal() {
        let alias = crates_root().join("orz-loop/src/blackboard.rs");
        let text = std::fs::read_to_string(&alias).expect("blackboard.rs readable");
        let def_lines: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("const BLACKBOARD_WRITE_TOOL_NAME") && l.contains('&'))
            .collect();
        assert!(
            def_lines.is_empty(),
            "orz-loop 不得再定义字面常量（应为 use 再导出）：{def_lines:?}"
        );
        assert!(
            text.lines().any(|l| {
                l.contains("use orz_assurance::tool_names::BLACKBOARD_WRITE_TOOL_NAME")
            }),
            "blackboard.rs 必须经 tool_names 再导出单一源"
        );
    }
}

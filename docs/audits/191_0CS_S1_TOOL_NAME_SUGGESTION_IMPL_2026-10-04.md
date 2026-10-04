# 191 批：0cs S1 落码——工具名近似提示 did-you-mean（2026-10-04）

> **用户令**（三段裁决，2026-10-04）：①「不能引入语意推断，只做(a)吧」＝pin 靶点定案为**纯字面**（`run_terminal_patch`→`run_terminal_cmd`，190 批登记的 `→search_replace` 语义期望退役）；②「将当前发生过的错误进行明确记录并硬匹配，再加上使用匹配规则兜底」＝**硬表＋规则兜底**形态采纳；③「我同意这一形态，请开始0cs S1吧」＝S1 放行。
> **性质**＝落码批；落点 `orz/crates/codegen/orz-tools/src/registry/types.rs`（单文件，orz `2d51d22c`，+303/−5）；**契约面零变化**（`ToolErrorKind::NotFound` 不动、`ToolId` 不动、`details` 形状不动、schema 不动、工具面零扩张）——提示仅落错误信封 `message` 文案面。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| S1 勘定（本批前置复查，用户令「请先再查一轮」） | 全语料（`D:/tb-eval`，1,076 个 journal）`Tool not found` 事件按 run_id+event_id 去重＝**193 次／49 个幻影名**；`run_terminal_patch` 本尊仅 2 次（sith RUN-0，与 188 批档吻合；原始 12 次系 6 份 checkpoint 快照重复）；主导族＝**在役真名 `run_terminal_cmd` 的拼写腐蚀**（40 名 174 次＝90.2%，真名经源码核证：`implementations/grok_build/bash/mod.rs` ToolId＋`codex_app.rs:906`＋`permission.rs:80`）；跨 harness 命令名族 16 次（`run_command` 9 为最大）；真离群仅 3 次（`SEMANTIC_SUMMARY` 2／`cursor_agent` 1） |
| 形态（用户裁决定案） | 两级查询单一优先级：**精确命中注册表恒先行**（正常派发路径，不受本批影响）→ **闭集硬表**（5 条，仅收规则够不到的跨 harness 命令名 14/16 次；`invoke`/`run_task` 目标不唯一、`cursor_agent`/`SEMANTIC_SUMMARY` 无近似物，均不收＝无近似不附）→ **规则臂**（大小写不敏感；Levenshtein≤2 **或**共前缀≥8；排序＝共前缀↓/距离↑/名↑；至多 2 条）。**零语义推断**（用户令）——硬表是已观测错误的显式记录，非运行时语义判断 |
| 落码 | `NOT_FOUND_SUGGESTION_TABLE`（const 闭集 5 条）＋`levenshtein_capped`/`common_prefix_len`/`not_found_suggestions` 纯函数＋`tool_not_found_error(tool_name, registered)` 漏斗改造；**表命中须目标仍在册**（注册表演化后提示永不指出闭集；缺册退化＝无近似不附）；三调用点同源接线（`try_parse`/`call_raw`/`prepare_dispatch`，均持 `tools` 读守卫） |
| 勘误（随批） | **190 批登记的装配点引证有误**：`use_tool/mod.rs:433` 系 `#[cfg(test)]` 内 mock dispatch（非生产装配点）；生产漏斗唯一＝`registry/types.rs` `tool_not_found_error`（行号随本批漂移，funnel 身份不变）。另 `registry/types.rs:1536/1744`（"Tool not found in LocalRegistry"）为已解析名的内层派发失败、`xai-computer-hub-*` 三处为远程会话解析——均不在 0cs 范围（观测幻影全走主注册表漏斗） |
| 钉子 | **＋11 全过**（§3）：表长钉／表∩`MANAGED_TOOLS`=∅ 与指向在册钉／表命中臂／表目标缺册退化臂／共前缀臂（pin (a)）／Lev 臂／大小写臂／三离群无近似臂／至多 2 帽／信封文案钉／finalize→try_parse 端到端装配钉 |
| 验证 | orz-tools **3009/0**（2998＋11 恰新增、既有零破坏）；clippy **26=26 零新增**（stash 对拍冻结树 `a7526cc2`；本批文件 3 条新增告警当批修复＝`manual_ignore_case_cmp`×2＋`collapsible_if`）；fmt 净；源清单随 orz `2d51d22c` 再生成（差异恰 1 行＝types.rs） |
| 边界 | 不重建不推送；S2 载体随下一重建批（0.8.14，先于 0cr S2——190 批排期位不变）；S3 真机随 0cr 首题顺带（判据基线＝0S FP-5「次轮自恢复 ~1 轮/次」）；0cr 冻结面「载体 0.8.13」随 S1 勘定批更新为重建后新版本号 |

## §1 幻影全语料清单（勘定证据表，49 名 193 次）

> 扫描口径：`D:/tb-eval` 全部 `events.jsonl`（1,076 个）中 `Tool not found: <name>` 载荷，按 `(run_id, event_id)` 去重（checkpoint 快照多副本不重复计）。

**族 A：`run_terminal_cmd` 拼写腐蚀（40 名 174 次＝90.2%；规则共前缀臂 100% 覆盖，不入表）**

| 次数 | 名 |
|---|---|
| 28/21/18/15/13/12/10 | `run_terminal_calls`/`_cell`/`_catch`/`_cpack`/`_cblock`/`_ccmd`/`_cpt` |
| 8/5/4 | `run_terminal_cord`/`_card`/`_cdot` |
| 3/3/3 | `run_terminal_cpt_cmd`/`_cdir`/`_catalog` |
| 2×6 | `run_terminal_patch`〔sith RUN-0×2＝188 批档原证〕/`_cpath`/`_cpacket`/`_cmand`/`_call` |
| 1×22 | `run_terminal_cscript`/`_craft`/`_crack`/`_cptmd`/`_cptest`/`_cplet`/`_cpackge`/`_cpackag`/`_cpack_cmd`/`_cp`/`_cool`/`_cold`/`_cock`/`_clock_cmd`/`_cdoc_cmd`/`_cdl`/`_cdir_cmd`/`_cdict`/`_cd`/`_cbox`/`_catch2`/`_CMD`（大小写） |

**族 B：跨 harness 命令名（16 次；共前缀最长 `run_c`=5＜8、Lev 超预算——规则够不到，硬表收录 5 名 14 次）**

| 次数 | 名 | 处置 |
|---|---|---|
| 9/2/1/1/1 | `run_command`/`run_cmd`/`exec_command`/`terminal_exec`/`run_command_name` | **入表**→`run_terminal_cmd` |
| 1/1 | `invoke`/`run_task` | 不入表（目标不唯一：terminal 抑或 tests？）＝无近似不附 |

**族 C：真离群（3 次；无近似物，负例钉载体）**：`SEMANTIC_SUMMARY`×2（单 run `RUN-CLI-6abd89b5`，模型自造语义摘要工具）、`cursor_agent`×1。

**历史档对账**：0S FP-5（2026-09-09）29 次＝本表族 A/B 子集（该轮只覆盖 unsolved-20）；0P S4（2026-09-08）`run_terminal_card`/`_calls` 在族 A；TB40 O4（2026-09-11）`cursor_agent`/`exec_command`/`run_cmd` 在族 B/C。**0S FP-5 当时已登记「did-you-mean 教学」候选观察（可选低优先）——0cs 立项即其承接**；0S 实证「模型次轮自恢复，成本 ~1 轮/次」＝S3 真机判据基线。

## §2 落码细节

1. **闭集硬表**：`NOT_FOUND_SUGGESTION_TABLE: &[(&str, &str)]`＝5 条（§1 族 B）；const 静态、更新随批附 journal 证据（近两月全量仅此 5 名够不到，预期增速极低）。
2. **规则臂**：大小写不敏感（ASCII）；`levenshtein_capped`（预算 2，行长差>2 早退、行最小值越限早退）或 `common_prefix_len`≥8；阈值 8 的依据＝族 A 最短成员共前缀 14（`run_terminal_c`），而族 B 最长共前缀 5（`run_c`）——单阈值即分离两族，无需词元臂（语义推断禁令下的第三条路已被否）。
3. **表命中在册校验**：`table_hit = 表[key].and_then(target ∈ registered?)`——目标不在册退化走规则臂（等效无近似不附）；叠加运行时优先级（精确命中恒先行），表在任何注册表演化下都不会误导。
4. **漏斗改造**：`tool_not_found_error(tool_name, registered)`——`registered` 由三调用点各自从持有的 `tools` 读守卫现取 client_name 列表；消息形态 `Tool not found: {name}` ＋ `; did you mean "a"?`／`; did you mean "a" or "b"?`（0–2 条，单文案形状）。

## §3 钉子（11，registry/types.rs tests 模块）

| # | 钉 | 臂 |
|---|---|---|
| 1 | `not_found_suggestion_table_is_pinned` | 表长＝5（0cb 式防膨胀） |
| 2 | `not_found_suggestion_table_disjoint_from_and_pointing_into_managed_tools` | 表键∩`MANAGED_TOOLS` client 名=∅＋表目标全部指向在册 managed 工具 |
| 3 | `not_found_suggestion_table_arm` | `run_command`/`exec_command`→`run_terminal_cmd` |
| 4 | `not_found_suggestion_table_requires_registered_target` | 表目标缺册→空（无近似不附） |
| 5 | `not_found_suggestion_prefix_arm_run_terminal_patch` | **pin (a)**：`run_terminal_patch`→`run_terminal_cmd` |
| 6 | `not_found_suggestion_levenshtein_arm` | `run_terminal_cpt`→`run_terminal_cmd` |
| 7 | `not_found_suggestion_case_arm` | `RUN_TERMINAL_CMD`→`run_terminal_cmd` |
| 8 | `not_found_suggestion_no_near_miss` | `SEMANTIC_SUMMARY`/`cursor_agent`/`invoke`→空 |
| 9 | `not_found_suggestion_capped_at_two` | 至多 2 条 |
| 10 | `not_found_error_message_carries_hint` | 信封 detail 文案＋kind/details 形状不动 |
| 11 | `try_parse_unknown_tool_carries_suggestion` | finalize（`GrokBuild:run_terminal_cmd`）→`try_parse` 端到端装配 |

## §4 验证明细

- orz-tools **3009/0**（2998→3009＝＋11 恰钉数；6 ignored 同基线）。
- clippy **26=26 零新增**：本批树 29 → 触碰面 3 条（`manual_ignore_case_cmp`×2＋`collapsible_if`）当批修复（`u8::eq_ignore_ascii_case`＋`table_hit` and_then 化）→ 26；stash 对拍冻结树 `a7526cc2`＝26。
- fmt 净（`cargo fmt -p orz-tools --check`）。
- 源清单随 orz `2d51d22c` 再生成：`orz_source_manifest.sha256` 差异恰 1 行＝types.rs 哈希。

## §5 边界与后续

- **S2 载体**：随下一重建批进体（版本 0.8.14；先于 0cr S2 跑批——190 批排期位）。0cr 立项冻结面写的「载体 0.8.13（`0e1c10e7`）」需在重建批随版本号顺延更新，判据表换窗同 187 批形态。
- **S3 真机**：随 0cr 首题顺带——幻影工具名事件改道轮数下降（对照基线＝0S FP-5 ~1 轮/次自恢复；台账沿 181 §4b 口径）。
- **不重建不推送**；本批双仓本地提交（orz `2d51d22c`＋父仓落账批）。

# 073 — 未提交内容复审与补口批（2026-09-24）

> 工作区 `D:\CLI`；orz 子模块 worktree HEAD `a46c7c02`（未提交）／父仓 HEAD `b5e084ed`。
> 用户令：「请先查看CLI_PROJECT_INDEX.md路由，随后回查所需文档」「请对当前未提交内容进行全面检查，包括设计合理性、实现合理性、设计与实现的符合性」「剩下的其他问题都可直接处理，能直接补的都补，不退一步仅做记录」。
> 复审对象＝worktree 四批未提交落码（0bi 四件／0bl 十件／0bk S2／必定压缩补足批）＋父仓文档批。按令**不提交／不推送／不重建**。

## §0 结论速览

- **复审结论**：无 P0／无 P1。三个审查子代理并行（编辑面／压缩面／取消与宿主面）＋主代理逐条亲核五条最重发现（全部属实）；27 个改动文件的 diff hunk **全部可归属已登记件，无未登记私货**；三套红判据独立复现逐位一致（orz-tools 2926/0/6、orz-loop 串行 845/0/3、orz-host 串行 322/0/5）；门禁唯一红＝orz 脏树（预期态）、台账钉子 15 OK。
- **处置**：P2×4 与 P3×9 同日补口落码／修档（§2）；**P2 前两件（T1 升级链停摆边缘、强制窗块缺摘要模板）按用户裁决单独处理**（§3，本批不动）；卫生两件（§4）。
- **补口后判据**：orz-tools / orz-loop / orz-host 三套全量档复跑全绿（读数见 §5）。

## §1 复审发现总账（分级随处置状态）

| # | 级 | 发现 | 处置 |
|---|---|---|---|
| R1 | P2 | T1 必定压缩升级链停摆边缘：强制窗收口轮「产出摘要但压缩未落地」⇒ 既不升级也不 rearm，T1 闩已消费 | ⏳ 单独处理（§3） |
| R2 | P2 | 强制压缩窗块「产出语义摘要块（见上）」悬空引用——块内不含摘要模板 | ⏳ 单独处理（§3） |
| R3 | P2 | 0bl⑤ 守卫性质测试无回归鉴别力（放行测试 measured=100K 虚高，旧公式同放行） | ✅ 放行测试改诚实 measured（60K 字符载荷 ⇒ 新公式放行／旧公式 after≈41K>18K 必拦），拦放双钉合看可分辨 |
| R4 | P2 | hashline 既有文件路径无 UTF-16 门（0bl ③ 修洞未随 0bl ② 带入 hashline） | ✅ 判定上移 `util::encoding::utf16_shaped_input` 单点（含残余边界注记），hashline 接入同门＋钉子 `hashline_edit_rejects_utf16_shaped_file`（逐字节不动断言） |
| R5 | P2 | 0bl ⑥⑦ 新杀点对 0bm ③ 的前向耦合未在裁决档点名 | ✅ TODO P1-0bm ③／BACKLOG 0bm ③ 补实施约束（保留全局可见杀伤面或 per-dispatch scope 同步接入杀路径＋回归钉） |
| R6 | P2 | host 定向杀注释保证强于机制（spawn sink 全局单槽竞态下的**误杀方向**未记，只记了漏杀方向） | ✅ `orz-host/lib.rs` 残余取舍补双向记录（误杀半径不大于旧全局杀；根治随 0bm ③） |
| R7 | P3 | 工具层 sha 注释滞后（仍说 loop 层用 `!=`，072「注释互指闭环」只成立一半） | ✅ 注释更正为双向一致 |
| R8 | P3 | anchor 模式清空行窗被 no-op 守卫误拒（`new_string=""` 触发「same」误导报错） | ✅ 守卫改 `anchor.is_none() && old==new`；顺带修空串清空整文件多产一个空行（终结换行空串特例）；钉子 `anchor_mode_allows_clearing_the_window_with_empty_new_string`（清空整文件⇒零字节／清空中段⇒前后缀保留） |
| R9 | P3 | hashline 整文件剥离会动既有 emoji，与设计档「历史文件不改写」字面冲突且口径未声明 | ✅ 实现注释＋设计档 §3 补记「本次写入＝整文件」解释（search_replace 口径不变） |
| R10 | P3 | `压缩块: 1-4、6` 顿号分隔被当注解静默截断（对账行还显示「全部命中」） | ✅ `、` 入分隔集（`parse_block_selection`）；钉子随解析钉扩展 |
| R11 | P3 | `5-3`（b<a）拒路径无直接钉 | ✅ 解析钉补断言 |
| R12 | P3 | 可压集合为空时告知块「当前可压区间：」后接空串 | ✅ 空集渲染「（无）」 |
| R13 | P3 | T1 窗口路径下低档 fire 的 form 记 `deferred_to_truncation_notice` 名不副实（实为被必定压缩窗口块取代） | ✅ 窗口路径改 `superseded_by_mandatory_window`；截断路径（无可压分块／种子直达第三步）维持旧值；既有截断钉不受扰 |
| R14 | P3 | `agent_loop.rs` 阶梯段「每档每会话一次、不 rearm」注释与本批 T1 重新武装语义不符 | ✅ 注释更正 |
| R15 | P3 | 取消延迟解耦措辞「纯内存计算」偏窄（实际只覆盖子进程族；web_search/CDP 等进程内阻塞面仍与超时耦合） | ✅ `tool_run.rs` 注释改口径「无子进程的调用面」并如实记录解耦边界 |
| R16 | P3 | emoji 范围表无快照语义声明（Unicode 15.1 冻结，未来新 emoji 落 gap 不剥） | ✅ 表头注补 snapshot 声明＋重生成指引 |
| R17 | P3 | emoji 告知行拼接在输出已有尾换行时产生一个空行 | ✅ 仅在无尾换行时补分隔（search_replace＋hashline 两处同改） |
| R18 | P3 | `acp_server.rs` 生产构造器注释仍称反例门「仍在终答前加一轮」（0bi ⑩ 收窄后失真） | ✅ 注释更正 |
| R19 | P3 | 0bm 前置建议措辞「纳入 0bl 十件」偏窄（worktree 另有 0bk S2＋必定压缩批待进载体） | ✅ TODO／BACKLOG／INDEX 三处改「全部未提交落码四批」 |
| R20 | P3 | 两处零语义 rustfmt 重排未登记（`tool_run.rs:2107/2560` 表达式重排、`orz-loop/lib.rs` pub mod 移位） | ✅ 本档登记（零行为变化，随 0bl 批带入） |
| R21 | P3 | 指针化分支（无可压分块）不带「已两次开窗未产出」事实——亲核后**确认无需改**：该路径 T1 直达截断（`t1_cut` 含 `t1_compressible==0`），本就未开过窗，携带该事实反而是虚构 | ✅ 复核结案，不改 |
| R22 | P3 | 跨 episode 升级计数携带（读数经非压缩途径跌破再越线，新 episode 从第二次询问起步）——设计字面允许 | 维持（设计口径内） |
| R23 | 既有 | `DispatchGuard::drop` take-all/put-back 交错可遗留他人条目（**已提交存量代码**，token 单调使残留条目 inert） | 随 0bm ③ 一并收口（已同 R5 约束同域） |

## §2 补口改动面（全部在 orz worktree＋父仓文档，未提交）

- **orz-tools**：`util/encoding.rs`（`utf16_shaped_input`/`utf16_rejection_message` 上移单点＋残余边界注记）；`grok_build/search_replace/mod.rs`（sha 注释双向更正、no-op 守卫 anchor 感知、空串清窗终结换行特例、中段分隔换行空串特例、告知行尾空行、钉子＋1）；`grok_build_hashline/edit/mod.rs`（UTF-16 门接入＋整文件剥离口径声明＋钉子＋1）；`util/emoji_strip_ranges.rs`（快照语义头注）。
- **orz-loop**：`agent_loop.rs`（守卫放行测试诚实化＋注释、form 标签窗口路径更正、阶梯段注释更正）；`context_scale.rs`（顿号分隔、`5-3` 钉、可压空集「（无）」）；`host_exec/tool_run.rs`（取消解耦边界注释）。
- **orz-host**：`lib.rs`（定向杀残余取舍双向补记）；`acp_server.rs`（生产构造器注释更正）。
- **父仓文档**：TODO P1-0bm／BACKLOG 0bm（③ 实施约束＋前置建议措辞）；INDEX v4.35 头行（前置建议措辞）；emoji 设计档 §3（hashline 整文件口径补记）；本档（R20 登记＋复审总账）。

## §3 P2 前两件（R1/R2）——单独立项 `0bn`（2026-09-24 同日）

两件均在压缩核心（`agent_loop` 强制窗收口状态机／`context_scale` 强制窗块文案），经主代理机制讲解与方案评估后**用户裁决：单独立项 ＋ 采纳主代理「就地小修」标准解 ＋ 补充设计文档**（不随本补口批落码）：

- **标准解（定案）**：① R1＝强制窗收口且压缩未真正落地 ⇒ 同计为「未产出」（计入升级计数并复位 T1 闩）；② R2＝摘要格式模板内嵌强制窗块本体。**被否备选留档**＝从上下文预算豁免／常驻化（破零常驻注入纪律、不解决 R1 状态机本体、牵连改写「仅留主滑块」语义）。
- **设计补充**：[`v8 设计稿 §15`](../CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md)＋ADR-0010 §14.77 复审补记（v1.80）。
- **账面**：BACKLOG/TODO **0bn** 立项（50 → 51；S1 设计已勾，S2 落码建议随 0bm 狗粮轮载体重建前置，S3 随轮观察）；INDEX v4.36 头行（v4.35 滚档）；第二卷 §1.12。

## §4 卫生

- 根目录无关草稿 `tmp-launch-browser.ps1`（浏览器批量打开脚本，与本仓无关）——按用户令删除。
- `.zcode/`（本地工具状态）——按用户令加入 `.gitignore`。

## §5 验证

- 补口定向面：orz-tools `-- anchor emoji utf16 hashline bom` 272/0；orz-loop `-- context_scale guard_ mandatory t1_ ladder block_selection` 27/0。
- 三套全量档（补口后复跑，串行档照旧）：orz-tools **2928/0/6**（＋2 钉＝`anchor_mode_allows_clearing_the_window_with_empty_new_string`／`hashline_edit_rejects_utf16_shaped_file`；解析钉为既有用例加断言不增数）、orz-loop 串行 **845/0/3**、orz-host 串行 **322/0/5**。
- 门禁：`python scripts/check_repository.py` 唯一红＝orz 脏树（预期态）；台账钉子 15 OK。

## §6 入口

[`TODO P1-0bm`](../../TODO.md) / [`BACKLOG 0bm`](../BACKLOG_AND_PRIORITIES.md) / [`072 审计档`](072_FULL_REVIEW_REMEDIATION_2026-09-24.md) / orz worktree 未提交改动。

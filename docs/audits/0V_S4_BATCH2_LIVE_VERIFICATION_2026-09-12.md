# 0v 第二批 S4 实机复验记录（2026-09-12）

> 入口：BACKLOG **0v** / TODO P0-0v——0v 第二批（引擎链软备忘 + 0v-A 引擎级
> 取证面）S4 实机复验：(a) dna-assembly 复跑对照 + (b) 0v-B 定向探针。
> 载体：orz 0.4.3 三件套（`D:/tb-eval/orz-linux/orz`，sha256 `eb724134…`，
> 与 [S3 重建记录](0V_S3_BATCH2_DUAL_PLATFORM_REBUILD_2026-09-12.md)锁定值
> 一致）；口径 k=1、`-r 0`、官方墙钟唯一、deepseek-v4-flash、eval_browser=
> true、Chromium 只读挂载、无代理直连。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0v-s3-20260912\`（本地留存，
> gitignore 覆盖）：`journal-analysis-s4a.txt` / `journal-analysis-s4b-probe.txt`
> / `serp-attempts-inventory.txt`。
> 0v-B 探针工件：任务 `D:/tb-eval/probe-0vb/`（固定 12 查询，装置侧产物不入
> 库）；执行器 `scripts/run_0vb_probe.py`（入库）。

## 0. 跑批事实

| 批次 | 任务 | 时间 | 时长 | 事件 | 终止形态 |
|---|---|---|---|---|---|
| (a) s4-0x-0v-b2 | dna-assembly | 11:21–11:54 | 33m12s | 334 / 31 模型轮 | **无终止事件（墙钟杀死）** |
| (b) s4-0vb-probe | serp-probe-0vb | 12:05–13:07 | 61m41s | 1348 / 132 模型轮 | **无终止事件（墙钟杀死）** |

0x 搭车复验两场再通过：`initial_round` 各恰好 1 次、`post_tool_batch_gap`、
三问 3/3、动作中零复发、机械审查（15/55 条）不含三问；探针场另见周期问询
2 次（external 车道 50 轮触发，`post_tool_batch_gap` 与 `loop_top_gap` 各一）
独立于 initial_round——互不影响获得**双会话实机样本**（0x S4 时为单会话样本
的边界补强）。

## 1. 判据总表（§4 判据 1–10 + 第二批新增 11/12）

| 判据 | 结论 | 关键证据 |
|---|---|---|
| 1（Google 失败置尾、成功路径不再试） | **✓ 决定性** | 探针首次 search：google failed(blocked, 19165ms)→bing ok→ddg not_attempted；其后 **26 次 search 尝试序恒为 bing:ok\|ddg:not_attempted\|google:not_attempted，google 真实尝试计数恒 1** |
| 2（navigate/search 零误拒） | **✓** | (a) 9 + (b) 28 次 browser_control 零权限误拒；`retrieval_role_write_denied` 5 例全部为 external 车道的 `run_terminal_cmd`/`search_replace`（写域动作）被车道门**正确拒绝** |
| 3（冷却/上限机械生效） | 部分 | 冷却/上限无拒绝样本（会话上限 40 未触及）；机制面由 S2 单测覆盖 |
| 4（零 400 / 命中率 ≥90% / 延迟有界） | 部分 | 零 400 ✓；延迟有界 ✓（browser_control 均值 3.2s / 最大 20.3s）；**命中率 81.66% / 87.71%，两场均 <90%**（观察项，见 §3） |
| 5（CAPTCHA/empty 识别为失败） | 部分 | (a) Bing 间歇墙页 5 次被识别为 `empty` 失败、不混入结果 ✓；CAPTCHA 无样本；**分类偏差观察**：连接层失败（Chrome 落 `chrome-error` 页）被最终 URL gate 归类为 `blocked` 而非 network/timeout（§3） |
| 6（en-US 生效） | **✓** | Chrome History 143 条 URL 直接可见 `setmkt=en-US` |
| 7（低质量加权标注+排尾不过滤） | **✓ 决定性** | (b) 18/27 份文件带 `low_quality` 共 43 条、266 结果；**零排序违例**——low_quality 全部排在同引擎尾部（如 8×default+2×low_quality）；内容农场查询（essay/gummies/spoofing/casino）如实标注 |
| 8（旧 journal 回放兼容） | **✓** | 0.4.2 生产 journal（dna-assembly-s4b，1050 事件）经 0.4.3 `journal-conformance` **OK** |
| 9（车道预算派发前拒） | 未取得 | (b) 模型把 12 查询拆成多个外部检索 activation，单 activation 内 search ≤4 次、`lane_budget.used` max 4 ≪ 16，未打穿；主车道无 search 调用。机制面由 S2 单测覆盖 |
| 10（会话底线） | 未取得 | 头寸条件达成（会话导航 28 → 头寸 12 ≤ 16），但主车道无 search 需求，`session_reserved` 拒绝未发生 |
| **11（软备忘）** | **✓ 决定性** | (b) 失败引擎置尾不移除（26 次重试序稳定）、`not_attempted` 53 条 / `skipped` 0 条、成功路径不试已置尾者；(a) `all_engines_failed` ×5 每条均三引擎真实失败（error_class+wall_ms 齐）；全员置尾退回固定链序（(a) 六次调用序恒 google→bing→ddg） |
| **12（取证面）** | **✓ 决定性** | (a) 6 文件↔6 search 调用逐条对应（含信封逐字段、`lane_budget`/`session` 读数）；(b) 27 文件、同轮 `-2`…`-12` 后缀顺延实机可见；**两 run 均被墙钟杀死而取证文件完整可复核**——本批 journal 链在树杀后损坏（§3），取证面恰为判据 12 的立意提供了对照证明 |

## 2. 软备忘语义的实机形态（判据 11 详证）

- **探针场（Bing 全程可用）**：首次 search 三引擎尝试序 google→bing→ddg，
  google 被 URL gate 拦（`blocked`，19165ms——连接层失败见 §3）后置尾；
  此后 26 次调用尝试序恒 bing→ddg→google 的表形态 `bing:ok |
  duck_duck_go:not_attempted | google:not_attempted`——**置尾持续全会话、
  成功不清除备忘、成功路径零浪费导航**，与设计 §8.2 逐条吻合。
- **dna-assembly 场（三引擎间歇/全败）**：六次 search 全部三引擎真实尝试
  （google 19.2s / bing 0.4s / ddg 19.1s 恒定失败形态），5 次
  `all_engines_failed` 每条带全量 error_class+wall_ms，1 次bing 间歇恢复
  成功（10 结果、low_quality 1）——**同会话内先前失败的引擎仍被真实试到**
  （备忘短路不存在）、失败只影响顺序。
- 对照上批（0.4.2 剔除式备忘）「首搜 44.7s 后 3 次 0.9–3.9s 零导航」的
  短路形态：本批 (a) 六次 search 全部真实导航（会话导航 3→17 单调）——
  备忘语义的版本间差异在实机面可辨。

## 3. 观察与缺口登记（不动码；0v 闭合裁决留用户）

1. **命中率两场均 <90%**（81.66% / 87.71%；判据 4 口径）。与三引擎失败
   重试的会话形态相关（(a) 三引擎全败重试消耗了大量非缓存上下文）；零 400、
   哨兵零触发不受影响。是否收紧判据口径或接受波动，留用户裁决。
2. **连接层失败的分类偏差**：Google/DDG 不可达（连接挂起 ~19s 后 Chrome
   落 `chrome-error://` 页）被最终 URL gate 归类为 `blocked`（"unsupported
   URL scheme: chrome-error"）而非 `network/timeout`。失败识别本身正确
   （不混入结果），但类别失真会误导置尾读数解读（`blocked` 原语义 =
   URL gate 对目标 URL 的策略拦截）。登记为分类精化候选，不阻塞。
3. **Bing 间歇墙页**：(a) 窗口内 Bing 5/6 次 `empty`（350–450ms，快败），
   1 次 981ms 成功 10 结果；(b) 窗口 Bing 全程可用。empty 识别为失败 ✓，
   但「无 captcha 标记的软墙」与真实空结果不可区分——判据 5 的 CAPTCHA
   面待真实样本。
4. **新缺口（候选 0v-C）：墙钟杀死 run 的 journal 完整性**——两份 run
   均无终止事件且链校验 INVALID（(a) 4 错：pdf_document evidence 枚举
   （法官枚举缺该 kind——GAP-PDF-EVIDENCE（2026-08-11）既有的生产者/法官
   漂移，先前 run 未踩到，另案）+ sha 断链×2 + terminal 缺失；(b) 8 错：
   sha 断链×5（全部集中在 592–769 外部检索 activation 接缝区）+
   `retrieval_result_committed` 的 result_digest/ledger_digest ×2 + terminal
   缺失）。对照：全部正常结束的历史 journal（0.3.x/0.4.1/0.4.2）链完整；
   断链仅出现在「墙钟杀死」形态——0k P3-4 的终止合成收口未覆盖外部接缝
   与树杀写入窗口。**判据 12 的取证面设计价值被本批直接实证**：journal
   损坏的同时 `serp-attempts/*.json` 逐次落盘完整幸存、可事后复核。
5. **0v-B 打穿未达**：模型把固定查询集拆成多次检索委托（单 activation
   内 search ≤4），判据 9/10 未取得实机样本。再探针需在 instruction 中
   约束"一次激活内完成全部查询"或改由主车道直调——留用户裁决是否加跑。

## 4. 0v-C 深挖：断链根因定位（2026-09-12 同日，静态分析 + 最小复现）

排查路径：数据面四重排除（行级全部自洽——重写假说否；sequence 单调无重复——双写/乱序否；单
`run_started` + manifest 唯一——进程重启否；正常结束历史 journal 链全部完整）；代码面单 writer 纪律
核证（recorder 单 task 串行 + 每行 fsync、EventWriter prev 单赋值点、子代理与主车道共用 writer、
host RunRecorder 仅独立 run）；**最终以最小复现实锤**——

**根因（结构性缺陷，`JournalRecorder::record_async`，orz `crates/orz-assurance/src/journal/recorder.rs:116`）**：

```rust
let mut event = event;
orz_secrets::redact_json_string_values(&mut event.payload);  // 0p S2 B5 脱敏漏斗（改 payload）
seal_event(&mut event)?;                                     // 重 seal —— 行内 sha = 脱敏后
```

而 emit 侧（`EventWriter::record`，orz-loop controller.rs）在调用 record_async **之前**已 seal 一次并取
`event.event_sha256` 作为链推进值：

```
emit: seal(原文) -> event_hash = sha(原文) -> record_async（脱敏改 payload -> 重 seal -> 落盘 sha(脱敏后)）
      -> writer.prev_hash = sha(原文)   ← 脱敏后该哈希对应的原文从未落盘
```

**任何 payload 被脱敏命中的事件都会使其后一行的 `previous_event_sha256` 指向一个不存在于文件的内容
版本 → 链断裂**。最小复现（临时测试，已删）输出：落盘行文本带 `[REDACTED_SECRET]`、行内 sha
`4a0758…`、emit 侧推进值 `fefd93…`——`prev 链断`形态与两份实机 journal 的 7 处断链逐项吻合。

- **引入点**：0p S2 B5 脱敏漏斗（2026-09-07，0.3.2）——**该缺陷自 0.3.2 起存在，与本批 0v 改动无关**；
  正常结束的历史 journal（0.3.x/0.4.1/0.4.2）链完整只是**恰无脱敏命中**（回放兼容判据 8 不受影响）。
- **修复方向**（待立项）：①record_async 把重 seal 后的 hash 回传调用方（改返回类型为
  `Result<String, _>`，emit 侧以回传值推进）；或②脱敏上移到 emit 侧构造 payload 时（seal 前一次完成）。
  修复须配正式钉子测试（脱敏命中事件的后一行 prev == 行内 sha）+ 旧断链 journal 的只读回放兼容注记。
- **残留疑点（交 orz 真机同步复核）**：s4a@98/196 的前一行（seq97/195 model_output）payload 用
  `redact_secrets` 全文重放**零改动**——脱敏触发源未逐行定位（候选：payload 深层字段/URL 规范化
  无痕改写/其他 record_async 调用点）。触发源定位不阻塞修复立项（机制已实锤）。

## 5. 结论与边界

- **0v 第二批 S4 执行完毕**：判据 1/2/6/7/11/12 成立（其中 1/7/11/12 为
  决定性实机证据）、3/4/5 部分成立（观察项 §3.1–3.3）、8 成立、9/10 未
  取得（§3.5）。0v 是否闭合、判据遗留（9/10、CAPTCHA 样本、分类精化）
  与 0v-C 缺口立项，**留用户裁决**。
- 未验证：检索车道 navigate 的 `browser_read` 主面交互、Phase 2 交互动作、
  Windows 平台行为。
- 本批不改码：全程零 orz 源变更（载体 0.4.3 = S3 冻结基线）。

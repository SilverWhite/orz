# P2-13 B3 全面审查处理（2026-09-03）

> 性质：审查处理记录（2026-09-03 对 P2-13 B3 契约与收尾当前实现的全面
> 检查——设计合理性 / 实现合理性 / 设计与实现符合性——所发现问题的处置
> 落地）。B3 范围定义见
> `P2-13_B3_IMPL_AUDIT_2026-09-03.md`；设计权威
> `BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md` 随本批升
> **v0.9**（B3 复审裁决登记）；ADR-0010 §14.52 不变。
> 结论：无 P0；P2 三项（疲劳档位状态机语义 ×2 + 存档时效边界 ×1，均已按
> 用户裁决处理）、P3/INFO 若干（代码修复或决策登记）。

## 处置清单（审查发现 → 处置）

1. **70 档压缩 ≥2 门槛冗余（P2-2，用户裁决：移除门槛）——代码删除**：
   疲劳提醒只按黑板 live 字节水位判定；50/70/90 三档不再要求会话内
   `context_compressed ≥2`。落点：`orz-loop/fatigue.rs` 去掉
   `compressed_total` 参数与门槛分支、70 档文本直接给换对话建议；ACP
   `StoredConversation.context_compressed_total`、`is_zero_u64` 与两处
   `count_journal_event_type` helper（acp_server / orz-bin）全部删除；
   CLI 与 ACP 不再读取压缩事件计数。理由 = W=10 MiB 对纯文本已足够宽松，
   压缩轮数判定多余（用户裁决原文口径）。
2. **巨幅跳跃后低档滞留补发、档位顺序颠倒（P2-1，用户采纳建议）——
   代码修复 + 测试**：`pending_fatigue_notice` 返回
   `FatigueDecision { notice, tiers_to_mark }`——单次只取达到的最高未提醒
   档；同一次判定内把已越线的未提醒档全部列入 `tiers_to_mark` 由宿主落档
   （跳过即视为已投递，低档不滞留到后续 run 倒序补发）。已提醒档的最高
   水位线以下视为「已跳越作废」（前缀闭包纪律，兼容异常 legacy 侧车态）。
   新增测试：巨幅跳跃只报 90 且 50/70/90 全落档、后续 run 静默；
   正常爬坡按 50→70→90 各一次；legacy 只记 70 时 50 不补发。
3. **close 时仍有 in-flight run → 存档可能缺尾部（P2-3，用户指定方案）——
   代码修复 + 测试**：`AcpServer` 新增 `pending_archives` 票表；
   `close_session` 若检测到 `RunInFlight::Prompt` 存在则挂票不立即归档，
   由 `handle_session_prompt` 收尾路径（成功 persist 后 / 失败取消路径 /
   注册后早期失败路径）消费并补触发存档。新增单测
   `close_with_active_run_defers_archive_until_run_completion`（close 后
   不落包 → 消费票后落包）。
4. **陈旧注释（P3）——代码清理**：`agent_loop.rs` 压缩 marker 注释链压缩
   为现状一句（v1.15 来源 + B3 生产面退役说明）；「（无注意事项）」陈旧
   注释改「（无）」；`summary.rs` 测试注释标注 --plan/测试域语境。
5. **存档文件名字面差 `<session>` vs `<session8>`（P3）——登记**：沿用
   conversations sidecar 的 8 字符前缀约定（`.gsa/archives/<session8>
   .json.gz`）；设计 §11.3 升 v0.9 登记口径，代码注释同步。
6. **Windows 替换归档 remove+rename 非严格原子（P3）——登记**：首次写入
   原子（tmp+rename）；替换既有归档在 Windows 下先 remove 再 rename。
   代码注释与设计登记（不改行为；同 8 字符前缀重复归档为罕见路径）。
7. **sidecar 损坏只 warn、无 failed 事件（P3）——登记为边界**：源 sidecar
   缺失（从未有成功 prompt）或损坏属「无存档尝试」而非「落盘失败」，仅
   warn 不产生 `session_archive` 事件；`archive_session_package` 重构后
   注释明示该边界。
8. **压缩计数子串匹配伪命中风险（P3）——随 P2-2 消除**：压缩计数链路整体
   删除，helper 不复存在。
9. **同步 gzip 阻塞 async worker（P3）——代码修复**：`archive_session_
   package` 拆为 blocking 打包（`tokio::task::spawn_blocking` →
   `package_session_archive`）+ async 审计（ARC journal）两段。
10. **orz-tui 等 ACP 客户端不展示 `user_notice`（P3）——登记**：展示通道
    只覆盖 codex_app（item/completed 附言）与 CLI stderr（§11.2 降级）；
    orz-tui 为开发/回退客户端，暂不补展示，随宿主面清单登记。
11. **ARC run journal 撞名风险（P3）——登记**：`ARC-<session8>-
    <prompt_count>` 与既有目录撞名（会话 id 复用 + 同计数重复归档）时
    bootstrap 会向既有目录追加 seq=0 preflight；当前「每会话 close 一次」
    语义下罕见，暂不做检测（close 有 active run 的推迟路径不会二次归档，
    票消费即删）。
12. **测试缺口（P3）——补齐/登记**：疲劳两语义序列（先事实后建议已随
    P2-2 消失；跳档多 run 序列新增覆盖）；ACP prompt e2e（user_notice
    进响应不进持久化）与 codex_app 附言无独立 e2e，随 B4 页面级验证补
    （登记待续）；close-with-active-run 以机制单测覆盖。

## 验证（2026-09-03 实测）

- orz-loop lib：**691 passed / 0 failed / 3 ignored**（fatigue 模块 5 → 7
  测试，覆盖跳档落档/正常爬坡/legacy 态）。
- orz-host lib：243 passed + 1 预存 flake（`call_tool_timeout_kills_
  process_tree` 单独复跑通过）/ 4 ignored（新增 close-with-active-run
  存档推迟测试 + 既有存档单包测试）。
- orz-bin：单测 11 passed / 12 ignored；stdio_e2e 1、acaf_e2e 23、
  real_flag 2 全绿。
- fmt：`cargo fmt --all -- --check` 干净；clippy 无新增告警（新代码段无
  warning，既有基线告警位置不变）。

## 登记状态与边界

- 设计稿 BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN：v0.8 → **v0.9**（疲劳
  门槛撤销、跳档只报最高档、存档命名口径、存档触发推迟四项登记）。
- TODO P2-13 / BACKLOG §13 / CLI_PROJECT_INDEX（v2.46 → v2.47）随本文件
  同步登记。
- 待续：B4 验证排期不变（S3 重建 + S4 复验 + 页面级：OCR 水位/提醒档位、
  会话存档包与 ARC- journal、close-with-active-run 存档时序的实机确认）。

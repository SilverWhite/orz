# P2-13 B1 全面审查处理（2026-09-03）

> 性质：审查处理记录（2026-09-03 对 P2-13 B1 会话化基础当前实现的全面
> 检查——设计合理性 / 实现合理性 / 设计与实现符合性——所发现问题的处置
> 落地）。B1 范围定义见
> `P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md`；设计权威不变
> （`BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md` v0.8 +
> ADR-0010 §14.52）。结论：无 P0/P1 阻断；P2 三项 + P3/INFO 若干，全部
> 收口或登记。

## 处理清单（审查发现 → 处置）

1. **failure_agg 轴语义声明与 B1 实现冲突（P2）——语义收口 + 文档取代**：
   `failure_agg.rs` 模块头与 `CONTEXT_COMPACTION_DESIGN` §4.4.4 的
   「run 作用域 / ACP 每 prompt 重建黑板 / 恢复同 epoch 继续累计属未来
   切片」与同提交 B1（会话快照跨 prompt 保留黑板 + LIF 轴原点 =
   session_started_at）冲突。
   决策：ACP 会话续载路径下 failure_agg 为**会话作用域**——聚合行 t/轮号
   会话相对、跨 prompt 单调累计（P2-12 登记的「未来切片」随 B1 落地）；
   CLI 单 run 与 `--plan`/epoch 轮换路径维持 run-relative 与随轮换重置。
   落地：`failure_agg.rs` 模块头加 B1 取代注；CONTEXT §4.4.4 已知边界加
   取代注。
2. **entities 恢复版本计数口径矛盾（P2）——代码修复**：
   `Blackboard::restore_conversation_snapshot` 现于 `entities.reset_revision()`
   后调用新增的 `EntityRegistry::bump_revision()`，实体分区与其他恢复分区
   一致归位「1 次变化」（与 entities.rs 注释、B1 审计 §1.2 对齐）；
   `conversation_snapshot_and_restore_keep_board_but_strip_run_faces`
   断言改为 entities=1、deps=0（空图静默）。
3. **失败 run 后检索分区在两套侧车间优先级未定义（P2）——语义决策 + 代码
   修复**：
   决策：检索分区归属 **activation 侧车生命周期**（派发全量覆盖、run 结束
   无条件下沉、跨失败 run 延续既有语义），为会话恢复时的权威源；会话黑板
   快照中携带的分区副本在 activation 无分区时作回退。
   落地：`AgentLoopController::with_retrieval_partitions` 改 compare-and-set
   （内容一致不覆盖、不产生徽章噪声）；ACP builder 顺序改为先
   `with_live_blackboard` 后 `with_retrieval_partitions`；新增单元测试
   `retrieval_partitions_overlay_after_conversation_restore`（同内容保持
   restore +1、不同内容覆盖 +1、未提供分区保持快照内容与徽章）。
4. **B1 审计措辞精度（P3）——登记口径**：审计 §1.1「统一写点」实为两形态
   ——章源统一为 `blackboard_stamp()`（LIF 单一来源，exec/edits/
   tool_actions/receipts/failure_agg/retrieval 同源），tool_actions 部分
   路径经 `push_tool_action_stamped`，exec/edits/receipts/retrieval 在各
   写点直写但均盖同源章；「持锁块内取章」实为 LIF 锁内取章 + 黑板写锁
   独立获取（loop 串行下单写者安全）。历史审计不重写，以本文件口径为准。
5. **进程重启「黑板 + LIF」续接缺 e2e（INFO）——登记 B4 验证项**：在既有
   `new_server_resumes_conversation_from_sidecar` 之上扩展黑板/lif 断言
   或新增等价用例（当前由序列化 roundtrip + 进程内跨 prompt 测试覆盖）。
6. **legacy 侧车轴混合（INFO）——登记边界**：legacy `temporal_spikes` 的旧
   t 为旧 run-relative 近似，新轴原点取加载时刻墙钟，新旧刻度不可比属
   legacy 迁移近似（新会话全量会话相对，无此问题）。
7. **序列化体积 / schema_version 演进（INFO）——登记**：ExecEntry 每行固定
   序列化 round/domain/ts（含 pre-stamp 空字段），W=10MiB 与 B3 疲劳口径
   按新形状校准；`StoredConversation.schema_version` 为 0.2.0-draft、解析
   不校验版本，单一消费方下无碍，存档/演进切片（B3）时登记规则。

## 验证

- orz-loop lib 全量：**658 passed / 0 failed / 3 ignored**（净增 1：
  `retrieval_partitions_overlay_after_conversation_restore`）。
- orz-assurance lib 全量：**201 passed / 0 failed**。
- orz-host lib 全量：241 passed / 1 预存 flake
  （`call_tool_timeout_kills_process_tree`，全量一次运行失败、单独复跑
  通过；仓库既有时序 flake，B1 实施审计 §2 已登记，非本批引入）/ 4
  ignored。
- `cargo fmt --all --check` 干净。

## 待续

- B2 渲染折叠 / B3 契约与收尾 / B4 验证排期不变。
- B4 新增两项验证用例：进程重启后的黑板/LIF 续接 e2e、失败 run 后检索
  分区延续（activation 权威覆盖语义）e2e。

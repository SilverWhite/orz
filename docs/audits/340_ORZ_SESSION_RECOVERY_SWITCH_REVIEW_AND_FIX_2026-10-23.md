# 340 批：ORZ 会话恢复／切换审计与首轮修复（2026-10-23）

> **用户令**：检查 `C:\Users\1\OneDrive\Desktop\新建文件夹` 下 ORZ 显示的唯二两个活跃会话；随后要求落审计文档并开始修复。本档只审查并修改 **ORZ**，不涉及 Codex 会话。
>
> **数据纪律**：取证期间只读真实工作区；本批不手工改写该工作区 `.gsa`。兼容迁移只会在用户显式执行「恢复并继续」并成功取得会话 lease 后，由 ORZ 自身完成。

## §0 结论

UI 所称“两个活跃会话”并不真实：`51da03ea` 是当前持有 lease 的活跃会话；`c23e0aaa` 已于 `2026-10-23 17:25:33 +08:00` 被用户关闭，存在权威 `.closed` 标记且没有 lease。它仍出现在“活跃会话”，是因为会话清单 API 未投影关闭态，前端又把所有“未归档侧车”都归为活跃。

`c23e0aaa` 同时存在历史身份分裂：conversation 信封内是短 ID `c23e0aaa`，orientation 与 activation 两份伴随侧车内是同一完整 UUID `c23e0aaa-e752-4215-9a4f-0e435ff09b6a`。前端若传完整 UUID，host 的严格相等校验拒绝恢复；若传短 ID，又会把短 ID 继续固化。这是恢复失败的直接根因。

本批首轮修复：

1. `/api/conversations` 增加 `closed` 与 `legacy_short_id`，并仅在 orientation、activation **两份独立侧车一致**时恢复完整 UUID；
2. Explorer 新增“已关闭会话”组，关闭会话不再冒充活跃，但仍可只读打开并点「恢复并继续」；
3. `session/load` 增加受约束的 legacy 身份迁移：短 ID conversation + 两份伴随侧车均精确等于请求完整 UUID时才接受；取得 lease 后把 conversation 原地规范化为完整 UUID；冲突时 fail-closed；
4. 恢复成功继续沿用既有语义：清除 `.closed` 标记、切换当前绑定，刷新后回到活跃组；
5. 修复“只换前端绑定、不释放旧 host 会话”的假切换：先 load 新目标，成功后切换绑定，再以 `close(switch)` 释放旧会话／lease；load 失败则完整保留旧绑定，关闭旧会话失败也不回滚已成功的新绑定并给出可见告警。

## §1 真机证据

| 项 | `51da03ea` | `c23e0aaa` |
|---|---|---|
| conversation 内 `session_id` | 完整 UUID | **短 ID `c23e0aaa`** |
| orientation / activation | 完整 UUID，彼此一致 | 完整 UUID `c23e0aaa-e752-4215-9a4f-0e435ff09b6a`，彼此一致 |
| lease | `51da03ea...lease` 被 PID 28632 持有 | 无持有 lease；遗留 lease 文件可独占打开 |
| `.closed` | 无 | 有：`state=closed`、`reason=user_close` |
| 对话／工具配对 | 225 messages；121 call/result 成对 | 275 messages；103 call/result 成对 |
| run | 0–4：4 finished + 1 cancelled | 27 RUN + 3 ARC；24–26 finished；历史 RUN-1 无终态 |
| 审计判定 | 真活跃 | 已关闭、可恢复，不应算活跃 |

运行载体为 ORZ `0.9.9`；安装二进制哈希与 `D:\tb-eval\orz-windows` manifest 一致。当前前端已有 338 批的 run 数字序修复，故本批不重复修改回放排序。

## §2 根因链

```text
.gsa/conversations/<s8>.json 仍存在
  ├─ list_conversations 不读取 .gsa/sessions/<s8>.closed
  └─ widgets.js: 非归档 => 活跃
       └─ 已关闭 c23e0aaa 被错误显示为“活跃”

用户打开 c23e0aaa 并点“恢复并继续”
  └─ 清单若给完整 UUID
       └─ load_conversation_sidecar 要求 conversation.session_id 完全相等
            └─ short != full => SessionNotFound / 无法切换
```

“打开会话”本来就是只读浏览，“恢复并继续”才是切换绑定；这个安全分层保留不变。本批通过关闭态分组与明确文案，减少把只读浏览误认成已切换的机会。

另有一条切换生命周期缺口：旧实现 `session/load(target)` 成功后只调用前端 `setBinding(target)`，未对原绑定发 `session_close(reason=switch)`。因此界面虽已切到目标，旧会话仍留在 host map、继续持有 lease。新实现固定为 `load 新目标 → setBinding 新目标 → close(switch) 旧目标`；不能先关旧目标，否则新目标 load 失败会让用户两头落空。

## §3 修复设计与安全边界

### 3.1 状态投影

- 服务端以 `.gsa/sessions/<s8>.closed` 是否存在投影 `closed`；不从 journal 是否有终态猜会话生命周期。
- 归档仍优先于关闭态：存在有效归档包的会话进入“归档会话”，避免同一会话重复出现。
- “已关闭会话”保留 conversation 行与完整恢复入口；恢复成功后 host 清标记，下一次清单刷新回“活跃会话”。

### 3.2 身份迁移

允许迁移必须同时满足：

1. conversation 内 ID 恰为文件键 `session8`；
2. 请求 ID 比 `session8` 长且以前者开头；
3. orientation 内 ID 与请求完整 ID 完全相等；
4. activation 内 ID 与请求完整 ID 完全相等。

少一项或两份伴随侧车冲突均拒绝恢复，不用 8 位前缀碰撞做推断。迁移不在清单扫描时写盘；只有显式 `session/load` 取得 lease 后才重写 conversation 信封。

## §4 代码触点

| 触点 | 改动 |
|---|---|
| `orz-web/src/runs.rs` | 清单投影 `closed`；双伴随侧车一致时恢复 legacy 完整 ID；暴露 `legacy_short_id` |
| `assets/app/state.js` | 增 `closedCollapsed` 与 closed 选择组 |
| `assets/app/widgets.js` | 活跃／已关闭／归档三态互斥分组 |
| `assets/app/main.js` | 已关闭会话只读浏览文案；恢复成功后真切换并释放旧 host 附着 |
| `orz-host/src/acp_server.rs` | guarded legacy load + lease 后 conversation ID 规范化；冲突 fail-closed |
| Rust / JS 测试 | 正例、冲突负例、关闭态分组回归 |

## §5 验证

| 门 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test -p orz-web --offline` | **78 passed / 0 failed**；含关闭态、legacy 正例、伴随侧车冲突负例 |
| `cargo test -p orz-host session_load_ --offline` | **4 passed / 0 failed**；含迁移正／负例与既有 load 不扫兄弟、归档物化回归 |
| `node crates/orz-web/tests/frontend_smoke.mjs` | **全部通过**；新增“closed 不计活跃且保留恢复入口”与“load 后 close(switch) 旧附着”钉 |
| `git diff --check`（ORZ 子仓） | exit 0 |

首次 host 冷编译因未设置 `PROTOC` 失败（环境门，未进入源码编译）；按仓库既有工具 `PROTOC=D:\CLI\orz\bin\protoc.exe` 重跑后全部通过。该环境摩擦不计产品失败。

## §6 残余与发布边界

1. 本批没有直接改真实 `.gsa`；`c23e0aaa` 的信封规范化要在含本修复的载体启动后，由用户显式恢复触发。
2. **341 批更新**：本修复已随 ORZ `0.9.10` 完成 Windows／Linux 双平台重建，Windows 已换装、重启并在活体 API 命中新字段；详见 [`341 批档`](341_CARRIER_REBUILD_V0910_SESSION_RECOVERY_2026-10-23.md)。
3. `RUN-c23e0aaa-1` 历史无终态以及 ARC 是否计入“运行次数”是既有展示口径问题，不是本次恢复失败根因，本批不改写历史 journal。
4. 338 批已经修复回档后 mtime 压平导致的组内回放乱序；归档包不保存原 mtime 的更广泛残余仍沿 338 档登记，本批不扩面。


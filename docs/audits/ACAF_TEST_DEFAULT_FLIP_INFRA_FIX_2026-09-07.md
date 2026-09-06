# ACAF fail-closed 默认翻转——下游测试面基础设施修复审计（2026-09-07）

> 范围：P0-0m S1+S2 复审后用户指示处理的遗留观察 1——orz-host 全量
> 34 失败 + 1 挂死（ACAF signer 族，`reference` 观察项）与 0j S5-2 验证期
> 发现登记的 orz-bin `acaf_e2e` 失败（7 → 本批前实测 3 → 本批后 0）。
> 关联：Task C（2026-09-04，fail-closed 默认翻转下沉）/ TER T1.10（read_file
> 粗门 16K→64K）/ 0j（ad5f9ee 回归登记）。

## 1. 根因

orz-loop 的单元测试有 `#[cfg(test)]` 特例：`initial_acaf_fail_closed()` 在
orz-loop 自身测试构建下默认 shadow（ACAF 专项测试经
`.with_acaf_fail_closed(true)` 显式开启）。**`cfg(test)` 不跨 crate**——
下游 crate（orz-host / orz-bin）的测试编译中，orz-loop 按生产默认
`default_acaf_fail_closed()`（Task C 2026-09-04 起：env 未设 = enforce）
解析。凡依赖旧环境默认（未设 env = shadow）的下游测试面，自 Task C 起
静默运行在 fail-closed 下：

1. **orz-host 33 项**（acp_server 19 + codex_app 12 + lib.rs 2）：
   controller 无 signer 配置 → run-start fail-fast
   （`ACAF fail-closed is enabled but no signer client is configured`）→
   测试 unwrap panic。挂死 1 项（codex_app
   `streamed_chunks_concatenate_to_response_text`）同根因的另一种表现——
   turn 被拒启后测试 `recv` 循环无限等待永不到来的 `item/completed`
   （CPU 零增长死等；shadow 修复后该测试直接转绿，确认同源）。
2. **orz-host 1 项独立测试漂移**（`call_read_file_large_file_returns_handle_
   envelope`，同在 34 名单但非 ACAF 族）：TER T1.10（2026-09-04）粗门
   16K→64K 后，旧夹具 ~39 KiB 不再超限、返回全文而非句柄信封——被 ACAF
   失败簇掩盖至今。
3. **orz-bin `acaf_e2e` 3 项**（`*_shadow_*` 场景）：测试内 controller 构造
   11 处未显式声明模式，依赖 Task C 前的「env 未设 = shadow」默认 → 静默
   fail-closed（signer 不可达拒后 `Blocked` → `retrieval_close_record`
   不落、动作被阻），shadow 语义断言失败。0j 原登记 7 项中 4 项已被后续
   批次修复；0j 归因链（ad5f9ee 回归）适用于当时清单，现存 3 项实为同一
   「默认翻转 + 下游测试未显式声明」族——本批以journal 形态比对证实
   （拒后有 `control_ticket_rejected` 无 `retrieval_close_record` =
   fail-closed Blocked 形态，非 shadow Proceed 形态）。

## 2. 修复（生产构造器默认值零改动）

原则：**fail-closed 生产纪律不动**；测试面显式声明模式，对齐 orz-loop
单元测试约定（逻辑测试默认 shadow、ACAF 专项测试显式开启）。

1. `orz-host/src/acp_server.rs`：tests 模块新增
   `shadow_server()` / `shadow_server_with_gateway()` 两 helper
   （构造 + `.with_acaf_fail_closed(false)` + 约定注记），测试区 39 处
   构造点批量替换；既有显式 fail-closed 负测
   （`acaf_fail_closed_without_fabric_refuses_prompt`）保留
   `with_acaf_fail_closed(true)` 不变。
2. `orz-host/src/codex_app.rs`：测试 helper `server_with` 加
   `.with_acaf_fail_closed(false)`（覆盖 12 测试，挂死测试同批治愈）。
3. `orz-host/src/lib.rs`：两处测试 controller
   （`retrieval_lane_web_search_routes_to_host_and_fast_fails` /
   `orz_host_full_loophost_chain`）显式 shadow。
4. `orz-host/src/lib.rs` 大文件夹具：`x.repeat(200)→repeat(400)`
   （~39 KiB → ~78 KiB > 64K 新门），断言同步 `> 64 * 1024`，注记 T1.10
   对齐。
5. `orz-bin/tests/acaf_e2e.rs`：11 处 shadow 场景 controller 构造显式
   `.with_acaf_fail_closed(false)`（脚本按链扫描插入，显式
   `with_acaf_fail_closed(true)` 的 fail-closed 场景 7 处不动）。

## 3. 验证

- orz-host 全量（`--test-threads=1`，**无跳过**）：**249 passed /
  0 failed / 4 ignored，EXIT=0**——34 失败 + 1 挂死清零；
  `call_tool_timeout_kills_process_tree`（P2-13 B1 登记的既有 flaky）
  串行复跑通过、全量中亦通过。
- orz-bin `acaf_e2e`：**23 passed / 0 failed**（本批前 20/3）。
- 生产路径核验：orz acp 显式
  `with_acaf(acaf).with_acaf_fail_closed(acaf_fail_closed_enabled())`
  （main.rs CLI 路径）；orz-loop 生产默认 enforce 不变；ACAF fail-closed
  语义面（D-14/D-15/D-16）零改动。

## 4. 登记与边界

- 0j S5-2「验证期发现」（orz-bin acaf_e2e 7 项失败）→ 全部闭合，本审计
  为收口注记；原归因（ad5f9ee 回归）部分修正：现存 3 项实为默认翻转族，
  回归部分已被中间批次修复。
- P0-0m S1+S2 审计 §5 遗留观察 1 → 闭合。
- 观察遗留：orz-workspace / orz-tui 等其它下游 crate 无同族测试面
  （grep 核对仅 acaf_e2e 一处集成测试构造 controller）。

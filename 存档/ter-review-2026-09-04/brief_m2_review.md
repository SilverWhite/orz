# 工单：TER M2 W-F12/T2.2 与 T2.3 实现审查

你是 TER M2 实现审查代理。环境：Windows PowerShell，cwd=D:\CLI。

## 纪律
- 只审查【已提交】内容；禁止修改/删除/创建仓库文件（报告只可写入 `D:\CLI\.ter_review_2026-09-04\04_m2_wf12_t23.md`）；禁止 git 写操作；禁止触碰工作树未提交内容（并行工作）。提交版内容用 `git -C D:\CLI show <commit>:<path>` 提取（可输出到 $env:TEMP 再读）。
- 不运行 cargo；python 仅允许只读语法检查（`python -c "import ast; ast.parse(open(r'<path>', encoding='utf-8').read())"`）；PowerShell 只静态阅读不执行；禁止联网。
- 证据给出 commit、文件:行号、结果文件数值；无法验证写 NOT-VERIFIED。

## 审查范围
1) T2.2 W-F12 本地透明层（主仓库提交 73ee183=egress 基线探针、d705a06=dns_refusal.py、7cb0fb4=enforcement_probe wf12_timings_ms、91a390e=闭合登记+NoAppcontainer+grant /T）。先 `git -C D:\CLI\diff 73ee183^ 91a390e --stat` 总览，重点：
   - dns_refusal.py：loopback UDP :53、非 allowlist 即时 NXDOMAIN、allowlist+upstream fail-closed、不泄露 allowlist、自测 DNS_REFUSAL_SELFTEST_OK；实现正确性/健壮性（绑定、多请求、错误处理、回收/恢复路径）。
   - enforcement_probe.py：-ExpectAppcontainer、network_blocked/allowlist_reachable/metadata_blocked 记 wf12_timings_ms、外部≤1.5s 判定语义、结果 JSON 结构。
   - run_wf12_egress_probe.ps1：四类失败耗时表 rows+total_wall_ms。
   - scripts/s4_vm_wf12_probe.ps1 受控 op、scripts/s4_bridge_request.ps1 路由、sandbox grant /T、run_agent_arm.ps1 -NoAppcontainer。
   - 证据复核：`D:\CLI\_windows_high_nist\vm-wf12-probe-result.txt` 与 job-wf12-probe-*.txt；核对审计 `TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md` 声称值：allowlist_reachable PASS 15–47ms、network/metadata_blocked 0–16ms、egress 复测总成本 920ms≤10s、最差行 735ms≤2s、NXDOMAIN 7ms、测后恢复、AC 基线 FAIL 对照登记。
   - “no-appcontainer+allowlist”生产墙裁决在文档/脚本/README/审计四处是否一致；T2.2 是否借机超范围。
2) T2.3 部分实现：orz 子仓库提交 dd5b1dac（ORZ_FAKE_SCENARIO 驱动，改 orz-bin main.rs）。TODO2 T2.3 未勾选。审查：插入位置（ORZ_REAL 之后、ORZ_FAKE_TOOL 演示路径之前）与语义；JSON 格式 {text}|{tool_calls:[{name,arguments,call_id}]}、缺省 arguments={}/call_id=call-i-j；fail-closed（不可读/非法/无 text 或 tool_calls/空数组 → eprintln+exit 2）；env 只影响 fake 路径、生产误触风险；单测 fake_scenario_loader_round_trip_and_fail_closed 是否存在且合理；该驱动离 T2.3 验收（Windows 后台任务跨调用存活/idle-kill 实机证据）还差什么；TODO2/BACKLOG2/审计登记缺口。
3) T2.4：确认未开始；检查有无提前实现半成品（如 sync 清单已含新三件套）。
4) 交叉一致性：TODO2/BACKLOG2/audits/CLI_PROJECT_INDEX 中 TER 登记互相一致。

## 输出要求
最终回答中文：总评 → 发现清单（ID M2W-xx、P0/P1/P2/P3、对应 T2.x、位置、问题、证据、建议）→ T2.2/T2.3/T2.4 逐项符合性表（PASS/PARTIAL/FAIL/NOT-VERIFIED+证据）→ 实机证据数值复核小节 → 正面发现。
同时把完整报告写入 `D:\CLI\.ter_review_2026-09-04\04_m2_wf12_t23.md`（只限该目录）。

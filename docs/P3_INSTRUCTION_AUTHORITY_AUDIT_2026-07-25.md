# P3 instruction authority and capability audit — 2026-07-25

## 结论

P3 已形成一个 runtime-neutral 的 development/conformance 纵向切片：不可信内容不能通过措辞获得
权限，结构化动作必须通过独立内核 gate，子能力只能收缩，敏感或外部副作用动作还必须消费精确绑定的
一次性许可。

这不是“提示注入已被识别或消灭”的证明，也不是生产安全边界。它证明的是：在当前 fixture API 的
受控输入面上，内容、候选动作和执行权限已经分离，回执可由 HMAC 与独立语义投影复核。

## 实现面

- `instruction-provenance-receipt-v0.1.schema.json`
  - 八类来源：platform、user、trusted/untrusted project、external content、tool output、
    recalled memory、derived summary；
  - 未观测 trust 的项目自动降级；memory 必须带 origin conversation；
  - 原始内容不持久化，只有 SHA-256 与 byte count；
  - data-only 内容不能直接产生 action authorization。
- `capability-delegation-receipt-v0.1.schema.json`
  - child process/agent/remote MCP 请求必须是父 envelope 的子集；
  - 成功委派产生新的签名 child envelope，冻结相同 conversation/context，并且 TTL 不超过父级；
  - remote MCP 的 credential/filesystem/host/process/sandbox/secret 本地能力直接拒绝。
- `sensitive-action-permit-v0.1.schema.json`
  - permit 绑定 confirmation digest、action digest、target digest、impact digest、attempt 和 TTL；
  - 消费前复核 envelope、签名、绑定和时限；
  - 排他 consumption claim 使并发争用 fail closed，重复消费被拒绝。
- `action-authorization-receipt-v0.1.schema.json`
  - normal 动作只能由有效 envelope 授权；
  - sensitive/external side effect 同时要求有效 envelope 与 consumed one-shot permit；
  - 模型文本中的“已批准”声明被显式记录为 ignored；
  - verifier 重建来源路由、动作投影、许可绑定、结果、authority source 和 reason code。

## 测试结果

2026-07-25 本机回归：

| 套件 | 结果 |
|---|---:|
| prototype | 50 passed |
| Grok reference adapter | 44 passed |
| runtime | 6 passed |
| assurance（含 P3 7 项） | 26 passed |
| 合计 | 126 passed |

`python scripts/check_repository.py` 同时验证 69 个 schema，`error_count=0`；
`python -m compileall -q assurance prototype integration\grok runtime scripts` 通过。

P3 的 7 个测试采用少量种子加组合断言，覆盖：

1. project/web/tool/memory 四类 data-only 路径；
2. 恶意和普通措辞不改变授权语义；
3. capability 越界、receipt 篡改和 remote MCP 本地能力拒绝；
4. child capability 子集成功并产生实际签名 envelope；
5. permit 错绑、过期、重放与归档删除；
6. no-action、normal、untrusted、sensitive action 的内核授权分流。

没有为每个字段维护一整套正反 fixture 笛卡尔积。

## 与 Grok CLI 的关系

P3 实现位于 `assurance/`，不依赖 Grok CLI。Grok 的 44 项测试仅作为现有 reference adapter 回归，
其 PASS 不授予 P3 权限，也不把 Grok 提升为默认、强制或唯一 runtime。

## 尚未关闭

- API 目前接收已规范化结构化动作，不包含从任意模型文本生成动作的 production parser；
- 不接受 shell command string；真实文件路径、URL endpoint、redirect 和参数 canonicalization
  应由未来 tool/network broker 负责；
- 尚未证明所有真实 runtime、tool、MCP、web 和 memory 入口不可绕过 gate；
- workspace trust receipt 与文件变化失效尚未统一接入；
- 排他 claim 解决并发双消费，但进程崩溃后的 stale claim recovery 尚未实现；
- HMAC/DPAPI、key rotation/revocation、跨主机证明及生产威胁模型沿用 P1 未关闭项；
- scanner、redaction 和 anti-injection authority gate 是不同控制面，不能互相替代。

因此下一阶段应进入 P4 的 metadata-only audit projection/compaction/恢复边界，同时把 P3
adapter enforcement 作为并行生产化缺口保留，而不是继续增加提示词或大量正反样本。

# P2.5 guarded execution and process trace audit — 2026-07-24

## 结论

已完成一条不依赖 Grok、无模型、无网络的 runtime-neutral guarded execution 纵向切片。固定 action
在执行前同时验证：

- P1 active conversation namespace 与 HMAC security envelope；
- workspace canonical path、内容和 disposable policy digest；
- P2 observed-compliant Docker observation 与同 conversation selection receipt；
- 固定 Docker profile、镜像、capability 与 action template。

真实执行结果为 `completed_verified`，独立重开 Windows DPAPI installation key 后验证通过。

## 最终真实运行

- execution：
  `GEX-B180FFB0B7C54270AC4E128BA5FFFAF5`
- conversation：
  execution receipt 中绑定的单一 active namespace
- container：
  `f208e375c58b7c1dbb9f2e1709c80fa29d376583682a323fea32d985c9ed63f8`
- 运行中 container host PID：`3458`
- container process snapshot：PID `3458`，PPID `3435`，command `python`
- terminal exit code：`0`
- receipt integrity：RFC 8785 + HMAC-SHA256
- independent verification：`valid=true`、`errors=[]`

固定 action 只在 disposable workspace 创建已知内容，核对 SHA-256 后立即删除；不调用模型、
不请求网络、不暴露任意 shell/CLI 参数。

## Docker CLI 宿主进程追踪

| Sequence | Role | PID | Exit |
|---:|---|---:|---:|
| 0 | create | 9412 | 0 |
| 1 | start | 10264 | 0 |
| 2 | inspect_running | 13284 | 0 |
| 3 | top | 25900 | 0 |
| 4 | wait | 17800 | 0 |
| 5 | logs | 24336 | 0 |
| 6 | inspect_terminal | 4380 | 0 |
| 7 | remove | 25232 | 0 |
| 8 | inspect_absent | 8508 | 1（预期：对象不存在） |

每项同时记录参数 RFC 8785 摘要、开始/结束时间、duration、stdout/stderr byte count 与摘要。
原始 Docker inspect/log 输出不进入 execution receipt。

## 残留复核

执行后独立检查：

- 9 个 Docker CLI PID 加 container host PID，共 10 个 PID，剩余 `0`；
- 对固定 container ID 执行 inspect 返回 `no such object`；
- 按 execution label 查询 `docker ps -a` 返回空；
- workspace 中只剩原 disposable marker；
- execution probe 已删除；
- receipt 被写入当前 conversation 的 `temporary_lifecycle_receipt`，归档单元测试验证删除。

## 失败与篡改测试

精简测试覆盖：

- 成功链和完整 process sequence；
- selection conversation 错配在启动任何 Docker 进程前 fail closed；
- process argument digest 篡改使 HMAC/verifier 失败；
- container log 损坏时仍执行 container cleanup 和 workspace probe cleanup；
- receipt 归档生命周期清理。

未为每种字段组合新增静态 fixture；组合篡改由程序化测试完成。

## DPAPI 上下文观察

第一次在受限工具沙箱内复核外部上下文创建的 DPAPI blob 时，Windows 返回
`NTE_BAD_KEY_STATE`，验证正确失败。随后在与原执行相同的 Windows 用户上下文中重开密钥，
独立验证通过。这说明 DPAPI blob 不能被当作跨执行身份可移植密钥；后续 runner 必须明确保持
installation-key 创建和验证的用户/机器上下文。

## 限制

- 仅执行一个固定 no-model action，不是通用 tool broker。
- 进程追踪证明本次观察与事后无已知残留，不证明 PID 永不复用或内核不存在未观测副本。
- Docker daemon、Docker Desktop/WSL2、host kernel、pagefile 和物理存储残留仍在证明边界外。
- Windows native strict backend、并发/crash recovery、磁盘 quota 与 endpoint-bound 联网尚未关闭。

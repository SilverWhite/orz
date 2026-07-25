# P2 Docker strict sandbox audit — 2026-07-24

## 结论

P2 已完成一个不依赖 Grok 的 runtime-neutral Docker strict sandbox 纵向切片：

- 固定镜像摘要和离线 `pull=never`；
- disposable workspace、唯一 host bind、受限 tmpfs；
- 非 root、只读 rootfs、network none、drop all capabilities、no-new-privileges；
- CPU、内存、PID 与 wall-time 上限；
- 容器内负向行为探针、容器配置投影、独立 verifier；
- observed-compliant backend 才允许选择，指定后端不合规则 fail closed。

本机 Docker backend 的一次 observation 为 `compliant`。Windows native strict backend 仍为
`noncompliant`，因为现有 Job Object 证据不包含 restricted token/AppContainer、文件系统/注册表或
完整网络边界。因此 P2 的 Docker 分支已形成 development/conformance 闭环，P2 的无 Docker 路径尚未关闭。

## 实测环境与固定输入

- Docker Client/Engine：`29.2.1`
- Server：Linux `amd64`，Docker Desktop/WSL2
- security options：cgroup namespace、builtin seccomp
- image：
  `python@sha256:4b0a8ebf16cf4563f3d3732bd4f4a464abb2f671b3b9d00aab281d705d224457`
- profile SHA-256：
  `9447b13c631f75542f29ec826381d718302d86302342b0fd67847cc6dbb62638`

探针只挂载仓库 `.observed-runs/` 下带精确授权 marker 的空目录。该目录和结果默认由
`.gitignore` 排除，不进入 canonical fixture。

## 观测结果

以下八项全部为真：

1. 容器用户不是 root；
2. root filesystem 写入被拒绝；
3. 显式 workspace 写入成功；
4. `/tmp` tmpfs 写入成功；
5. 对 `1.1.1.1:443` 的连接被阻断；
6. Docker socket 不存在；
7. `/host-home` 不存在；
8. workspace 探针文件已清理。

Docker inspect 同时确认：

- `user=65532:65532`
- `ReadonlyRootfs=true`
- `NetworkMode=none`
- `CapDrop=[ALL]`、`CapAdd=[]`
- `no-new-privileges=true`
- memory `268435456` bytes、CPU `500000000` nano-CPUs、PID `64`
- 唯一 host mount 为 `/workspace`
- `/tmp` 为 `rw,noexec,nosuid,nodev,size=67108864`
- probe exit code 为 `0`
- 临时容器已删除

自动选型回执选择了 Docker observed-compliant candidate；Windows native candidate 被保留为
observed-noncompliant，未发生 fallback。

## 正反例数量与过限制审计

当前不需要为每个字段组合维护大量正反 fixture。P2 采用三层策略：

1. schema 只固定真正的安全不变量；
2. 每个硬不变量至少保留一个成功路径和一个拒绝/篡改路径；
3. 组合与字段篡改由程序化单元测试和独立 verifier 覆盖。

为避免把当前实现误当成永久产品边界，以下集合已改为开放扩展：

- assurance `profile_id`
- archive `delete_on_archive` 与 `never_persist` 的新增类别
- sandbox backend kind

以下门禁仍刻意保持封闭：

- `persist_after_archive` 只能是最小三类终结数据；
- backend 必须同时为 available、compliant、observed 才能 allow；
- 指定 backend 不得被其他 backend 暗中替代；
- Docker 镜像摘要、mount、身份、网络、capability 和资源控制必须与 profile 一致。

## 限制

- 这是一次本机、固定镜像、network-off 的 development observation，不是普遍安全证明。
- Docker daemon、Docker Desktop/WSL2、Linux kernel 与容器逃逸不在本 profile 的证明范围。
- 尚未接入真实模型、tool broker、credential broker、conversation lifecycle 或真实用户项目。
- 未证明无 Docker 的普通 Windows 环境具有 strict backend。
- 未验证 endpoint-bound 临时联网、磁盘 quota、并发与 crash recovery。

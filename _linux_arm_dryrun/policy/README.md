# 严格策略层（Linux 原生强制，无 shim）

## 三臂

- **control**：root、全 egress、开放 FS——baseline，探针可达性 + verifier 断言成立。
- **non-root**：全 egress、开放 FS，agent 以普通用户运行（无 sudo）。
- **high-nist**：per-task egress allowlist + 只读 OS + 冻结 home +
  non-admin + no_new_privs + 能力剥离。

## 实现载体

每臂一个任务环境模板（`tasks/<arm>/`）：

- `environment/Dockerfile`：arm64v8 基础镜像 + 摩擦探针数据。
- `environment/docker-compose.yaml`（可选，Harbor 任务级 compose 覆盖）：
  - non-root：`[agent] user = "agent"`（task.toml 字段，不依赖 compose）。
  - high-nist：`security_opt: ["no-new-privileges:true"]`、
    `cap_drop: [ALL]`、`read_only: true` + 工作区 tmpfs。
- `environment/apply_hardening.sh`：容器启动后、agent 运行前执行
  （root 阶段）：创建受限用户、锁定 root 密码、remount 只读、写 enforcement
  探针断言文件。

## enforcement-probe（先验墙，无 agent）

`policy/enforcement_probe.sh`：独立探针，断言每臂墙真实生效：

- root 写不进只读 OS 路径（EROFS）；
- 普通用户写不进系统路径（EPERM）；
- 工作区可写（tmpfs）；
- no_new_privs 生效（`grep NoNewPrivs /proc/self/status` 为 1）；
- 非白名单主机连接被拒；
- DeepSeek 模型端点可达（`api.deepseek.com`）。

每臂启动前跑一遍；任一断言失败 = 该臂不作数（fail-closed）。

## 已知约束（2026-09-01 登记）

**Harbor 原生 egress-control sidecar 在本机不可用**：Docker Desktop VM 内核
`/proc/config.gz` 含 `CONFIG_NFT_FIB=m` / `CONFIG_NFT_FIB_IPV4=m`，但 Harbor
内核探针匹配的 `CONFIG_NFT_FIB_INET=[ym]` 符号缺失，且 `CONFIG_NFT_FIB_IPV6`
未设置 → `_egress_control_kernel_support()` 返回 False → Windows 宿主下
`_enable_egress_control=False`，allowlist sidecar 不注入。

影响：high-nist 臂的 restrict-egress 不能依赖 Harbor 原生 allowlist 落
（容器层实际不强制）。干跑采用**容器内 DNS/hosts 层替代**（/etc/hosts 只钉
白名单 IP + /etc/resolv.conf 指向本地 stub），并作为方法学缺口登记：
正式 Windows 主载以 host firewall/WFP 兜底（呼应设计 §3 的 AppContainer
原始 TCP 残余必须由 host firewall 兜底）。orz 侧 fail-closed 语义不变
（非 PUBLIC 任务不带 --allow-network）。

**2026-09-01 干跑口径调整**：egress 探针的 high-nist 臂预期改为
`reachable`（记录基线而非假装墙存在）——restrict-egress 是 Windows 主载的
核心摩擦轴，Linux 干跑不伪造不可用的强制；对应任务 task.toml 已标注。

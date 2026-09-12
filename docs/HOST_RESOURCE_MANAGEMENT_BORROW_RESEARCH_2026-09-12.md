# 宿主资源管理与调度：成熟产品借镜调研（2026-09-12）

> 登记：BACKLOG **0z**；设计入口 [`HOST_RESOURCE_SAFETY_DESIGN_2026-09-12`](HOST_RESOURCE_SAFETY_DESIGN_2026-09-12.md) §3。
> 状态：`reference`（借镜调研，不拥有生产设计）。
> 检索方式：2026-09-12 一手来源抓取（官方文档 / 官方源码仓库 man 页），逐页读原文并按需摘录；
> 缓存原文留在工作机临时目录（不入库）。**未取得的来源**单列于 §5，不得据其下结论。
> 边界：本文件只回答"成熟产品怎么处理资源与调度、哪些可借、哪些不借"，
> 不定义 orz 的参数与机制（那是设计文档的职责）。

## 1. 五条主流路线（附一手来源）

### 1.1 内核/容器配额：把边界外包给内核

- cgroup v2 提供**两级内存边界**：`memory.high` 是"内存用量节流限"（越过即被限流并产生回收压力），
  `memory.max` 是硬限（越界触发 OOM kill）；内核文档明确写
  "**`memory.high` is the main mechanism to control memory usage**"，并允许
  "over-committing on high limit + 全局压力分摊"的用法。`memory.events` 暴露
  low/high/max/oom 等计数，`memory.oom.group` 决定 OOM 时是否把整组当不可分割单元杀掉。
  来源：Linux kernel docs，*Control Group v2*（来源 1）。
- 容器层把这些开关产品化：Docker `--memory=` = "容器可用的最大内存"（默认**无任何资源约束**），
  文档同时提示"内存耗尽的风险"（可能被迫杀宿主进程）。来源：Docker docs，*Resource constraints*（来源 11）。
- Windows 对应物是 **Job Object**：文档定义其为"把一组进程当作一个单元管理"的内核对象；
  `CreateProcess` 创建的子进程**默认自动加入**父进程所属 job；
  置 `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` 后"**关闭最后一个 job 句柄即终止所有关联进程**"；
  `JOB_OBJECT_LIMIT_JOB_MEMORY` 则是"该 job 可提交的虚拟内存上限"。
  来源：Microsoft Learn，*Job Objects* / *JOBOBJECT_EXTENDED_LIMIT_INFORMATION*（来源 8、9）。
- Job Object 还能**硬限 CPU 速率**：`JOBOBJECT_CPU_RATE_CONTROL_INFORMATION` 的
  `JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` 语义是"达到本调度区间的 CPU 上限后，
  该 job 关联线程在本区间内不再运行"。来源：Microsoft Learn，*JOBOBJECT_CPU_RATE_CONTROL_INFORMATION*（来源 13）。

### 1.2 压力信号驱动：用"卡住多久"而不是"用了多少"触发

- PSI（Pressure Stall Information）在 `/proc/pressure/{cpu,memory,io}` 暴露
  `some avg10/avg60/avg300 total` 与 `full ...` 两行：`some` = 有任务在该资源上停顿的时间占比，
  `full` = **所有非 idle 任务同时停顿**的时间占比。来源：Linux kernel docs，*PSI*（来源 2）。
- systemd-oomd 就是 PSI 的消费者：用 cgroup v2 + PSI "在内核 OOM 之前"采取纠正动作，
  周期轮询被监控 cgroup 的 PSI，超限则选中一个 cgroup 并对其**全部进程 SIGKILL**
  （只有后代 cgroup 是候选）。来源：systemd 官方 man，*systemd-oomd.service*（来源 7）。
- 响应谱固定为 **throttle → reclaim → kill**：先限流（`memory.high`）、再回收、最后才杀。

### 1.3 准入控制与排队：重活不并发抢同一资源

- Kubernetes 用 **QoS 分级 + 驱逐顺序**做优先级：节点资源不足时先驱逐 BestEffort、
  再 Burstable、最后 Guaranteed；并且"**只有超出 requests 的 Pod 才是驱逐候选**"。
  来源：K8s docs，*Pod Quality of Service Classes*（来源 4）。
- 驱逐由**阈值信号**驱动，K8s 默认硬阈值：
  `memory.available<100Mi`（Linux）/ `<500Mi`（Windows）、`nodefs.available<10%`、
  `imagefs.available<15%`、`nodefs.inodesFree<5%`（Linux）。来源：K8s docs，*Node-pressure Eviction*（来源 3）。
- 存储侧同样是 request/limit 契约：`ephemeral-storage` 以 Bytes 计 requests/limits，
  超限同样触发驱逐。来源：K8s docs，*Resource Management for Pods and Containers*（来源 5）。
- 构建系统的对应物是**并发度可配 + 目标目录锁**：cargo 的 `CARGO_BUILD_JOBS` = "并行 job 数"，
  `CARGO_TARGET_DIR` 决定产物落点（同 target 并发会被锁串行化）。
  来源：Rust 官方文档，*Cargo Environment Variables*（来源 12）。

### 1.4 写面保留与降级：满不是死，是降级

- journald 对自身日志面同时设**用量上限与保留量**：`SystemMaxUse=`/`RuntimeMaxUse=` 限制日志
  最多占多少盘，`SystemKeepFree=`/`RuntimeKeepFree=` 要求**给其他用途留出多少盘**；
  两组默认分别 **10% / 15%**（各自以 4 GiB 封顶）。它还有一条关键兜底：
  "**如果文件系统接近写满、启动时 KeepFree 已被违反，上限会被提高到实际可用的百分比**"——
  即宁可少留、也要能继续运行。速率侧用 `RateLimitIntervalSec=`/`RateLimitBurst=`，
  超限消息被**丢弃**而不是让进程失败。来源：systemd 官方 man 源，*journald.conf*（来源 6）。
- 这套"上限 + 保留 + 限速丢弃"正是日志面在 ENOSPC 下的工业解法：**先保命、再保量**。
- **删除安全**（与回收直接相关）：Windows 的 `SHFileOperation` **默认永久删除**文件，
  只有置 `FOF_ALLOWUNDO` 才"送到回收站"；反过来若要保证不进回收站则用 `DeleteFile`。
  即"进回收站"是**显式选择的语义**，永久删除是默认语义。
  来源：Microsoft Learn，*SHFileOperationA*（来源 14）。
- **XDG Trash 规范（v1.0，2014-01-02；2026-09-12 经 orz 真机检索轮的浏览器车道渲染全文取得）**
  两条对本设计关键的事实：①**目录位置隐含同文件系统**——`$topdir` 的定义即"文件系统挂载点"，
  投废动作规定为 "**it MUST be moved into** `$trash/files/`"（同设备即 rename）；跨设备投废只是 MAY，
  且被注明 "costly file copying"。②**规范不设容量上限、不规定保留期或自动过期删除**，唯一相关条款
  是非强制的实现注记："Automatic trash cleaning may, and probably eventually should, be implemented.
  But the implementation should be somehow known to the user."；v1.0 的 `$trash/directorysizes` 只是
  统计总大小的缓存、非配额。元数据约定：`$trash/info/<原名>.trashinfo`、首行 `[Trash Info]`、
  键仅 `Path` 与 `DeletionDate`、`Path` 不得含 `..`、值做 URL 风格百分号编码。来源：来源 16。
- **Bazel 磁盘缓存是"两级"事实**（官方 CLI 参考全文取得）：`--disk_cache` **只有路径语义，没有容量
  或 GC 参数**；容量/年龄回收由三个实验开关承担——`--experimental_disk_cache_gc_max_size`（默认
  **0**）、`--experimental_disk_cache_gc_max_age`（默认 **0**）、`--experimental_disk_cache_gc_idle_delay`
  （默认 **5m**），自 Bazel 7.4 起，**默认值 0 = 关闭**，必须显式设正值才生效（空闲后后台 GC，
  另有按需 GC 工具）。来源：来源 15/17。

### 1.5 Windows 页面文件：commit 上限与磁盘余量绑定

- 页面文件的作用是"**扩展系统 commit 上限（虚拟内存）**"；系统管理模式下，当 commit 计费
  达到 commit limit 的 90% 时，页面文件会向 **3× 物理内存或 4 GB（取大者，且不超过卷的 1/8）**
  增长——**前提是磁盘还有可用空间**。来源：Microsoft Learn，*Introduction to the page file*（来源 10）。
- 推论（与本设计直接相关）：**盘满与 commit 耗尽不是两件事**——页面文件无法增长时，
  分配失败会立刻表现为 `os error 1455`（页面文件太小）与 Rust alloc abort。

## 2. 编码类 agent 产品的实际做法

本次检索未取得可直接引用的 agent 产品资源策略一手文档（见 §5），因此**不作产品级断言**。
可确证的只有基础设施层事实：主流做法把资源边界交给**沙箱/内核层**（cgroup、Job Object、
容器配额），agent loop 自身不做资源调度。orz 真机模式没有这层，故必须在框架内重建等价物。

## 3. 借镜映射（→ 0z 设计子项）

| 借镜点 | 来源路线 | 落到 0z |
|---|---|---|
| 两级边界（throttle / hard） | cgroup v2 `memory.high`/`memory.max` | §4.1 三档响应：soft = 拒新重活（throttle）、hard = 回收/树杀 + 收尾 |
| 压力信号优先于裸绝对值 | PSI / systemd-oomd | §4.1 读数同时看余量与在跑重活；hard 档先回收再杀 |
| 分级 + 只处置超限者 | K8s QoS / eviction | §4.1 动作分档只卡重活，轻活不误伤 |
| 阈值双口径（绝对值 / 百分比） | K8s 默认阈值 | §4.1 卷用绝对 GiB、commit 用百分比 |
| 上限 + 保留量 + 超限丢弃 | journald | §4.3 reserve + Degraded 骨架模式（丢事件、不丢链骨架） |
| 生命周期归内核 | Job Object `KILL_ON_JOB_CLOSE`、`JOB_OBJECT_LIMIT_JOB_MEMORY` | §4.2 内层 Job（含 commit 上限可选） |
| 回收优先于杀 | cgroup reclaim / oomd | §4.6 回收机制（可再生白名单 + 审计先写 + 有界冷却） |
| 并发度即资源杠杆 | cargo `CARGO_BUILD_JOBS`、K8s requests | §4.7 并发限流（commit 轴的机械注入） |
| commit 与盘同源 | Windows 页面文件 | §4.1/§4.3 把盘余量与 commit 余量当同一约束的两面 |
| CPU 亦可硬限 | Job Object CPU rate hard cap | §4.7 上限制第三维（CPU 80%） |
| 删除语义显式化 | `SHFileOperation` + `FOF_ALLOWUNDO` / `DeleteFile` | §4.6 三分类动作（`cache` 延迟删除 / `unknown` 拒绝 / 证据面拒绝） |
| 回收优先于杀（**最终不采纳回收站**） | cgroup reclaim / 回收站约定 | §4.6.1 终裁：**回收站整体取消**（同卷不释放空间），改以"轮数窗口延迟删除 + git 兜底"承接 |
| **回收站规范自身无容量/过期语义**（一手实证） | XDG Trash v1.0（来源 16） | §4.6.1：**不做容量溢出**的独立依据——连规范都不定义该语义，溢出策略属自造复杂度 |
| **缓存上限默认关闭、须显式开启** | Bazel `--experimental_disk_cache_gc_*`（默认 0，来源 17） | §4.8 表 4：soft 档柔性降级**默认关 + 可配逃生阀**的同类取舍 |

## 4. 明确不借镜

- **不引入容器/cgroup 化前提**：真机直跑是产品形态（用户裁决）。
- **不做全局调度器**：不排队宿主其他进程、不与相邻 agent 协调、不抢占。
- **不自动改道**：不把重活搬到别的卷/临时盘（0z §2 硬约束）。
- **不做无差别 OOM 杀**：systemd-oomd 式"整 cgroup SIGKILL"会毁掉审计收尾；
  只允许"树杀 + 显式终止形态"。
- **不照搬 K8s 默认数值**：其阈值面向节点级多租户，本机是单租户真机，取值另裁（0z §11）。
- **不采用系统回收站**（2026-09-12 三轮裁决，见 0z §4.6.1）：官方 API 已证"同卷回收站不释放
  空间"，它既不解决空间、其可恢复价值又被"轮数窗口 + git 兜底"覆盖；仅作为回查记录保留。

## 5. 未取得与限制

- ~~Bazel 磁盘缓存 / XDG trash 规范~~：**均已取得**（§1.4 两条 + 来源 15/16/17）。其中 XDG 项由
  **orz 真机检索轮**（2026-09-12，`RUN-CLI-6aa530bb`，浏览器车道渲染全文 + 原生镜像复核）取得：
  无 JS 的原始抓取会被 **Anubis 反爬**挡住（拿到的是挑战页），且原指定的
  `trash-spec/trashspec-latest.html` 现为 **404**，现行地址是
  `https://specifications.freedesktop.org/trash/latest/`（规范正文内部的 Location 仍写着旧地址）。
- **GitHub Hosted Runner 磁盘/清理策略**：目标页未含预期措辞（页面已改版），未取得可引用事实。
- **agent 产品官方资源策略文档**：未检索到一手声明；§2 只保留基础设施层事实。

## 6. 来源清单（2026-09-12 抓取）

| # | 来源 | 用途 |
|---|---|---|
| 1 | https://docs.kernel.org/admin-guide/cgroup-v2.html | cgroup v2 内存两级边界与事件计数 |
| 2 | https://docs.kernel.org/accounting/psi.html | PSI some/full 语义 |
| 3 | https://kubernetes.io/docs/concepts/scheduling-eviction/node-pressure-eviction/ | 默认驱逐阈值 |
| 4 | https://kubernetes.io/docs/concepts/workloads/pods/pod-qos/ | QoS 分级与驱逐顺序 |
| 5 | https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/ | ephemeral-storage 契约 |
| 6 | https://raw.githubusercontent.com/systemd/systemd/main/man/journald.conf.xml | 日志面上限/保留/限速丢弃 |
| 7 | https://raw.githubusercontent.com/systemd/systemd/main/man/systemd-oomd.service.xml | PSI 驱动的用户态 OOM |
| 8 | https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects | Job Object 语义与 KILL_ON_JOB_CLOSE |
| 9 | https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information | JOB_OBJECT_LIMIT_JOB_MEMORY（commit 上限） |
| 10 | https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/introduction-to-the-page-file | 页面文件与 commit limit/磁盘余量 |
| 11 | https://docs.docker.com/engine/containers/resource_constraints/ | 容器资源约束默认值与风险提示 |
| 12 | https://doc.rust-lang.org/cargo/reference/environment-variables.html | CARGO_BUILD_JOBS / CARGO_TARGET_DIR |
| 13 | https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_cpu_rate_control_information | Job CPU 速率硬限语义 |
| 14 | https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shfileoperationa | 永久删除为默认、`FOF_ALLOWUNDO` 才进回收站 |
| 15 | https://bazel.build/remote/caching | Bazel 磁盘缓存机制（`--disk_cache`，目录即缓存面） |
| 16 | https://specifications.freedesktop.org/trash/latest/ | XDG Trash v1.0 全文（浏览器车道渲染；无容量/过期语义、topdir 同设备语义、`.trashinfo` 约定） |
| 17 | https://bazel.build/reference/command-line-reference | `--disk_cache` 无容量/GC 参数；GC 三开关（默认 0/0/5m，自 7.4） |

# Agent OS 先验调研：seL4 / Plan 9 / Erlang / Fuchsia / WASI / 持久对象系统

> 日期：2026-10-27
> 目的：为 orz 从 harness 转 OS 找可借的**不变量**；不抄代码，不做源码级复刻。
> 方法：官方文档 / 设计综述优先；一手论文缺失处明确标注“工程综合”。
> 结论先行：我们要的是**微内核风格的 agent runtime**，不是内核、不是 hypervisor、不是 VM。

## §0 给 orz OS 的 10 条不变量

1. **小核只给机制，语义在工具/用户态**（seL4、Plan 9、OTP 的共同骨架）。
2. **命名即权限**：句柄/能力/私有命名空间优先于“路径字符串 + 事后 ACL”。
3. **同步会合 + 有界消息**：内核不存储转发；异步/突发由用户态队列与通知吸收。
4. **回复通道是一次性能力**：请求/回复精确绑定，第三方不可伪造。
5. **每会话私有命名空间**：同一份资源，不同视图；组合发生在命名空间层。
6. **任务隔离 + 监督树**：故障处理是结构（谁重启谁），不是业务代码里的 try/catch。
7. **轻量执行体 + 私有内存 + 时间片/reduction**：隔离与预算由运行时给，不靠模型自觉。
8. **能力路由**：静态声明 + 运行时校验；父即沙箱边界；环检测；弱依赖可降级。
9. **描述符即能力**：preopen 注入初始句柄；没有全局 `open`；路径只在工具边界做受控解析。
10. **持久性作为系统属性**：OID + 检查点/事务；崩溃回到最近完整检查点；不持久化 native 栈/JIT/OS 句柄。

## §1 逐系统

### 1.1 seL4：同步会合 + 能力传递

- IPC 载体是 **endpoint**；队列里排的是**线程**，不是消息——同步 rendezvous，内核不存储转发。
- `Send` 阻塞至消费；`NBSend` 仅在接收方已等待时成功；`Call` = Send + Recv，但接收阶段阻塞在**一次性 reply capability** 上（存于接收方 TCB），`Reply` 精确回给正确客户端。
- 消息走**有界消息寄存器**（MR）；小消息免拷贝，超出部分有界拷贝；无无界缓冲，最坏执行时间可分析。
- IPC 同时是**能力委派通道**（cap transfer）；badge 让一枚 endpoint 能力复用为多个逻辑服务入口。
- 借：同步 syscall/IPC、有界消息、reply cap、能力即命名；不借：完整微内核/驱动/形式化验证负担。

### 1.2 Plan 9：私有命名空间 + 单一协议

- 三条原则：资源像文件一样命名；标准协议 **9P**；不同服务的层次结构汇合进**单一的私有命名空间**。
- 每进程命名空间被推广到进程、图形、网络；局部性靠**全局约定**维持，不靠内核检查。
- 9P 是请求-响应事务；命名与实现解耦（本地/远程对客户端不可见）；组合＝挂载，不是 API 胶水。
- 借：每会话命名空间、统一文件式工具面、9P 式请求/响应、私有视图；不借：只靠约定不靠强制、所有东西都必须是文件。

### 1.3 Erlang/OTP + BEAM：隔离、监督、调度

- OTP：worker/supervisor 二分；监督树层次化编排；重启策略是结构；behaviour 收敛相似模式。
- BEAM 进程 = 栈 + 堆 + mailbox + PCB；进程间**无共享可变内存**，只能拷贝传消息。
- 每进程**拷贝式分代 GC**：minor 看栈顶新数据，老数据进 old heap，fullsweep 才回收；停顿局部化。
- 调度：就绪/等待双队列；时间片以 **reduction** 计量；用尽抢占回队尾；每核一调度器，可接管别的 run queue。
- 借：隔离、监督/重启、每会话上下文与 GC、reduction 预算、消息传递；不借：无界 mailbox/选择性接收、完整 BEAM。

### 1.4 Fuchsia：能力路由 + 组件沙箱

- 能力 = 「访问权 + rights」；组件只能通过命名空间里可发现的能力/少数编号句柄交互。
- 两层：manifest 静态声明（`offer`/`expose`/`use`/`capabilities`）+ 运行时有效路由校验；**父组件即子组件的沙箱边界**。
- 七类可路由能力：`protocol`（打开 FIDL channel 的目录项）、`service`、`directory`、`storage`、`dictionary`、`resolver`、`runner`；`resolver`/`runner` 只路由到 environment，组件实例运行时不可见。
- 环检测 + `weak` 依赖 + `availability`；能力获取（拿到目录项）与能力使用（打开 channel）解耦。
- 借：能力路由、命名空间、父即沙箱、类型化能力、环/弱依赖；不借：完整 component framework、Zircon 内核。

### 1.5 WASI：描述符即能力

- 没有全局 `open`：所有文件函数要求 handle；只有 descriptor 上的 `open-at`，路径相对该目录树解析。
- 沙箱靠**禁止绝对路径/`..`/越界符号链接**；初始 handle 集由 **preopens** 注入。
- 血统：CloudABI ← Capsicum；绝对路径兼容在 libc 层用 libpreopen 模拟，WASI 层不出现。
- 重要修正：ambient authority 不是“有/无”，而是**程度与粒度**；Wasm 组件的 import 是链接期能力，不同实例可链到不同实现/衰减/拒绝版本。
- “ghosts”：资源用整数/字符串标识、由隐含继电器解析（路径是典型）；应优先 handle/resource。
- 借：handle、preopen、无全局 open、路径只在边界受控解析、链接期权威；不借：纯度导致工具链不兼容、完全禁用绝对路径。

### 1.6 Smalltalk / KeyKOS / Oberon：持久对象与检查点

> 证据状态：本节一手论文未取到（GemStone/S、KeyKOS/EROS 技术报告、Oberon GC 报告均缺），
> 以下机制细节含**工程综合**，不是文献结论。

- Smalltalk image = 对象内存 + 恢复执行所需的 VM 状态，是**检查点**，不是普通序列化；不能当跨 VM 的交换格式。
- 最小可行：停世界全量快照 + 稳定 OID；根集覆盖 specialObjects/class table/symbol table/processList/finalization/外部资源；弱引用与 ephemeron 单独处理；对象头存 OID/classOid/format/size/flags/hash；盘上指针存 OID，加载时 swizzle；`临时文件 → fsync → 原子 rename` + magic/版本/端序/校验和。
- **绝不持久化**：native 指针、JIT 机器码、C 栈、OS 句柄（fd/socket/FFI/timer）；恢复时按策略重建或作废。
- KeyKOS：持久化、纯能力制 OS；对象按 capability 寻址；内核对象、进程状态、能力图同处一个存储——正交持久性。一致性单位是**整个检查点**，不是任意对象事务（检查点一致性，不是 ACID）。
- 借：OID、检查点、原子 rename、持久根、能力图持久化；不借：完整 image（native 状态）、全系统正交持久化的复杂度。

### 1.7 Unix（作为对照）

- process/file/pipe/signal + “一切皆文件” + 小工具组合；file descriptor 是能力，但进程有 ambient authority（uid/gid/全局命名空间）。
- 借：进程模型、fd、pipe、signal、组合小工具；不借：全局命名空间、环境权威、路径 + ACL。

## §2 对照表

| 维度 | seL4 | Plan 9 | Erlang/BEAM | Fuchsia | WASI | Smalltalk/KeyKOS |
|---|---|---|---|---|---|---|
| 核心机制 | endpoint 同步 IPC + 能力 | 9P + 私有命名空间 | 进程 + 监督树 + 调度 | 能力路由 + 组件 | 描述符 + preopen | 持久对象 + 检查点 |
| 命名即权限 | 是（cap） | 是（namespace） | 弱（PID） | 是（cap + rights） | 是（handle） | 是（cap/OID） |
| 隔离基元 | CSpace/TCB | 进程命名空间 | 每进程堆栈 | 组件实例 | Wasm 线性内存 | 对象存储 |
| 组合方式 | 能力派生 | 挂载 | 监督树 | offer/expose/use | handle 传参 | 对象图 |
| 故障边界 | fault handling | 重挂载/换服务 | supervisor 重启 | 组件/路由 | 实例退出 | 检查点回滚 |
| 对 agent OS 的启发 | syscall/IPC 形状 | 会话命名空间 | 隔离/监督/预算 | 能力路由 | 工具边界 | 状态持久化 |

## §3 映射到 orz OS

- **对象模型**：Fuchsia 的“能力 = 访问权 + rights” + Plan 9 的“每会话命名空间” + Smalltalk/KeyKOS 的“OID/检查点”。对象：task/run/session/artifact/message/tool/capability；稳定句柄；生命周期；审计。
- **IPC/syscall**：seL4 的同步会合 + 有界消息 + reply cap；工具调用就是 syscall；不做无界内核队列；异步事件走通知/用户态队列。
- **调度**：BEAM 的 reduction/时间片/每会话隔离 → agent 的 turn budget、tool budget、per-session 上下文；协作式，turn 内不抢占。
- **上下文**：BEAM 每进程私有堆 + GC → 每会话上下文/分页；`context_map` 是页表；压缩/截断是 swap；`recenter` 是 context switch；回放是 page fault。
- **能力/权限**：工具即 capability；按 session/run 授予；preopen 式初始工具集；无 ambient authority；父即沙箱边界；审计。
- **持久化**：OID + 检查点 + journal；终态落盘；崩溃恢复；不持久化 native 栈/JIT/OS 句柄。v1 先做 state/checkpoint，不做全系统正交持久化。
- **宿主适配**：文件/进程/网络/凭据/终端路由到 Windows/Linux；Job Objects/cgroups；named pipe/Unix socket；atomic rename；credential store。

## §4 不借什么

- 完整微内核（驱动、MMU、调度器）——宿主已有。
- 完整 Smalltalk image（native 栈/JIT/OS 句柄）——只持久化逻辑状态。
- 完整 BEAM（VM/GC 实现）——借隔离与预算，不实现 BEAM。
- 完整 Fuchsia component framework——借路由/命名空间，不引入整套组件模型。
- WASI 的绝对纯度——handle 在内部，路径在工具边界受控解析。
- Erlang 无界 mailbox/选择性接收——用有界队列 + backpressure。
- Plan 9 “只靠全局约定”——关键不变量要机械强制。

## §5 证据缺口与下一批阅读

1. **Zircon handle rights / channel / VMO**：`/concepts/kernel/objects` 404；下一批读 `zx_channel_write`、`zx_vmo_create`、`zx_handle_duplicate`、`concepts/kernel/handles`。
2. **seL4 CSpace/权限位/mint-revoke**：`Tutorials/capabilities.html` 返回空；下一批读 seL4 Reference Manual 的 CSpace 章节。
3. **GemStone/S 事务与并发 GC**：持久对象堆最完整的工业样本。
4. **KeyKOS Nanokernel Architecture (1985 OSR) / EROS checkpoint 技术报告**：检查点一致性的原始出处。
5. **Oberon 持久对象/GC 技术报告**：ETH 技术报告系列，不是目录页。
6. **Lively/Kolmogorov “object memory as database”**：OODB 化持久堆路线。

## §6 参考

- seL4 IPC: https://docs.sel4.systems/Tutorials/ipc.html
- Plan 9: https://9p.io/sys/doc/9.html
- Erlang/OTP Design Principles: https://www.erlang.org/doc/system/design_principles.html
- The BEAM Book: https://blog.stenmans.org/theBeamBook/
- Fuchsia capabilities: https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities
- WASI capabilities & filesystems: https://blog.sunfishcode.online/capabilities-and-filesystems/
- Smalltalk: https://en.wikipedia.org/wiki/Smalltalk
- KeyKOS: http://www.cap-lore.com/CapTheory/KK/
- Project Oberon: https://people.inf.ethz.ch/wirth/ProjectOberon/index.html

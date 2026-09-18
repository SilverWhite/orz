# ZCode 静默快照上传事件调查（ZCODE SNAPSHOT EXFIL INVESTIGATION，2026-09-19）

> 状态：`reference`；索引登记 `SEC-ZCODE-SNAPSHOT-EXFIL`。
> 发起：用户令「zcode社区当前汇报有git自动静默上传的警告，我需要确保本机安全，请进行一轮相关事件的全面检索并对本机zcode进行严格扫描」（2026-09-19）；后续令「把 checkpoints 彻底锁死吧，打成空实现」「请加一个 checkpoint 路径的哈希检测」「请将其记录进 CLI 项目中」。
> 方法：社区检索（本地检索技能链：DuckDuckGo 经代理命中原始披露）＋本机只读取证（进程、网络连接、配置与凭据文件、日志、SQLite 会话库、归档二进制静态分析）＋受控变更后逐项复验（进程重启对照、端到端拦截实测）。
> 范围：受影响工作区＝`D:\CLI`（ZCode 唯一以 project 用途打开的工作区）；被检对象＝ZCode Desktop 3.12.3.7463（`B:\Zcode\ZCode.exe`）。**本仓库自身未引入该行为**——事件源自第三方工具 ZCode，本档记录其对本仓库工作区的实际影响与已落地处置。
> 范围限制：上游快照服务端（阿里云 OSS）侧无法取证，云端留存现状不可核实；被上传密文使用服务端下发公钥加密、私钥仅在云端，本地不可解密，故「云端究竟持有什么」只能由其上传清单与本地密文包推定，不能直接验证。

---

## §1 结论速览

1. **社区告警属实，且本机命中。** ZCode 在每次提问前与每轮任务结束时，把工作区整体（含完整 `.git` 历史）打包加密上传；界面上的「仓库快照索引」开关**不参与**采集与上传判定，只影响服务端是否建索引。
2. **本机实际外传范围＝`D:\CLI` 工作区。** 可回查的最后一次成功上传为 **2026-09-13 11:50:36 (+08:00)**，清单 8,043 条路径 / 声明 181,585,134 字节，其中 `.git/` 6,199 条；与 `git ls-tree 659435e8` 的跟踪清单逐项吻合，即**该时点的整棵工作树＋完整版本历史**。
3. **主项目 `D:\AGI` 未进入任何快照。** ZCode 工作区历史仅 `D:\CLI`、`D:\CLI\orz`、默认工作区三条；两份快照清单的 `workspaceKey` 分别为 `D:\CLI` 与默认工作区，`D:\AGI` 零命中，清单内 0 条路径位于工作区之外。但 `D:\AGI` 的内容**经模型通道**流出（`KNOWLEDGE_TRANSFER.md` 全文、`models/lif3d_neurons.py` 全文、`phase0_model.py`/`lif3d_only_model.py` 片段、实验 `lif3d_summary.json` 解析输出）——此为 API 形态的固有结果，与本档关注的「结构化归档上传」性质不同。
4. **处置已落地并逐项实测**：投料区清空＋拒绝写入锁；`app.asar` 采集模块打桩为空实现；文件系统看门狗（状态哈希＋变动即杀进程）；`app.asar` 补丁看门狗（补丁丢失即杀进程、自动重打、自动重启）；阿里云监控出口 DNS 层封禁；微信机器人删除。
5. **残余风险**：ZCode 自动更新会替换 `app.asar`（已由第二道守卫自动中和）；ACL 拒绝项对目录所有者不构成硬边界（已实测可被自身摘除），故真正的锁是代码打桩而非文件权限。

---

## §2 快照通道实证

### 2.1 机制（静态分析）

`app.asar` 内 `out/host/index.js` 含完整采集链路：`RepoSnapshotSidecarService`（侧车服务）、`repoSnapshotCaptureIntentScheduler`（采集调度）、`RepoSnapshotAfterTaskComplete`（任务完成触发）、`RepoSnapshotUploadClient` / `RepoSnapshotUploadWorker`（上传）、`RepoSnapshotEncryptedArtifacts`（产物加密）、`RepoSnapshotGlobalConfigs`（全局配置采集）。

上传流程：`POST /api/v1/snapshot/upload-credential` 申请临时凭证 → 以 `multipart/form-data` 上传 `repo-snapshot.tar.gz.enc` 至对象存储。触发点两处：`captureBeforePrompt`（提问前）与 `captureTaskCompleteUpdate`（任务完成）。

采集范围含 `manifest`（文件清单，**作为请求元数据明文发送**）、`extraManifest`（`app-memory:global-settings`）、文件体（加密包）。加密信封为 `aes-256-ctr` 内容加密 + `rsa-oaep-sha256` 密钥封装，公钥服务端动态下发、私钥仅在云端。

> 判据订正记录：调查初期曾依据 `Cf(e){return e?.repoSnapshotIndexingEnabled===!0 && e.repoSnapshotIndexingUserConfigured===!0}` 判定「双重用户开关为门槛」，据此得出「本机从未采集」的结论。该判定**错误**——`Cf()` 仅决定服务端是否建索引。订正依据为本机 `checkpoints` 目录下的实际产物（下节）。

### 2.2 本机产物（决定性证据）

`C:\Users\1\.zcode\v2\checkpoints\` 下发现两个工作区目录：

| 目录 | `workspaceKey` | 内容 |
| --- | --- | --- |
| `6e23576dd1ed` | `D:\CLI` | `pending\…tar.gz.enc` **200,049,245 字节**；清单 9,155 条（`.git/` 7,186 条）；`state.json` 记 `failureCount: 289`、`uploadCredentialHandle: d61fb041-…`、`captureStage: terminal`、`historyRoundCount: 171` |
| `95e4e38f3ef5` | `.zcode\workspace\default` | 内容为 0 字节的空工作区快照（756 字节密文） |

`state.json` 另存 `lastAcceptedManifestHash`，指向 2026-09-13 11:50:36 生成的清单——该字段语义为「服务端已接受」，即**该次上传成功**。

### 2.3 最后一次成功上传的定位

| 时刻（+08:00） | 事件 |
| --- | --- |
| 2026-09-13 11:50:06 | 提交 `659435e8`：`0z S3 载体重建完成 + GitHub Release v0.5.0 发布` |
| 11:50:36.845 | ZCode 会话 `sess_57152ff1` 末轮 turn 结束 |
| 11:50:36.923 | **快照 `createdAt`**（距 turn 结束 78 ms，`captureStage=terminal`） |
| 11:51:21 | 清单落盘 |
| 11:56:37 | 下一笔提交（TB 2.1 V4.1 排期） |

触发该次采集的会话标题为「查阅CLI_PROJECT_INDEX.md确认项目进度」（10:15:40 起，`D:\CLI`，权限 `yolo`，118 条消息）——即**仅一次索引查阅**即在该轮收尾时带走了整个工作区。

### 2.4 上传内容重建（与 git 交叉验证）

清单 `790eee2f…`（`workspaceKey = D:\CLI`）共 8,043 条路径，0 条位于工作区之外。按顶层目录与 `git ls-tree 659435e8` 对照：

| 顶层 | 清单记录 | git 跟踪 |
| --- | --- | --- |
| `.git` | 6,199 | —（对象库） |
| runtime | 467 | 467 |
| assurance | 335 | 336 |
| docs | 306 | 306 |
| _windows_high_nist | 206 | 213 |
| scripts | 202 | 205 |
| _linux_arm_dryrun | 73 | 73 |
| 存档 | 55 | 56 |
| integration | 49 | 49 |
| regression | 33 | 33 |
| architecture | 30 | 30 |
| prototype / adr / protocol / … | ~80 | ~80 |

非 `.git` 条目 1,844 条 vs 跟踪文件 1,857 个：**云端拿到的是该时点整棵工作树（源码、文档、保障资产）加完整 `.git` 历史**。未纳入：`orz/` 子模块内容（子模块被排除）、`.claude`、`.agents` 及库外其他仓库。

### 2.5 本仓库内的 LIF 资产与快照的交集

`659435e8` 时点被跟踪且与 LIF 直接相关的文件（随该次上传进入云端）：

| 文件 | 大小 |
| --- | --- |
| `存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-31_V2.json` | 1.31 MB |
| `存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-31_R2.json` | 59.9 KB |
| `存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-31_R1.json` | 60.0 KB |
| `存档/root-artifacts-2026-09-06/LIF_102RUNS_REPLAY_2026-08-30.json` | 64.6 KB |
| `存档/root-artifacts-2026-09-06/LIF_102RUNS_CLUSTERING_2026-08-31_V2.json` | 3.3 KB |
| `assurance/lif_v2_cluster_analysis.py` | 5.7 KB |
| `assurance/fixtures/p5/lif-r211-readonly-projection-v0.1.json` | 2.2 KB |

注：`docs/LIF3D_BASE_PORT_EXTRACTION_2026-09-16.md`（2026-09-16 生成、后随 `8f27d30c` 撤除）与 `docs/LIF_DYNAMICS_PROJECTION_AND_ROUND_BUDGET_DESIGN_2026-09-16.md` 会被 2026-09-18 那次打包（9,155 条）纳入，**但该次上传未成功**（289 次重试未获接受，密文留存本地后已被删除）；9-13 成功那次在其创建之前（`git cat-file` 证实 `659435e8` 中不存在）。

---

## §3 主项目 `D:\AGI` 与 LIF 的核查

### 3.1 快照通道：零命中

- ZCode 工作区历史仅 `D:\CLI`、`D:\CLI\orz`、默认工作区（来源：`setting.json` 的 `recentProjects` / `lastWorkspaceSession`，CLI 会话库 `session.directory` 去重）。
- 两份快照清单 `workspaceKey` 分别为 `D:\CLI` 与默认工作区，`D:\AGI` 从未出现；清单内 0 条工作区外路径。
- `D:\AGI` 本身不是 git 仓库（无 `.git`）。

### 3.2 模型通道：确有内容流出（记录事实）

CLI 会话库 `part` 表工具调用（`state.input`）中命中 `D:\AGI` 的三次会话：

| 时间 | 会话 | 送出内容 |
| --- | --- | --- |
| 2026-09-16 00:47–01:28 | `sess_ddb21044` | 仅 `du` / `ls` 目录名与体积，无文件内容 |
| **2026-09-16 11:58–12:00** | `sess_a99ba820` | **`KNOWLEDGE_TRANSFER.md` 全文（2,848 B）**、**`models/lif3d_neurons.py` 全文（32,989 B）**、函数签名与 `training/` 下 LIF3D 参数检索 |
| **2026-09-17 09:43–09:53** | `sess_d194574c` | `models/phase0_model.py` 第 55–100 行、`models/lif3d_only_model.py` 第 28–145 行、`_tmp_decoder_summary.py` 前 60 行、`experiments/overnight_2026-05-08_R22_combined/*/seed_*/lif3d_summary.json` 解析输出 |

该通道属 API 形态固有结果（上下文必需），本档只做事实登记，不列为缺陷。

---

## §4 已落地处置

### 4.1 投料区：清空 ＋ 拒绝写入锁

- 删除 `C:\Users\1\.zcode\v2\checkpoints` 全部内容 **201,928,817 字节**（含 200 MB 密文包与两个工作区目录）。
- ACL：`icacls … /deny "SWITCH\1:(OI)(CI)(WD,AD,D)"`；实测写入、建子目录、枚举均被拒。
- **负面发现（重要）**：该目录所有者为 `SWITCH\1`，所有者身份可自行移除该拒绝项（`icacls /remove:d` 实测成功、写入随即恢复）。故 ACL 只能拦「程序无意中的写入」，**不是硬边界**；真正的锁见 4.2。

### 4.2 `app.asar` 采集模块打桩为空实现

目标：`app.asar` 内 `out/host/index.js`（宿主进程）。注入点全局唯一：

```js
Li=new Bk({stateRepo:Fe,uploadClient:zt,uploadWorker:rn, ... })
```

改为等长（9 字节 → 9 字节）字节覆盖：

```js
Li=0&&(0)({stateRepo:Fe,uploadClient:zt,uploadWorker:rn, ... })
```

`&&` 短路使右侧永不求值，`Li` 恒为 `0`；所有消费处均为 `if(!…repoSnapshotSidecar)return` 形态，采集与上传链路整体失效。**归档头部未改动**，条目偏移不变，仅文件内 6 字节被覆盖。

| 验证项 | 结果 |
| --- | --- |
| `Li=new Bk` 出现次数 | 0（改前 1） |
| `new Bk(` 全归档 | 0（另有一处属 mermaid，无关） |
| 侧车类实例化点 | 不存在（类定义保留、永不构造） |
| `out/host/index.js` 长度 | 2,588,119 字节前后一致 |
| ESM 语法校验（node `vm.SourceTextModule`） | 原始与补丁后均通过 |
| ZCode 启动 | 宿主正常加载、全部 RPC 通道注册成功 |
| 连续 3 次重启对照 | 无新增崩溃，`checkpoints` 始终为空 |

补丁后首次启动曾出现 1 次**原生**渲染进程崩溃（该机历史日志唯一一次）；随后 2 次对照重启均未复现、宿主每次正常加载，判定为前一步强制结束进程留下的一次性环境事件。

### 4.3 看门狗（两道守卫，计划任务 `ZCodeCheckpointWatchdog`）

脚本：`C:\Users\1\.zcode\watchdog\zcode-checkpoint-watchdog.ps1`（纯 ASCII，规避 PowerShell 5.1 ANSI 解析问题）。

**守卫一 — 投料区状态哈希**：每 2 秒对 `checkpoints` 计算 SHA256 状态指纹（存在性、所有者、全部拒绝 ACE、递归条目清单〔相对路径/大小/UTC ticks〕；「不可枚举」本身亦作为状态参与指纹）。指纹偏离基线即**立即强制结束全部 ZCode 进程**并记录事件；状态回归基线自动解除。次级扫描每约 20 秒在 `~/.zcode/v2` 下查 `*.enc` / `*.tar.gz`。

**守卫二 — `app.asar` 补丁监控**：记录补丁标记绝对偏移，每 2 秒读该处 9 字节比对；首次不匹配后 0.7 秒复读，两次不一致则跳过本轮（防更新写入中的瞬时误判）。确认丢失后：记 `CRITICAL` → **立即杀进程** → 自动重打补丁 → 更新偏移基线 → 自动重启 ZCode；重试 15 次仍失败则记 `CRITICAL` 且**不重启**（宁可不跑，不放行未打桩实例）。

计划任务：登录时启动 ＋ 每 10 分钟兜底触发（`MultipleInstances=IgnoreNew`），失败自动重启，无执行时限。

**端到端实测**（两道守卫均实跑）：

```
# 守卫一：摘除拒绝 ACE
02:08:53 [KILL] terminated ZCode pids [1556,6216,8852,9368,10104,19612,21572,22624,23916,24048,24588]
02:08:53 [DIFF] state detail: deny=（空）; entries=\t.tmp|1|…   ← 含测试残留文件，已清除
# 守卫二：还原 asar 模拟更新
02:12:48 [CRITICAL] asar stub LOST (offset 253505887 reads 'Li=new Bk') -- ZCode would be able to snapshot again
02:12:48 [KILL]     terminated ZCode pids [6176,8252,11668,17900,21188,24540]
02:12:51 [REPAIR]   asar stub re-applied (offset=253505887, attempt=2)
02:12:53 [RELAUNCH] ZCode restarted after repair
```

守卫一测试暴露并修复两个问题：权限测试残留的 1 字节文件（已清除）与无进程时的日志刷屏（改为仅在首次检出与 ZCode 复起时记录）。

### 4.4 出口拦截（阿里云监控）

ZCode 的阿里云外发仅两个域名，均属纯监控，已在 hosts 指向 `0.0.0.0` 并 `ipconfig /flushdns` 验证解析：

| 域名 | 用途 |
| --- | --- |
| `sdk.rum.aliyuncs.com` | ARMS 前端监控 SDK 资源与配置 |
| `proj-xtrace-7e235817c9b9381c22d8b743908d469f-cn-beijing.cn-beijing.log.aliyuncs.com` | ARMS/RUM 数据上报 ＋ `apm/trace/opentelemetry` 链路上报 |

边界：ZCode 宿主进程直连（不经本机 Clash，实测其 socket 不接 `127.0.0.1:7890`），故规则层拦截须落在 hosts/防火墙而非代理规则；`zcode.z.ai` 同时承载 OAuth、客户端配置、更新清单与遥测 `/api/v1/event/report`，按主机封禁会破坏登录与使用，路径级隔离需本地 MITM，未实施。

### 4.5 微信机器人删除

`bot-config.v3.json` / `bot-config.json` 配置置空、`bot-state.*.json` 状态置空、`credentials.json` 移除 `bot:…:credential`；因轮询器每轮重写状态文件，按「停进程 → 改文件 → 重启」顺序执行。重启后配置保持为空，到 `weixin.qq.com` 的连接消失。原始文件备份留存于调查工作目录。

补充：调查中一度观察到 `220.196.154.96:443` 长期连接，经读取其 TLS 证书确认为 `CN=weixin.qq.com`（腾讯），即上述机器人的轮询出口，非陌生外联。

---

## §5 复现与运维入口

| 用途 | 命令 / 路径 |
| --- | --- |
| 补丁状态检查 | `python zcode-snapshot-stub.py check` |
| 打补丁 / 还原 | `python zcode-snapshot-stub.py apply` / `restore` |
| 看门狗事件日志 | `C:\Users\1\.zcode\watchdog\events.log` |
| 投料区基线 / asar 基线 | `…\watchdog\baseline.json` / `asar-baseline.json` |
| 暂停看门狗（改动投料区前必须先做） | `Stop-ScheduledTask -TaskName 'ZCodeCheckpointWatchdog'` 并终止对应 `powershell.exe` |
| 彻底移除 | `Unregister-ScheduledTask -TaskName 'ZCodeCheckpointWatchdog' -Confirm:$false` |

运行约束：`apply` / `restore` 前须完全退出 ZCode；ZCode 自动更新会替换 `app.asar`，守卫二会自动中和，但如需人工确认可加 `-NoAutoRepair`（只报警不动手）。

---

## §6 边界与残余风险

1. **更新面**：每次 ZCode 自动更新都会替换 `app.asar`，打桩失效——由守卫二自动重打；若上游改变代码布局导致标记消失，工具会安全拒绝改写并记 `CRITICAL`，此时需人工重新定位注入点。
2. **权限面**：ACL 拒绝项对所有者不构成硬边界（§4.1 实测）；真正的约束是代码路径已不存在。
3. **出口面**：对象存储地址由服务端凭证响应动态下发，无法以域名钉死；防线设计为「不产生可上传物」而非「传不出去」。`zcode.z.ai` 未整体封禁（会破坏登录与用量接口）。
4. **取证面**：云端侧不可见。本档「已成功上传」的判定基于客户端 `lastAcceptedManifestHash` 语义与清单时间戳，属强证据但非服务端回执。
5. **早前产物**：更早的 checkpoint 目录若被程序清理则无法回查；2026-09-13 为本机现存记录中的最后一次成功上传。

---

## §7 对仓库的直接影响与后续建议

1. 本仓库 2026-09-13 时点的全部跟踪内容（含完整 `.git` 历史）已进入第三方云端；由于该仓库非公开（外部未认证访问返回 404），其性质属**私有源码与历史的结构化外泄**，而非「已在公网」。建议按此定性评估是否需要轮换历史中可能存在的任何凭据。
2. 在本机使用任何第三方 AI 编程工具前，应先确认其是否存在工作区归档上传行为；本档记录的检测手法（`checkpoints` 产物 + `state.json` 字段 + 清单与 `git ls-tree` 对照）可直接复用。
3. 若后续更换工具或版本升级，应重跑本文 §2.2 与 §4.2 的检查项。

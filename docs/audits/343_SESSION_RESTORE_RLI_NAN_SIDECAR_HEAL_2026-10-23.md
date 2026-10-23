# 343 批：ORZ 会话恢复 RLI NaN 侧车毒化修复（2026-10-23）

> **用户令**：「当前恢复并继续存在 bug，最新会话记录在 `C:\Users\1\OneDrive\Desktop\新建文件夹`，请进行 bug 检查，并且交互侧也存在问题。」
>
> **数据纪律**：真实工作区 `.gsa` 只读取证，全程不改写；复现与验证在隔离副本 `D:\tb-eval\repro_343\ws`（整卷拷贝、剔除 chrome-profile）完成。

## §0 结论

340/341 修复的 legacy 身份迁移条件在真机数据上**全部成立**，恢复失败的真正根因在更下游：`c23e0aaa` 与**当前活跃会话** `51da03ea` 的会话信封（`.gsa/conversations/<s8>.json`）内嵌的 RLI 快照被 **NaN→null 毒化**，读取端整份拒绝，`session/load` 把「损坏」误报成 `SessionNotFound`，前端渲染为「该会话已删除或从未创建」——用户点「恢复并继续」必然失败且文案撒谎。

毒化源头是 RLI 通道闭式自由演化（0am 影子层）的**过阻尼分支数值溢出**：`φ/cosh/sinh` 形式在跨长间隔（隔夜续跑）推进时 `cosh(μΔt)→inf` 而 `φ→0`，`0·inf = NaN`，serde_json 把 NaN/±inf 落成 `null`。ζ=2.0 过阻尼的 prog/verify/ctx/infra 四通道全部中招；ζ<1 欠阻尼的 err/stall/slow/deny 因 `cos/sin` 有界而幸存——与真机文件逐通道吻合。

本批修复三件：

1. **根因**：`free_evolution_at` 过阻尼分支改**两根指数形式**（两特征根恒 ≤ 0，指数不再溢出；与旧式在同点代数恒等）；
2. **读取容忍（毒化侧车自愈）**：`RliChannelSnapshot.u/v/envelope` 与 `RliDomainRow.u_prog` 四个被判字段 `null → 0.0`（长间隔自由演化的数学极限即静止水平，非虚构观测；FR-7 的 Option 语义面不在此列）；
3. **交互侧如实报错**：侧车在盘但解析失败时，`session/load` 报 `Host`（「会话侧车损坏，拒绝装载」+ 路径 + serde 证据），不再伪装成「会话不存在或已被删除」。

端到端实测：毒化副本上「恢复并继续」从失败转为成功；磁盘三件自愈——`.closed` 清除、信封 `session_id` 规范化为完整 UUID（340 迁移首次真机落地）、lif 段毒点 null 清零（信封重写自愈）；恢复后探索器分组正确（已关闭 0、活跃 2），标记栏落 T27（最新轮）。

## §1 真机取证（只读）

| 项 | `c23e0aaa`（已关闭） | `51da03ea`（活跃） |
|---|---|---|
| 信封 `session_id` | 短 ID（legacy 形态，340 已知） | 完整 UUID |
| RLI 通道毒化 | prog/verify/ctx/infra 的 `u/v/envelope` 全 null（4×3） | 同（4×3） |
| 域行 `u_prog` 毒化 | 19 处 | 15 处 |
| 欠阻尼通道（err/stall/slow/deny） | 全部正常数字 | 全部正常数字 |
| `d24eadd6`／`54d12bcc` | 无毒化（前者信封无 null 状态字段，后者无 RLI 快照） | — |

活体复现（0.9.10 装机件、隔离副本）：探索器正确投影 `closed=true / legacy_short_id=true` + 完整 UUID（341 核证结论复现）；点「恢复并继续」→ `（会话 c23e0aaa 恢复失败，仍为只读回看：会话不存在或已被删除（session）……证据：session=c23e0aaa-e752-…）`——与用户所见一致。

## §2 根因链

```text
会话跨隔夜续跑（Δt 数万秒）
  └─ RliChannel::advance → free_evolution_at（过阻尼 ζ=2.0）
       ├─ phi = exp(−ζωΔt) → 0
       ├─ cosh(μΔt), μΔt ≳ 710 → inf
       └─ 0 × inf = NaN（u/v/E 全字段）
            └─ snapshot() → quantize 不改 NaN
                 └─ serde_json 写 NaN → null（毒化信封）

「恢复并继续」→ session/load（完整 UUID）
  └─ load_conversation_sidecar_for_attach
       └─ StoredConversation 反序列化失败（invalid type: null, expected f64）
            └─ attach refused → SessionNotFound   ← 毒化的唯一出口
                 └─ 前端：「该会话已删除或从未创建」← 交互侧谎言
```

注意：340 的身份迁移逻辑本身无缺陷——它从未被执行到（解析先死）。

## §3 修复设计

### 3.1 两根指数形式（根因）

过阻尼闭式解按特征根重排：`φ·cosh(μΔt) = (e^{r₁Δt}+e^{r₂Δt})/2`、`φ·sinh(μΔt) = (e^{r₁Δt}−e^{r₂Δt})/2`，其中 `r₁ = μ−ζω ≤ 0`、`r₂ = −μ−ζω < 0`。指数恒非正 → 长间隔自然衰减到 0，无溢出。小间隔与旧式逐位等价（代数恒等，回归钉对拍）；半群性、`scan_crossing`、`forecast` 等共用调用面自动受益。欠阻尼分支不动（`cos/sin` 有界、`φ·√(A²+B²)` 在现实量级下安全）。

### 3.2 读侧容忍（已毒化侧车的自愈路径）

`f64_null_as_zero`（`Option<f64>` 反序列化 + `unwrap_or(0.0)`）应用于四个被判字段：通道快照 `u/v/envelope`、域行 `u_prog`。语义依据：这些字段是通道**动力学状态**（对静止水平的偏移），长间隔自由演化的正确极限就是 0；FR-7「不落假 0」约束的是 λ̂/时间戳等「不可得 ≠ 0」的 Option 面，不在本列。写侧修复（3.1）落地后 NaN 不再产生，毒化文件在首次成功装载的信封重写中**永久自愈**。

### 3.3 损坏 ≠ 不存在（交互侧如实面）

`conversation_sidecar_parse_error`（只读探测）+ `session/load` 失败分支分流：文件可解析但身份不满足 → 维持 340 语义 `SessionNotFound`（fail-closed 不变）；文件在盘但解析失败 → `AcpError::Host("会话侧车损坏，拒绝装载（sidecar corrupt，非「不存在」）：{path}: {serde 错误}")`，经 334 四要素面（internal 类）上浮真实原因与证据。fail-closed：绝不自动改写毒化文件。

## §4 代码触点

| 触点 | 改动 |
|---|---|
| `orz-assurance/src/lif/rli.rs` | `free_evolution_at` 过阻尼两根形式；`f64_null_as_zero` + 四字段容忍；2 个回归钉 |
| `orz-host/src/acp_server.rs` | `conversation_sidecar_parse_error`；load 失败分流如实报错；1 个回归钉 |
| 测试 | 清退上批遗留的 `ORZ_DIAG_ROOT` 取证测试（未提交物），换正式回归钉 |

## §5 验证

| 门 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test -p orz-assurance` | **306+9+4+9+1 全绿**（含新钉 2：null 容忍往返、隔夜 Δt 有限+衰减+与旧式对拍） |
| `cargo test -p orz-host -- --test-threads=1` | **430 passed / 0 failed**（含新钉：损坏侧车报损坏不报不存在、fail-closed 不改写） |
| `cargo test -p orz-web` | 78/0 |
| `frontend smoke` | 全部通过（17.1/17.2/17.3 回放顺序钉保持） |
| `git diff --check`（orz 子仓） | exit 0 |
| 端到端（隔离副本 × 修复版 debug 二进制） | 恢复成功消息；`.closed` 清除；信封规范化完整 UUID；lif 段 null 清零；活跃（2）/已关闭（0）分组；标记栏 T27 |

端到端过程含一次中间失败：毒点第二类（域行 `u_prog`）首轮未覆盖，如实报错钉当场给出 `line 3104 column 26` 定位，补容忍后全通——该失败同时验证了 §3.3 交互面的取证价值。

## §6 残余与发布边界

1. **Windows 已同批换装 0.9.11（用户令「只重建 win 包并换装」）**：orz 子仓 `bd42e011`（fix）＋`73b95597`（bump）已推 `cli/feat/fusion-architecture`；release 重建 3m02s 零 warning，`orz.exe` 59,286,016 `c1c5faf5…`／signer `1bac63e4…`／provision `3c6aa9db…`；载体位＋装机位 MATCH 3/3，两侧 0.9.10 三件备份 `.0.9.10-343-bak`；ACAF 重 provision（`signer-manifest.json.bak-20261023-343`，新 `binary_sha256=1bac63e4…`）；载体清单重生成；carrier doctor `12/0/0`、装机位 `11/1/0`（装机位本不放载体清单）。目标工作区活体：bridge/child 均 `0.9.11`、`version_skew=false`，四会话投影正确。换装时在役栈已由用户自行停止（无停栈动作）；重启后旧标签页 token 失效，须用新 URL。**未发行**（无 GitHub Release、无 Linux 载体——留下一自然节点）。用户点「恢复并继续」（c23e0aaa）即触发真机自愈；活跃 `51da03ea` 在后续桥重启 attach 时自愈。
2. 交互侧既有登记项不动：`30 次运行` 计数口径（ARC 计入、RUN-1 无终态）沿 340 §6 残余；归档包不保存原 mtime 沿 338 残余。
3. `session/new` 侧 `attach_session_with_options` 对毒化侧车的行为（空信封回退）不在本批面；该路径用户不可达（load 已前置拦截），登记为边界不扩面。
4. 桥重启后旧标签页 token 失效须开新 URL 是 341 已登记行为，本批未改。

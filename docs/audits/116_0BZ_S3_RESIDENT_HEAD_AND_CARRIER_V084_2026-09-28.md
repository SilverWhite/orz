# 116 批：0bz S3 渲染稳定化修码与 0.8.4 载体重建（2026-09-28）

> **用户令**：「请进行0bz S3吧」。
> **本批**＝**0bz / GAP-CONTEXT-FACE-TRANSIENT-FORK 的 S3**——按 115 批 S2 读数落两处
> 渲染稳定化修码（只动位置／时点、不动内容）＋双平台载体重建 **0.8.4** 进体。
> **源冻结**＝orz **`9f12ecc2`**（`b956b3ba` S3 修码＋`9f12ecc2` 版本 bump）；**未推送未发行**。
> **计数 56 不变**（`0bz` 维持 `pending`：余 S4 真机对账）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 修码①（窗口开窗脸） | 指针＋D4 **自首个轮次起常驻** face（注入点＝`round_ranges[0].0`，与开窗后 `preamble_end` 恒同值）⇒ 开窗转换只剩尾部追加，前缀零重排 |
| 修码②（压缩落地脸） | 审计确认现树 marker 单写手、D4 重渲与 marker 同拍落 +1（115 批 0/2 第 2 针实证）；**两连压稳定性钉**锁死纪律（marker1 字节/位置与指针位在第二次落地后不变） |
| 钉子 | ＋2：`resident_head_from_first_round_and_opening_keeps_prefix_byte_stable`／`compression_landing_marker_is_byte_stable_across_next_landing`（model_face tests） |
| 读数 | orz-loop **860/0**（+2 钉；既有协议钉 2 处按新不变量更新 3→5 条）／orz-assurance **278/0**／orz-tui **178/0**；clippy/fmt 本批触碰面零新增 |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` exit 0（3m42s）；换装 MATCH；`--build-info`＝`version=0.8.4`；ACF 重 provision 绑 `9f4b4c73…`，keystore 未动 |
| Linux musl | 见 §3（冷缓存重建，/target 缓存 113 批清理后首次全量） |
| 打包 | `rel-116-stage` 两侧各 5 entries；容器核证＋清单活体两态（§4） |
| 台账 | pin `9f12ecc2`；源清单重生成；TODO/BACKLOG/索引 v4.84/第二卷 §1.68；门禁 `valid: true` |

## §1 修码①：窗口开窗脸（机理与修法）

- **机理（115 批 S2 指纹定位）**：会话前期上下文在主滑块内 ⇒ `build_model_face` 早退分支
  **原样返回**（指针/D4/块表全不在场）；首个分块形成（开窗）那一刻，指针＋D4 才整体插入
  face 前部 ⇒ 全部消息位置后移 ⇒ 前缀全变。`RUN-CLI-6ab99969` r40（seq417）＝hit 6,528／
  miss 135,754、fd=index 1 mutated（head＝指针消息文本）——即六轮狗粮「自发塌陷」族
  （miss 分账 7%）的机理解剖。
- **修法**：`try_clone_messages_with_resident_head`（新）——无分块分支在 `round_ranges[0].0`
  （首个轮次起点）注入指针＋D4。该点与开窗后 `preamble_end = blocks[0].msg_start` 恒同值
  （首个分块必为首个完整轮，r59 的 `[题面, marker, 指针, D4]` 几何同规则复现）⇒ 开窗转换
  只在**尾部**追加块表/RUN_END 行，前缀逐字节稳定。估算/计数两面同口径修改。
- **边界**：`build_pointer_message`/`render_run_context_block` 文本一字未改；D4 的 epoch
  冻结纪律不变（`agent_loop.rs` 仅把渲染起点从「分块非空」提前到「首个 loop-top」）；
  轮次未成形时不注入（成形时 face 仅数 K token，一次性小成本）。

## §2 修码②：压缩落地脸（审计与锁死）

- **审计结论**：marker 单写手（`compress_blocks_now` 插入被压首轮起点一次）；D4 重渲
  （`face_d4_block = None`）与 marker 插入**同拍**落在 +1 轮 face；+1→+2 之间现树无二次
  写手（115 批实机 0/2 第 2 针、r71 marker 在 r72–74 逐字节稳定）。
- **锁死**：新钉模拟连续两次落地（生产几何），断言 marker1 字节与 face 位置、指针位、
  前缀（到指针位）在第二次落地后全部不变——防未来回退；S4 长轮以「+2 针归零」判据实测。
- **既有测试更新（如实记）**：`tool_call_round_trips_through_dispatcher`／
  `tool_round_replays_reasoning_content_on_declaration` 的 round2 协议形状 3 → **5** 条
  （常驻头＋2）；断言改为动态定位 declaration（provider 契约「声明先于结果」不变）。

## §3 Linux musl 三件套（Docker，直写换装位）

- 预检 `APT_OK`（零代理切换）；**冷缓存全量重建**（113 批清理 `/target` 后首次；
  rust:1.97-slim ＋ `build_orz_aliyun_trixie.sh`，ORZ-BUILD-MOUNT-001 契约）。
- 换装：旧三件预置 `.0.8.3-bak` 链后直写，与 `orz-target/.../release` **MATCH 3/3**；
  三件均 **static-pie linked**＋**INTERP=0**（alpine 3.20 file/binutils）；
  alpine/bookworm 双冒烟 `version=0.8.4`（exit 0）。
- 冷缓存：cargo **40m59s**（113 批清理 `/target` 后首次全量）；警告＝Linux-only cfg
  既有族（orz-config 1／orz-assurance 4／orz-host 3），本批触碰面零新增。
- 陈旧 bak 清理：Windows 侧 `.0.8.2-bak`×3 移除（回滚点更新为 `.0.8.3-bak`）。

## §4 打包与核证（`rel-116-stage`）

- `.tmp-b116-package.ps1`（沿 113 形态：清单打包阶段生成、System32 tar）；两侧各 5 entries；
  包内 README＝`.tmp-b116-readme.md`（新增 0.8.4 段）。容器核证与清单活体两态读数见文末补记。

## §5 台账同步

- pin `778fad39` → **`9f12ecc2`**；源清单重生成；TODO `P1-0bz` S3 勾选＋S2 部分读数注记保留；
  BACKLOG `0bz` 状态行更新；索引 §3 `0bz` 条目＋§8 桶＋头行 v4.84（v4.83 滚存档）；
  第二卷 §1.68；门禁 `valid: true`。

## §6 构建读数补记（已回填）

- **Windows**：exit 0（3m42s）；警告 1 条（orz-host unused import，既有面）；
  `--build-info`＝`version=0.8.4 os=windows arch=x86_64 profile=release`。
- **Linux**：cargo 40m59s（冷）；MATCH 3/3；双冒烟 exit 0。
- **版本字面量**：linux `0.8.4` 4 → 5／`0.8.3` 37 → 36（版本串滚动；Windows 侧
  build-info 直读 0.8.4）。S3 修码无新增可检字面量（纯装配路径改动，注释不入二进制）
  ——进体判据＝版本字面量＋源冻结一致性＋行为钉（S4 真机指纹读数）。
- **打包**：`orz-0.8.4-windows-x86_64.zip` 28,017,771 B `sha256 be400cb570262a434d6ea5e84ac214c75426d44483f9a03a095a91f91e56e6db`／
  `orz-0.8.4-linux-x86_64.tar.gz` 37,183,261 B `sha256 a4717696aa91096818b8af4d48d7eba5c99861b1fb5f069dda7053b9ce64d28b`／
  顶层 SHA256SUMS 191 B `sha256 3e8d34aa1e30310de6358c441eed3dd031647cd0fb436ab1d46a56530b3057b3`；
  包内 6/6 MATCH；容器核证 zip 4/4・tar 4/4・顶层 2/2；包内 `--build-info`＝0.8.4；
  清单活体两态：解压态 0 finding／README+1B 恰 1 条（32072 ≠ 32073）。

| 件 | Linux 0.8.3 → 0.8.4 尺寸差 | SHA256（0.8.4） |
|---|---:|---|
| `orz` | −304 B | `87941130b37da51e585d216df55c4e84e62d2571e448470bae20c86b5a01745e` |
| `orz-signer` | +8 B | `b1f639576aaa8253db1c51949027c843c5e056f911befa5a748cb63f2ea1fd94` |
| `orz-acaf-provision` | −16 B | `5bb027c4c88a6d3f128413b4f0ef28be7da95ff07a45e15132b3b9e608d1c27b` |

- Windows 侧三件 SHA256：`orz.exe 5e8f0f55…`／`orz-signer.exe 9f4b4c73…`／
  `orz-acaf-provision.exe b6ec0dd0…`（换装位与包内 6/6 MATCH；manifest 绑 `9f4b4c73…`）。

关键词：0bz S3、渲染稳定化、常驻头、开窗瞬态、自发塌陷机理、两连压钉、0.8.4 重建、未推送未发行、计数 56。

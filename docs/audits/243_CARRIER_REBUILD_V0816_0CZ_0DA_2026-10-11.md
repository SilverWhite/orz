# 243 批：0.8.16 双平台载体重建进体——0cz／0da 合并窗口（2026-10-11）

> **用户令**：「请开始进行重建吧」。
> **本批**＝0.8.16 代窗口双平台载体重建（沿 218/194/187 批形态：源冻结→bump→双平台重建换装
> →进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**进件面**＝`39116a81`
> （238 批 0cz S2 context_manage 在役＋241 批 0da S2 D1/D2/D5 修复与 replace 族＋242 批 0da S2
> 审查处置＋12 个纯 rustfmt 漂移文件随批）。
> **源冻结**＝orz **`316107f6`**（`39116a81`＋bump 0.8.15→0.8.16；Cargo.toml＋Cargo.lock 恰两行、
> `cargo metadata --locked` exit 0）；未推送。
> **零源码语义增量**；**计数不变（59）**；**未推送、未发行**（发布面停 v0.8.15〔225 批〕）。
> **0cz S3 与 0da S3 合并同一次重建**（设计稿 §9 既定），S3 字节判据双双在本批达成。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结＋bump | `316107f6`（恰两行 Cargo.toml/Cargo.lock 0.8.15→0.8.16；locked exit 0）；源清单随批再生成 **1,486 条**（差 27 行＝落码 25 文件＋bump 两文件） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（3m53s 暖缓存）**；换装 **MATCH 3/3**、`.0.8.15-bak` 链预建；`--build-info`＝`0.8.16 os=windows` rc0；载体清单 3 entries 刷新 |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261011-243`；provision **exit 0**；`binary_sha256=1919b56f…`↔换装位 `orz-signer.exe` 逐位一致；keystore 两件 Sep 12 mtime 未动；**本批零偏航**（正斜杠沿 218 教训） |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**（`-j 1` 暖 /target）；bak 链宿主预建；**MATCH 3/3（直写换装位）**；**static-pie×3**（ET_DYN＋PT_INTERP=0）；alpine 3.20/bookworm 双冒烟 `0.8.16 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新。**首跑偏航如实记**＝docker `-w /orz/orz` 被 MSYS 改写为 `B:/Git/orz/orz` ⇒ daemon 拒绝、构建未执行、旧件未触——`MSYS_NO_PATHCONV=1` 重跑成立（218 批 MSYS 参数剥除同族；冒烟 docker 命令同族一并修正） |
| 进体字节判据 | **0cz＋0da 合并窗口全命中**：`clear_all` 0→27（WIN）／1→28（LIN；基线 1 处为巧合字节序列、如实记）；`context_manage` 0→3／0→3；`keep_recent_rounds` 0→13／0→13；`handover` 0→38／0→27；清零 marker 前缀 `[前文上下文已清零` 0→2／0→2；guide 教学句 `清空黑板分区` 0→1／0→1；权限句 `一键清理` 0→8／0→8；`mode=clear` 0→23／23、`op=clear` 0→42／20。**194/218 窗保留面零回归**（`--target-directory` 1、`若意图是备份/暂存` 1、`deepseek-v4-flash` 0、`did you mean "` 2、`前推(` 1、负例 `cursor_agent` 0）；`findings` 62→92（WIN）／59→67（LIN）＝0da 清空路径三分区枚举等新增的增长面、非回归（如实记） |
| 冒烟 | WIN `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6acabdc9`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.8.16 在册、sha `76904567…`） |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `75515440…`→**`db95ed15…`**（注释同步；adapter `6d55c26e…` 未动；语法＋`--help` rc0） |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（187 批在案先存） |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.8.15→0.8.16） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 57,531,392（+396,288） | `76904567f4ad8fae742168bff99b29630dfa5c29b6a0f6475fde5b70c3e55b0a` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `1919b56fa7c9d37ba78ac1c8c71ba43cfc37d97dd84c85f6b77c76478df13d39` |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `446d8cc773e18144735d50b965ed02895f46d9b77fc498bbc2a6a5235ecfb211` |
| LIN | `orz` | 116,019,040（+403,128） | **`db95ed15c32d086fe5ad400c7f56cf535b18af04626c75a8ba9b54a3b7c3c0fb`（身份门新值）** |
| LIN | `orz-signer` | 1,397,464（−80） | `d462590f218bca5efee3fa166682b860d22fb200d4bdc1b118c6a577eaf25f5b` |
| LIN | `orz-acaf-provision` | 1,216,048（+56） | `a8c2dd4c7b4029d2452579f8a1f8499b73813b6539777f37496091ffd7b41020` |

（换装基线＝0.8.15 三件 `c9f30c75…`/`f86158ed…`/`c887b1a9…`（WIN）与 `75515440…`/`f1a1d784…`/`5893eec4…`（LIN），与 218 批账面逐位吻合。）

## §2 进体字节判据（方法与全量读数）

**方法**：python 字节直数（`bytes.count`；218 批同法）。**判据形态注（如实记）**：设计稿 §9 的
「section 枚举 +all」以组合面判据代证——`all` 单字节串不可数，由 `clear_all`/`op=clear`/
`清空黑板分区`/权限句 `一键清理` 的 0→N 合读覆盖（枚举值从不入面到入面的同一机械事实）。

| 判据 | 基线（0.8.15）WIN/LIN | 0.8.16 WIN | 0.8.16 LIN |
|---|---|---|---|
| 0da/0cz 新增：`clear_all` | 0／1（LIN 基线含 1 处巧合字节序列——如实记） | **27** | **28** |
| 0cz 新增：`context_manage` | 0／0 | **3** | **3** |
| 0cz 新增：`keep_recent_rounds` | 0／0 | **13** | **13** |
| 0cz 新增：`handover` | 0／0 | **38** | **27** |
| 0cz 新增：清零 marker 前缀 `` [前文上下文已清零 `` | 0／0 | **2** | **2** |
| 0da 新增：guide 教学句 `清空黑板分区` | 0／0 | **1** | **1** |
| 0da 新增：权限句 `一键清理` | 0／0 | **8** | **8** |
| 0cz 新增：`mode=clear` | 0／0 | 23 | 23 |
| 0da 新增：`op=clear` | 0／0 | 42 | 20 |
| 0cz 增长：`清零` | 0／0 | 45 | 48 |
| 0cz 增长：`清空` | 1／1 | 64 | 31 |
| 218 保留：`--target-directory` | 1／1 | 1 | 1 |
| 218 保留：`若意图是备份/暂存` | 1／1 | 1 | 1 |
| 218 保留：`deepseek-v4-flash` | 0／0 | 0 | 0 |
| 218 保留：`did you mean "` | 2／2 | 2 | 2 |
| 218 保留：`前推(` | 1／1 | 1 | 1 |
| 218 增长面：`findings` | 62／59 | 92 | 67（0da 清空路径三分区枚举/分区名新增——增长面非回归） |
| 负例：`cursor_agent` | 0／0 | 0 | 0 |

**判读**：0cz S3 字节判据（mode 枚举 +clear／+clear_all、权限句换版、声明描述扩写）与 0da S3
字节判据（op 枚举 +clear、section 枚举 +all〔组合面代证〕）**双达成**——`context_manage`/
`clear_all`/`keep_recent_rounds`/`handover` 自 0 全数入面、保留面零回归。

## §3 台账

- 本档：`docs/audits/243_CARRIER_REBUILD_V0816_0CZ_0DA_2026-10-11.md`。
- BACKLOG：0cz 批序 S3 勾达成（243 读数）＋0da 批序 S3 勾达成＋指针行（本批＝243、前批链插
  242）＋计数行＋P1 总览两片段＋开放项锚点两片段。
- BACKLOG 第二卷：§1.189。
- TODO：头行（本批＝243）＋P1 路由两片段＋P1-0cz S3 勾＋P1-0da S3 勾。
- 索引：头行 v4.215 → **v4.216**＋§6 两条目（0cz/0da S3 达成）＋§8 pending 桶两片段。
- orz 子仓：`316107f6` bump 批提交（**不推送**；落码提交 `39116a81` 同在本地）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json`
  刷新 0.8.16。

## §4 边界与如实记

1. **未推送未发行**：orz 两提交（`39116a81`/`316107f6`）与父仓各批均在本地；不建 Release、
   不做 rel-stage 打包；发布面停 v0.8.15（225 批）。
2. **构建为暖缓存增量**（WIN 3m53s／LIN docker `-j 1` 暖 /target）——非 clean 全量口径。
3. **0cz/0da S4 狗粮长轮实测待 0cy 卫生前置**（D7 五观察项＋0cz 既有判据）。
4. **MSYS docker 偏航（首跑）**：`-w /orz/orz` 被改写 `B:/Git/orz/orz` ⇒ daemon 拒绝、构建
   未执行、旧件未触（换装位 bak 预建于首跑前、逐位核对未动）；`MSYS_NO_PATHCONV=1` 重跑
   成立——与 218 批 ACAF MSYS 剥除同族（Windows 壳层参数形态），教训并入同族台账。
5. **`clear_all` LIN 基线＝1**：0.8.15 LIN 件中 `clear_all` 字节序列既有 1 处巧合命中（非
   工具面）——基线先行取样在案，判读以增量为准（1→28）。
6. Linux 载体不重 provision（沿 093…218 同口径）。

## §5 关键词

243 批、0.8.16 重建、源冻结 `316107f6`、0cz S3 进体、0da S3 进体、`db95ed15…` 身份门、
ACAF 重 provision `1919b56f…`、static-pie×3、alpine/bookworm 双冒烟 0.8.16、
`clear_all` 0→27/1→28、`context_manage` 0→3、MSYS docker 偏航、RUN-CLI-6acabdc9、
未推送未发行、计数 59 不变、v4.216。

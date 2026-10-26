# 360 批：Windows 0.9.14 重出件换装（358/359 进体）

> 日期：2026-10-26
> 用户令：「请帮助我进行换装」；版本裁决＝用户转述上会话回复（「cargo build -p orz-bin --release -j 2 成功（4m00s，exit 0）……--version：orz 0.9.14……安装位仍是旧件——尚未换装」）⇒ **直接以 0.9.14 重出件换装，不 bump**。
> 基线：父仓 HEAD `44440145`（docs(358)）＋ orz `568ea733`（358a/b/c＋审查修复，同树含 359 补批）。
> 计数：65 不变（纯载体换装，不新增项）。

## §0 结论

在役 Windows 载体位＋装机位由 357 的 0.9.14（orz `95faff3a`）升至 **0.9.14 重出件（orz `568ea733`：358 scout 只读子代理＋审查修复＋359 终态声明教学面清理；版本串未 bump）**。三件 release→载体位→装机位 cmp 逐字节 **MATCH 3/3**；ACAF 重 provision（`binary_sha256=2956ca23…`＝新 signer）；载体清单 v0.9.14 重生成 3 entries；doctor 载体位 `12/0/0`、装机位 `10/2/0`；活体 bridge/child 0.9.14、`version_skew=false`、五会话投影连续。

## §1 源与构建证据链

- orz `568ea733`（2026-10-26 21:05 提交）＝358a（`SubagentRole::Scout`＋只读工具投影白名单＋`spawn_scout` 路由深度 1）＋358b（`agents/scout.rs`：JSON 契约／≤2K 摘要／`scout-report/` 落盘）＋358c（capabilities 登记／`inspect children[]`／并发 2／预算 partial）＋审查修复（P0 ToolStarted 配对等）＋**359 补批同树**（NEUTRAL 自查块／deliver 实质文本轮）。
- 构建为上会话留存（21:06–21:10 完成；构建日志未留存——证据链沿 352 口径：下表哈希＋`--version`/`--build-info` 活体＋部署件逐位一致）。
- 在体特征核证（`grep -ao` 计数）：358＝`spawn_scout`×3、`ORZ_SCOUT_MAX_CONCURRENCY`×1；359＝NEUTRAL 块文案（「请对当前结论执行反例自查（三问）」×3、「这不是硬门。请据此修订或确认终答」×2）。`--version`＝`orz 0.9.14 (windows/x86_64/release)`，build-info 同。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,778,048 | `d9fd73872806f3ebfb285cd56a7bcc546cdc5607b79cebc86b99bfeeff2994ec` |
| `orz-signer.exe` | 6,740,480 | `2956ca2315b0b7d2e07d0f865fc84ecdc381e3919695ebba91f2a8321afccbb0` |
| `orz-acaf-provision.exe` | 6,644,736 | `90b80f9b81eb553589fb86a25d5e8c73a1769de9e581d0e5056823695e2f2c45` |

## §2 备份、换装与 ACAF

- 旧件对账：换装前载体位与装机位旧三件 sha256 逐位等于 357 账面（`d848bbef…`／`d959aa3b…`／`a974f14f…`），回滚基线成立。
- 备份：两侧各 `*.0.9.14-360-bak` 三件；旧 manifest `acaf/signer-manifest.json.bak-20261026-360`（回滚路径同 352 §2：停栈→成组还原→重 provision）。
- 停栈插曲：首检无 orz 进程；部署中途栈被拉起（装机位 `orz.exe`×2＋载体位 `orz-signer.exe`×1）致装机位 `orz.exe` 一次写入未遂（`cp` busy）——当时载体位 orz.exe 已入位、余五件未动（哈希逐件复核定位）；用户停栈后复检 0 进程，余件补齐，终态 6/6 MATCH 为准（无静默混装）。
- 换装：release→载体位→装机位 **cmp 逐字节 MATCH 3/3**（两侧各三件，共 6/6）。
- ACAF：以新 provision 件重 provision exit 0，`binary_sha256=2956ca23…` 与新 signer 逐位一致；用户级 env 三指针不变（`ORZ_ACAF_KEYSTORE/MANIFEST/BINARY` 均指载体位；keystore 幂等装载，未新建）；新旧 manifest 逐字段 diff 仅 `binary_sha256` 一处。

## §3 清单与 doctor

- 载体清单：`python scripts/generate_carrier_manifest.py D:\tb-eval\orz-windows --version 0.9.14` → 3 entries（`generated_at=2026-10-26T13:31:16Z`），三件哈希与部署件一致。
- doctor（各槽位目录内实跑）：载体位 `ok=12 warn=0 fail=0`（同 357）；装机位 `ok=10 warn=2 fail=0`——两 warn 环境性（该目录无 run journal；装机位无 `carrier-manifest.json`，沿 341/346/352「装机位本不放清单」口径）。357 曾读装机 `11/1/0`；本批换装前后装机位均无清单文件（实测），按基线同形如实计 `10/2/0`。

## §4 活体核证

目标工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹`，装机位 `orz.exe web --real`（日志 `D:\tb-eval\buildlogs\orz_360_web_stdout.log`／`orz_360_web_stderr.log`，监听 `127.0.0.1:21487`）：

- 启动 banner：`[permission] mode=interactive-yolo (auto-approve, source=default; benchmark axes off)`——§14.91 缺省姿态逐字核收。
- `/api/boot`：bridge `0.9.14`／child `0.9.14`／`child_probe=ok`／`version_skew=false`／cwd 为目标工作区／agent 为装机位件。
- `/api/conversations`：五会话——`51da03ea`、`54d12bcc`、`d24eadd6` 活跃，`073b8892`、`c23e0aaa` 已关闭，`legacy_short_id` 全 false；与 357 后已知状态连续。
- 旧标签页 token 失效，须用本次启动新 URL（token `eec520f1…`；343/357 同款注记）。

## §5 边界

- **0.9.14 重出件 ≠ GitHub Release v0.9.14 资产**（同号不同物；后者源 `95faff3a`，Release immutable）。对账以载体清单哈希＋本档为准；**下次发行前须 bump 0.9.15**（339 批「已发行版本号不重出」口径的本批豁免仅限本地在役，不外推到发行面）。
- 未推送、未发行；Linux 载体未动（仍 357 的 0.9.14，358/359 未进 Linux 载体）。
- 358 接续档 §7「账实同步」（BACKLOG/TODO 与 358 设计稿自记计数 66 的差异核对）仍待 358 收口批处理；本批不动索引（无召回路由变化，索引 §0.5 口径）。

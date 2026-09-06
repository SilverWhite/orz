# P0-0m S1+S2 复审处理审计（2026-09-07）

> 范围：S1 全面复审（用户指令「对当前实现的 S1 部分进行全面检查，包括设计
> 合理性、实现合理性、设计与实现的符合性」）发现问题的处置批。主代理直审
> （三路：设计合理性 / 实现质量 / 设计-实现符合性 + 对抗面专项推演）。
> 上游：[S1+S2 实施审计](P0_0M_GSA_SESSION_VOLUME_S1_S2_IMPL_AUDIT_2026-09-07.md)
> / 设计权威 [`GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06`](../GSA_SESSION_VOLUME_BOTTOM_LAYER_DESIGN_2026-09-06.md)
> / ADR-0010 §14.56。

## 1. 复审总结论

无 P0/P1。P2×2 + P3×6，全部处理（用户指令「对审查出的全部问题进行处理」）。
三路结论：判定模型健全、实现无正确性缺陷、D1–D5 逐条符合（D2 顺序句为
设计文本瑕疵，实现取唯一自洽解）；对抗面推演（尾点拼写/ADS/大小写/硬链接）
无新越读通道。

## 2. 逐项处置

### P2-1 设计文档 §3 D2 判定顺序句自相矛盾 → 设计文档勘误注

「先 workspace，后 session volume」字面短路序不可满足设计自身矩阵：workspace
判 false 即拒 ⇒ 矩阵 #6（symlink 卷挂载，canonical 落点在 cwd 外）永不成立；
「在 workspace 即放行」短路 ⇒ 破坏矩阵 #5（真实 `.gsa` 内部面
agent-invisible）。唯一自洽解 = 域归属分类先行（会话卷域为 workspace 覆盖
层、workspace 回退），即实现语义。处置：设计文档 §3 D2 加勘误注（分类优先
序 vs 字面短路序论证）；ADR-0010 §14.56 条 2 只列三个判定域、无顺序句，
不受影响。

### P2-2 permission.rs 镜像未消费 SessionVolume 资源（双计算残留）→ D1 规则单源函数

镜像 `access_in_scope` 为同步函数、桥构造在 toolset 异步面之外，接 Resources
实例不可行（tokio Mutex）。实质处置：D1 装配解析规则抽为共享单源函数
`orz_tools::types::resources::session_volume_canonical_root(cwd)`——
host `build_toolset`（装配 SessionVolumeRoot 注入）与 permission.rs 镜像
（`gsa_canon`）改共用，消除解析规则双计算漂移面。登记口径：`.gsa` 访问
语义单源权威 = orz-tools 窗口契约；镜像消费同一 D1 规则函数，权限层不裁撤
（用户裁决 2026-09-06）；OBS-PERMISSION-DUAL-IMPL 的 `.gsa` 面消解程度
如实表述为「规则单源 + 判定双载体（工具层权威 + 权限镜像冻结）」。

### P3-① grep/list_dir 域内 deny 文案沿用 workspace 语义 → 文案区分

grep（stderr 信封）与 list_dir（PermissionDenied 变体）对会话卷域 deny
改报 `... is inside the agent-invisible session volume: ...`，与 read_file
一致；workspace 越界文案不变。

### P3-② 矩阵 #8 工具级资源在场形态未直接测 → 新增工具级负测

`read_file_denies_gsa_symlink_to_non_volume_with_resource_present`：
`.gsa` symlink → 任意非会话卷目录 + SessionVolumeRoot 注入——幽灵白名单
形态（卷内无此文件，canonical 回退词法落点不在卷内）拒、卷内白名单外
（journal）拒，双断言均锁 session-volume deny 文案。

### P3-③ `..`-含路径进窗口缺显式测试 → 新增纯函数测试

`dotdot_paths_into_volume_judge_by_normalized_window_shape`：`..` 折叠后
= 精确白名单文件放行；折叠后落白名单外（journal）拒。

### P3-④ 尾点/ADS 变体仅分析核验 → 新增 cfg(windows) 尾点测试 + ADS 登记不改

`windows_trailing_dot_gsa_spelling_denied`：`.gsa.` 尾点拼写经 Win32 剥点
后 canonical 域臂命中卷 → 域归属成立；窗口词法形态不匹配 → 白名单外面与
白名单窗口文件**均拒**（fail-closed 可用性损失，安全无破口）。POSIX 下
`.gsa.` 为另一不存在的目录名，走 workspace 面 NotFound，无暴露面（测试
注记登记）。ADS 变体维持分析核验不改（extension 判定 + canonical 剥离使
其不可达白名单形态，读白名单文件自身的 ADS 无害）。

### P3-⑤ ADR §14.56 头部「实施未开始」滞后 → 登记更新

ADR-0010 §14.56 头部改登记：S1+S2 已闭合（orz `59eba7f2` + 本复审处理批），
S3/S4 待续，入口双审计。

### P3-⑥ 既有 `.gsa` symlink 拒读测试形态身份未澄清 → doc 注记

`read_file_rejects_gsa_symlink_resolving_outside_git_root_even_when_gitignored`
加形态注记：不注入 SessionVolumeRoot，行使资源缺席回退路径（矩阵 #11 形态）；
资源在场等价负测指向上条 P3-② 新测试。

## 3. 验证证据

- orz-tools `--lib`：types::resources 84（+2：P3-③④）、read_file 111
  （+1：P3-②）、grep 46、list_dir 40 全绿。
- orz-host permission 族 20 passed（镜像改接共享 helper 后零行为变化）。
- orz-host 全量（`--test-threads=1`，skip 挂死 1 项）失败集与 S1 批基线
  逐项 diff **完全一致**（存量 34 项，见实施审计 §3）。
- orz-loop `--lib` 全绿；workspace check 零警告；clippy 新增零告警；
  fmt/diff --check 净；门禁 Exit 0（manifest 重算）。

## 4. 遗留（不阻塞）

1. orz-host 存量失败 34 + 挂死 1（ACAF signer 族）——独立轮次（沿用
   实施审计 §5 登记）。
2. OBS-PERMISSION-DUAL-IMPL 整体收敛随终局治理视野（P2-2 处置已如实
   表述 `.gsa` 面消解程度）。

# 214 批：写控 `.gsa` 拦截建议面——会话卷拦截信封附备份/暂存指引（212 考虑项 i 落地；写控设计稿 v4.1）（2026-10-07）

> **用户令**：对 212 批登记的备份区考虑项裁决「加教学的话，如果将教学部分并进幻影提示呢？
> 在模型想要备份的时候询问是否想要或者需要/tmp/副本/黑板？」→ 主会话勘定落点（挂载点不在
> 0cs 派发期查表面，而在写控拦截信封——被拦本身即最可靠的意图信号）→ 用户裁「**我同意，
> 请落成小批吧，并将这一设计补充进写控拦截设计的设计稿中**」。
> **性质**：落码小批（exec_policy `block_message` 单点文案）＋设计稿 v4.1 增补；**计数不变
> 63**；未提交、未推送。

## §1 实现（exec_policy.rs 单点）

- `block_message()` 新增 `.gsa` 会话卷臂指引：`rule == "carrier-write"` 且 detail 含
  `session volume` 措辞时，在「已越过保底硬边界——命令未执行」之后追加固定一句——
  「若意图是备份/暂存：`/tmp` 与工作区写向不受本闸限制；运行状态可记 `blackboard_write`；
  `.gsa/rollback` 为运行时逐调用备份区，只读使用。」
- **仅会话卷臂携带**：keystore 根／signer manifest／祖先臂是秘密与载体保护、备份指引不对症
  （判别式＝detail 的 `session volume` 措辞；生产中 `.gsa/keystore` 由会话卷臂双覆盖先行，
  属会话卷拦截口径、携带无碍）。
- **定性**：0cs 同构（错误信封内给可执行替代物）；应答式非主动提醒（不涉 P9 隐式提醒面）；
  建议不询问（模型读信封自决）；不软化拦截（未执行语义在前）；单源 `block_message`（与
  规则 1 精准删除指引同点）；0cl 纪律一句；零契约面（WCR detail 与判官/fixtures 不动）；
  零新工具。

## §2 设计稿增补（v4.1）

`WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`：头行 `design v4.0` → **`v4.1`**、
沿革行补 v4.1；新增 **§8「`.gsa` 会话卷拦截建议面」**＝触发与来源（212 调查实证＋用户两步
裁决）／形态（措辞全文）／定性边界六条（应答式／不软化／不询问／单源／零契约面／零新工具）／
误伤面与预期收益（官方 5 起写向 5/5 命中；ttr/mtl 型得直接替代路径、l2m 型得 blackboard
指引）／钉位。

## §3 钉（先红后绿）

- 新钉 1：`block_message_session_volume_carries_backup_suggestion`——会话卷臂携带恰一处
  （0cl）＋`/tmp`/`blackboard_write`/rollback 语义字样在位＋「未执行」语义先于建议
  （不软化）＋keystore 臂（直接构造非会话卷 finding）不携带＋读向放行回归。
- 既有钉补负例 1：规则 1（catastrophic-recursive-delete）信封不含会话卷建议。
- **如实记**：首版钉用 `review_linux("cp /tmp/k /proj/.gsa/keystore/x")` 期望 keystore 臂
  拦截，实际由会话卷臂双覆盖先行命中（生产口径）——改为直接构造非会话卷 finding 验证
  block_message 的条件判别，测试意图更精确。

## §4 验证

- orz-tools lib 全量 **3024/0**（3023＋恰 1 新钉；6 ignored）；`block_message` 组 3/3、
  `gsa_` 组 19/19。
- clippy（lib＋tests）26 条全存量（与 212 时点同数）；fmt：`cargo fmt -p orz-tools` 净。
- 门禁 `check_repository.py`：本批面全绿（残余仅 orz submodule dirty＝204/209/212/214
  未提交的预期结果）。

## §5 台账

- 本档：`docs/audits/214_WRITE_CONTROL_GSA_BLOCK_SUGGESTION_FACE_2026-10-07.md`。
- 写控设计稿：v4.1（§8 增补＋头行/沿革）。
- TODO：`P1-0ct` 节备份区考虑项行注记「i 已随 214 落地」。
- BACKLOG：`0ct` 节注记；指针行/计数行（63 不变）。
- 索引：头行 → v4.186（并行窗顺延；206–213 编号竞用史见 §5 注）。第二卷 §1.161。

## §6 编号说明

206–211 编号被并行窗 SCB 系（210/211→空号）/0cv（208）与本批改号过程竞用占用：本批立项时曾误占 210/211 两号（即今 212 批），终号 214；沿 208/212 先例如实记。

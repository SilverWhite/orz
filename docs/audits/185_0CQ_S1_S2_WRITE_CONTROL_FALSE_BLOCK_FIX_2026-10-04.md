# 185 批：0cq S1 勘定＋S2 落码——写控误拦两族修复（2026-10-04）

> **用户令**：「接下来请进0cq S1/S2部分吧」＝0cq（写控误拦两族，182 批立项 57 → 58）S1 勘定与 S2 落码放行；S3（执行形态随裁：杂项狗粮轮同载 or 直接修码批）**不在本批**，载体重建亦未做（沿 184 批边界，本批双仓本地提交不推送）。
> **性质**＝勘定＋落码批；落点 `orz/crates/codegen/orz-tools/src/types/exec_policy.rs`（单文件）；契约面零变化（`BLOCK_RULES` 封闭集 5 条不动、schema v0.3 枚举不动、`write_control_review` 事件族形状不动）。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| S1 勘定 | 三条真机误拦（181 §4b）经 journal sha256 反查**全原文到手**，最小复现全数确证；**勘定勘误**＝三例**同根**，且 181 §4b 的两条机理描述均需修正（§1） |
| 同根结论 | **`path_candidates` 裸词空白拆片把 echo 散文撕成伪词元 × 规则 1/5 全局词扫描**：②③（`carrier-write`/`.gsa` 臂与卷根臂）的触发词元 `.gsa/usr)`、`/` 均为 `echo` 散文（`"(excluding .gsa/usr) =="`、`"=== csv / json / yaml / count / ids ==="`）拆片产物；①的 `/workspace/.gsa/*` 是 find 读排除模式**值位**词元（`-not -path` 之后） |
| 机理勘误 | 181 §4b 记「`-not -path` 读排除模式/引号内模式被词法提取」（①对）但「递归删除**不存在路径**经近祖先链展开至卷根」（③）**不成立**——真命令的删除目标 `t3demo`（裸名、无分隔符）根本不成路径候选；卷根误报来自 echo 散文拆片的裸 `/`。近祖先链展开机制存在但三条命令均未触达卷根 |
| S2 修复 | 四件（`exec_policy.rs` 单文件）：① **拆片收紧**＝裸词不再空白拆片（kv 等号右值保留）；② **规则 1 扫描精准化**＝删除动词与递归旗**同段**武装、仅扫**动词位之后**的本段词；③ **规则 5 arm 收窄**＝重定向目标位为 `/dev/null`/`NUL`/fd 数字不武装（纯读弃音槽不开扫描）；④ **规则 5 扫描段内化＋读模式值豁免**＝仅**写段**（程序位 ∈ 破坏/修改集∪`dd` 或段内含重定向词）参与扫描，[`READ_PATTERN_OPTIONS`] 值位词元与 kv 前缀闭集（`-path/-name/-regex/--exclude/--glob` 等 15+6）不作写目标 |
| 验证 | orz-tools **2997/0**（＋0cq 钉 7：三真机回归反例→不拦、`cd /` 非删除目标、null 重定向读 `.gsa` 放行、读模式值豁免、写段局部化；均带真阳性对照臂）；全原文核证＝三条真机命令 verbatim 注入审查：**Allow／Allow／warn-only**（§3）；orz-assurance 301/0、orz-loop 850/0/3、orz-host 350 过＋**5 失败＝冻结树同款先存**（负载敏感族如实登记）；clippy **26=26 零新增**；触碰面 fmt 干净 |
| 边界 | 不重建不推送（载体重建待 0cq S3 形态随裁＋用户令）；规则 6（broad-destructive warn）全局性不动（warn 留痕不阻断、不在 0cq 拦截面）；残余边界＝非 null 重定向命令全段扫描保守保留（「命中即拒、绝不因解析失败放行」charter 条款） |
| 计数 | **不变 58**（0cq 线内 S1/S2 勾选；S3 未勾） |

## §1 S1 勘定（三例逐条）

| # | 真机命令（sha 前缀） | 181 记录 | S1 勘定（实际机理） | 最小复现（修复前行为） |
|---|---|---|---|---|
| 1 | `7672ecc56ea3`（`cd /; ls…; find / … -not -path '/workspace/.gsa/*' 2>/dev/null \| head`） | `carrier-write`：target `/workspace/.gsa/*` | `-path` **值位**词元经引号剥除成候选（token 本体非拆片）；arm 面＝`2>/dev/null` 的独立 `>` 词无条件武装规则 5 | Block(carrier-write, 同 detail) |
| 2 | `9b93a0ea4d6c`（`…echo "== find test-ish (excluding .gsa/usr) =="; find …`） | `carrier-write`：target `.gsa/usr)` | echo 散文**整词候选**含空白 → **拆片**成 `(excluding`、`.gsa/usr)` 伪词元 → `.gsa` 包含命中 | Block(carrier-write, 同 detail) |
| 3 | `7d0e68698f45`（`cd /tmp && rm -rf t3demo && … echo "=== csv / json / yaml / count / ids ==="`） | `catastrophic-recursive-delete`：volume root (`/`)，181 归因「不存在路径近祖先链展开」 | **勘误**：删除目标 `t3demo` 无分隔符不成候选；`/` 词元＝echo 散文拆片产物；规则 1 动词（rm）＋旗（-rf）在场即**全局**扫词 → 裸 `/` 恰为卷根 → Block | Block(catastrophic-recursive-delete, 同 detail) |

**修法定案**（0cq S1 的两问）：①读语境豁免 vs 目标提取收紧——**分层并用**：读模式值位豁免（闭环①）＋拆片收紧（闭环②③根因）；②祖先链卷根 form 剔除 vs 精准化——**精准化**（规则 1 段内化），祖先链 form 剔除不需要（该机制本批未证误拦）。

## §2 S2 落码（四件细节）

1. **`path_candidates` 拆片收紧**：裸词臂只出整词候选（引号内空白＝路径合法字符，整词直接可用）；kv 等号右值臂保留拆片（`of=…` 类偶带空白的目标形态）。
2. **规则 1 段内精准化**：新增 `segment_prog_index`（镜像 `program_entries` 头部包装词/赋值跳过）；删除动词与递归旗**同段**才武装；扫描仅及动词位之后词——`cd / && rm -rf t3demo` 的头部 `/` 不再算删除目标，`rm -rf /`、`cd / && rm -rf /` 真阳性保留（钉）。
3. **规则 5 arm 收窄**：`is_nullish_redirect_target`（`/dev/null`/`nul`/纯数字 fd）；仅含此类重定向的命令不开目标扫描——顺带修复同族相邻面（`cat /proj/.gsa/log 2>/dev/null` 修复前被拦＝读 `.gsa` 日志窗口被堵，钉）。
4. **规则 5 段内化＋读模式值豁免**：`segment_is_write`（破坏/修改集∪`dd` 程序位或段内重定向词）；`READ_PATTERN_OPTIONS`（15 项）前词豁免＋`READ_PATTERN_KV_PREFIXES`（6 项）词元前缀豁免。真阳性保留：`find / -type f > /proj/.gsa/out.txt`、`grep -r foo /proj/.gsa/x > /proj/out.txt`、`cp x /proj/.gsa/y`（钉）。

## §3 验证明细

- **全原文核证**（临时探针跑毕即删）：三条真机命令 verbatim（c1/c2 全文、c3 触发片段）注入 `review_command_with(/workspace, …)`：**None（Allow）／None（Allow）／broad-destructive（warn-only，规则 6 既有留痕面）**——三例零 block。
- orz-tools **2997/0**（2991→2997：＋0cq 钉 7、−勘定探针 1）；既有 2991 测试零破坏＝旧全局扫描无任何在案依赖。
- orz-assurance 301/0（判官 `write_control_review` 族/`BLOCK_RULES` 封闭集钉不动）；orz-loop 850/0/3；orz-host 350 过＋5 失败＝**冻结树同款**（stash 实测，`run_tests_timeout_kills_process_tree` 等负载敏感族，0aq 在案如实登记）。
- clippy 26=26（触碰面零新增；`map_or→is_none_or` 随批采纳）；触碰面 fmt 干净（exec_policy 0 diff）。
- 门禁：orz 提交后源清单再生成＋`check_repository.py`（§5）。

## §4 边界与后续

- **S3 未动**（执行形态随裁：杂项狗粮轮同载〔0bs/0bd 同形〕or 直接修码批收口）；**载体重建未做**——修复进在役载体随重建批（用户令时序）。
- 残余边界（charter 声明 best-effort）：非 null 重定向命令全段扫描保守保留（如 `grep x /proj/.gsa/f > out.txt` 仍拦——重定向真实写盘时无法机械区分 grep 输入位）；`bash -c "…"` 脚本内命令覆盖不变（解析边界）。
- 123 批「零误拦」结论的勘误面随本批修复闭环：两族机制性误拦已修，S4 级真机复核（零误拦读数）归 0cq S3 执行形态。

## §5 台账

- 本档：`docs/audits/185_0CQ_S1_S2_WRITE_CONTROL_FALSE_BLOCK_FIX_2026-10-04.md`。
- TODO：`P1-0cq` S1/S2 勾选（S3 留未勾＋注记）；头部计数行指针（计数不变 58）。
- BACKLOG：`0cq` 节 S1/S2 条目；头部本批指针。
- BACKLOG 第二卷：§1.139 本批流水。
- 索引：头行 v4.156 → **v4.157**；`GAP-WRITE-CONTROL-FALSE-BLOCK` 条目进展更新。
- 提交：orz 子树本批（单文件＋钉）＋父仓（档＋台账＋清单）；**不推送、不重建**。
- 机械门禁：orz 提交后源清单再生成＋`check_repository.py` valid。

## §6 关键词

185 批、0cq S1 勘定、0cq S2 落码、写控误拦两族、散文拆片伪词元、path_candidates 拆片收紧、
规则 1 段内精准化、动词位之后扫描、规则 5 arm 收窄、null 重定向不武装、写段局部化、
读模式值豁免、READ_PATTERN_OPTIONS、机理勘误（近祖先链归因不成立）、全原文回归核证、
真阳性对照钉、计数 58 不变。

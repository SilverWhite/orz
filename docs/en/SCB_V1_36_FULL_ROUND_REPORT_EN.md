# SCBench V1 Official Round — Full-Round Report (36 problems / 196 checkpoints, k=1): Results and Friction Summary (2026-10-07)

> Type: round results and situation report (public-disclosure draft / internal close-out artifact;
> batch 207 deliverable).
> Scope: the six-day run window of 2026-10-05 – 2026-10-06 (D1–D6; all 36 problems exit 0, zero
> harness-side failures, zero reruns) plus the 2026-10-07 S3 close-out (full-set Harbor conversion,
> publication, and the 0bz S4 offline reconciliation).
> Authority chain: scoping and freeze (batch 196) → the six run days (batches 197–202) → close-out
> (batch 205).
> Per-problem readings here are **not second-hand**: every number is mechanically extracted from the
> per-problem `evaluation.json` plus deduplicated journals (RUN entries deduplicated by largest
> snapshot; extractor `scan_0cr.py`, artifacts `D:/tb-eval/scbench/0cr_official/readings_<problem>.txt`).
> Boundary: k=1 screening round; **not an official score submission** (the outward form is the Harbor
> friction-artifact publication, dual-tagged). Evaluation philosophy: the tasks are a tool for finding
> bugs and gaps; the score is a reference point only (batch 152 adjudication).

LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.

---

## 1. Headline summary

| Item | Reading | Notes |
|---|---|---|
| **Checkpoints solved** | **134/196 = 68.4%** | A checkpoint counts when its Core set is fully green; official judge (pytest oracle, pass_policy=any) |
| Fully-solved problems (all checkpoints) | **11/36 = 30.6%** | Easy 7 / Medium 2 / Hard 2 (mechanically recomputed from the per-day table; corrects batch 202's "6/1/4", see §9) |
| Full collapses (never solved from a 0-score checkpoint onward) | **3/36** | dag_execution 0/3, eve_jump_planner 0/3, eve_market_tools 0/4 |
| Near-miss family (1–2 checkpoints short) | ≥9 problems | circuit_eval, meshctl, sheeteval, file_merger, xjq, migrate_configs, file_query_tool, mvvault, mocked_http |
| Round duration | Six-day wall clock: per-problem sum **≈36.5 h** (batch-basis ≈41 h including harness overhead) | D1–D6 ≈5h00 / 5h30 / 4h40 / 5h42 / 7h29 / 8h09 |
| Scale | 4,593 model steps / 6,157 tool calls / 186 compressions | Whole-round totals (mechanically reproducible) |
| Official leaderboard v1.0 (reference) | Top = GPT 5.5/Codex 28.1% Isolated Solve; Opus 4.6/Claude Code 20.9% | **Not directly comparable** (§6); under the closest metric (isolated fully-passed share) this round's **30.6%** is directionally higher (§13) |
| Actual cost (DeepSeek console) | **¥157.49 ≈ $22.0** (National Day off-peak pricing throughout; hit 693.9M / miss 606.3M / output 87.5M) | 1/3.8 of the paper's cheapest run; $/CKPT 1/8.1 (§14) |
| Paper strict-pass reference | Opus 4.6 17% / GPT-5.4 11% / HumanLayer's Opus 5 retest 24% | As above |
| Main friction harvest | 0ct chartered + 0cq family ledger of 6 + 0cs first sample + 0bz S4 old "second needle" zeroed + 4 newly found reference-solution defects | §7 / §8 |

Three things that must be read together with the numbers:

1. **Single snapshot**: carrier 0.8.14 (identity gate `fc990a8a…`) with deepseek-v4-flash, unchanged
   throughout. Friction found during the round was **recorded, not deferred into the round**
   (in-service friction such as 0ct/0cq/0cs is honestly counted into the readings) — the score is
   the score of *this snapshot*, not a framework ceiling.
2. **k=1 matches the paper**: the official experimental setup states "we select a single run per
   model"; this repository's frozen protocol (seed 42 / pass_policy any / one_shot off / the full
   alphabetical set) is that same official form. Batch 206 correction: there is **no** "≥5 attempts
   per task" threshold — batch 152's sentence was a cost reminder during scoping, not a criterion.
3. **The point is friction, not the score**: zero harness-side failures and zero reruns across the
   round; all 9 write-control blocks autopsied individually (§7.1); the compression surface's old
   "second needle" went to zero across the whole corpus (§7.5) — the single most valuable mechanical
   result this round produced for the framework.

---

## 2. Protocol and setup

| Item | Value |
|---|---|
| Harness | Official slop-code-bench, pinned `main=31ceea3` (harness) + `38d627e` (scb-problems) |
| Problem set | Full set of 36 problems / 196 checkpoints, official discovery order (alphabetical); difficulty Easy 12 / Medium 12 / Hard 12; frozen manifest (`0cr_official/manifest.json`) |
| Official parameters | seed 42 (default) / pass_policy any / one_shot off (verified against source, point by point) |
| Model | `deepseek/deepseek-v4-flash` (unchanged for the whole round). **Interface-name note (batch 208):** `deepseek-v4-flash` is a legacy alias now served by DeepSeek-V4.1-Flash (Flash pricing); the current official name is `deepseek-flash`. Same base model, so readings here are unaffected |
| Carrier | orz 0.8.14, identity gate `fc990a8a…` (Windows + Linux, batch 194 rebuild) |
| Execution shape | k=1; one independent `slop-code run` per problem; a fresh agent container per problem holding one continuous session across all its checkpoints, with evaluation in a separate per-checkpoint container; fresh container between problems |
| Reading extraction | `scan_0cr.py` emits `readings_<problem>.txt` per problem (mechanical extraction from `evaluation.json` + deduplicated journals) |
| Upload | `scb_to_harbor.py` official pipeline, full-set conversion 36/36, plus `harbor publish --public` (`silverwhite/*`, dual-tagged `friction-hunting-artifact` / `not-official-scores`) |
| Positioning | k=1 screening round; friction-hunting as the main line; not an official score submission |

---

## 3. Final tally

### 3.1 Checkpoint level (134/196)

| Day | Problems | Checkpoints | Solved | Notes |
|---|---|---|---|---|
| D1 (10-05, batch 197) | ①–⑥ | 34 | **24/34** | cfgpipe 6/6 perfect start; dag_execution 0/3 first collapse sample |
| D2 (10-05, batch 198) | ⑦–⑫ | 27 | **19/27** | all-checkpoint results for env_manager / etl_pipeline / eve_industry (Hard) |
| D3 (10-05, batch 199) | ⑬–⑱ | 26 | **20/26** | all-checkpoint for eve_route_planner / execution_server / file_backup |
| D4 (10-06, batch 200) | ⑲–㉔ | 35 | **29/35** | all-checkpoint for forge / log_query / metric_transform_lang (Hard); meshctl (Hard) 7/8 |
| D5 (10-06, batch 201) | ㉕–㉚ | 37 | **20/37** | heaviest day (4 Hard), no all-checkpoint result; recli 3/8 (5th point of the variance series) |
| D6 (10-06, batch 202) | ㉛–㊱ | 37 | **22/37** | textdrop 6/6; test_translator 2/8 with the round's longest wall clock, 189 m |
| **Total** | **36** | **196** | **134 (68.4%)** | zero harness-side failures, zero reruns, 36/36 exit 0 |

### 3.2 Difficulty surface (mechanically recomputed from the per-day table)

| Difficulty | Problems | Checkpoints | Solved | Mean solve rate | All-checkpoint problems |
|---|---|---|---|---|---|
| Easy | 12 | 67 | **59** | 88.1% | **7/12** (cfgpipe, env_manager, etl_pipeline, execution_server, file_backup, forge, textdrop) |
| Medium | 12 | 57 | **34** | 59.6% | **2/12** (eve_route_planner, log_query) |
| Hard | 12 | 72 | **41** | 56.9% | **2/12** (eve_industry, metric_transform_lang) |
| Total | 36 | 196 | **134** | 68.4% | 11/36 |

The difficulty gradient follows the benchmark design (Easy → Hard decay). But the **Hard mean solve
rate of 56.9% is markedly above the paper's early-collapse shape across the board** (the paper's
strongest entry, Opus 4.6, is 17% under strict) — this suppression of "degrade as checkpoints
advance" through same-session continuity plus compression management is the round's most striking
result-side observation (causal attribution bounded in §6).

---

## 4. Per-problem results (36 problems, k=1)

Reading: "ckpt" = checkpoints solved / that problem's checkpoint count; "last core" and "last full"
= core / full test passes at the problem's final checkpoint; "wall clock" = per-problem start-to-end;
"steps/calls" = model steps / tool calls. ◆ marks a reference-solution-defect footnote problem
(oracle = tests, not the reference solution; run and reported as-is, see §8).

| # | Problem | Difficulty | ckpt | Last core | Last full | Wall clock | Steps/calls |
|---|---|---|---|---|---|---|---|
| 1 | cfgpipe | Easy | **6/6 all** | 3/3 | 206/216 | 55.6 m | 82/139 |
| 2 | circuit_eval | Medium | **7/8** | 15/17 | 564/566 | 80.7 m | 176/227 |
| 3 | code_search | Easy | 4/5 | 13/13 | 102/104 | 33.1 m | 86/149 |
| 4 | dag_execution | Hard | **0/3 collapse** | 0/3 | 33/51 | 34.9 m | 126/208 |
| 5 | database_migration | Medium | 3/5 | 1/3 | 123/137 | 39.6 m | 132/174 |
| 6 | datagate | Easy | 4/7 | 16/16 | 382/405 | 56.0 m | 156/214 |
| 7 | dynamic_buffer ◆(ck4) | Hard | 1/4 | 7/20 | 84/172 | 82.6 m | 192/188 |
| 8 | dynamic_config_service_api | Medium | 2/4 | 4/6 | 45/81 | 45.6 m | 110/218 |
| 9 | env_manager | Easy | **5/5 all** | 4/4 | 280/304 | 43.3 m | 110/161 |
| 10 | etl_pipeline | Easy | **5/5 all** | 4/4 | 158/164 | 32.8 m | 71/113 |
| 11 | eve_industry ◆(ck5) | Hard | **6/6 all** | 2/2 | 80/80 | 86.2 m | 231/266 |
| 12 | eve_jump_planner | Medium | **0/3 collapse** | 0/1 | 3/31 | 39.6 m | 125/144 |
| 13 | eve_market_tools ◆(ck1–3) | Hard | **0/4 collapse** | 3/8 | 14/75 | 55.3 m | 86/102 |
| 14 | eve_route_planner | Medium | **3/3 all** | 1/1 | 27/41 | 43.3 m | 89/88 |
| 15 | execution_server ◆(ck6) | Easy | **6/6 all** | 14/14 | 70/70 | 44.0 m | 81/194 |
| 16 | file_backup | Easy | **4/4 all** | 1/1 | 62/89 | 34.9 m | 99/134 |
| 17 | file_merger ◆(ck2–3) | Medium | 3/4 | 14/19 | 140/147 | 58.5 m | 120/196 |
| 18 | file_query_tool | Medium | 4/5 | 5/8 | 75/81 | 43.7 m | 128/147 |
| 19 | forge | Easy | **8/8 all** | 3/3 | 266/295 | 37.3 m | 95/150 |
| 20 | l2m | Easy | 3/5 | 4/6 | 40/64 | 42.7 m | 87/101 |
| 21 | layered_config_synthesizer | Medium | 1/4 | 7/9 | 83/98 | 53.7 m | 92/115 |
| 22 | log_query | Medium | **5/5 all** | 3/3 | 334/336 | 44.4 m | 147/229 |
| 23 | meshctl | Hard | **7/8** | 3/3 | 77/78 | 96.8 m | 111/229 |
| 24 | metric_transform_lang | Hard | **5/5 all** | 1/1 | 69/74 | 67.5 m | 189/259 |
| 25 | migrate_configs | Easy | 4/5 | 3/3 | 102/129 | 56.2 m | 138/179 |
| 26 | mocked_http | Hard | 6/8 | 3/3 | 168/200 | 106.5 m | 240/243 |
| 27 | mvvault | Medium | 4/6 | 5/5 | 40/42 | 53.1 m | 113/140 |
| 28 | pwd_manager | Medium | 1/5 | 4/6 | 179/257 | 41.3 m | 77/122 |
| 29 | recli | Hard | 3/8 | 5/7 | 153/255 | 113.9 m | 150/162 |
| 30 | rejector | Hard | 2/5 | 2/3 | 74/79 | 78.0 m | 97/94 |
| 31 | sheeteval | Hard | **6/7** | 1/3 | 127/164 | 121.6 m | 110/179 |
| 32 | sith | Hard | 3/6 | 14/21 | 199/228 | 73.8 m | 173/185 |
| 33 | test_translator | Hard | 2/8 | 42/50 | 827/2069 | **189.0 m** (longest) | 207/277 |
| 34 | textdrop | Easy | **6/6 all** | 3/3 | 182/183 | 34.1 m | 116/196 |
| 35 | trajectory_api | Medium | 1/5 | 2/3 | 127/373 | 40.2 m | 91/171 |
| 36 | xjq | Easy | 4/5 | 18/18 | 159/167 | 30.4 m | 60/64 |
| — | **Total** | — | **134/196** | — | — | **≈36.5 h** | 4,593/6,157 |

---

## 5. Result morphology (what we read beyond the score)

- **11 all-checkpoint problems** (§3.2 list): six of them are green through late checkpoints
  (forge 8/8, textdrop 6/6, eve_industry 6/6, execution_server 6/6, log_query 5/5,
  metric_transform_lang 5/5) — the benchmark's expectation of "late checkpoints must collapse" is
  fully falsified on 11 of 36 problems.
- **3 full collapses**: dag_execution (8/12 → 2/5 → 0/3, monotone collapse — the first real sample of
  the degradation shape the benchmark is designed to probe), eve_jump_planner (all three checkpoints
  failed; full stuck at 3/31 — the task body never got off the ground), eve_market_tools (◆ footnote
  problem, unsolved in itself). A 3/36 collapse rate is far below the paper's early-collapse shape
  across the board.
- **Near-miss family (1–2 checkpoints short, ≥9 problems)**: circuit_eval 7/8 (2 core tests lost at
  the last checkpoint), meshctl 7/8 (Hard, only ck4 short by 1), sheeteval 6/7 (six in a row then the
  last lost), file_merger 3/4 (◆ ck2–3 both perfect, then the last lost), xjq 4/5, migrate_configs
  4/5 (single test lost at ck2 only), file_query_tool 4/5, mvvault 4/6, mocked_http 6/8 (the
  best single-day result among dependency-heavy problems). The lost checkpoints cluster at the
  **final checkpoint** or are **single-checkpoint single-test** losses — direct evidence of long-range
  retention.
- **Degrading / collapsing family**: test_translator 2/8 (no recovery after the ck5 collapse; 2,069
  tests in the problem statement, the round's longest wall clock at 189 m — a capability-boundary
  sample), recli 3/8 (ck6 collapse matching its three historical runs — a mid-trajectory drift
  recurring), pwd_manager 1/5 / rejector 2/5 / trajectory_api 1/5 / layered_config 1/4 /
  dynamic_buffer 1/4 (each with a single-checkpoint collapse).
- **Non-monotone recovery**: datagate lost the first three checkpoints and steadied over the last
  three (4/7) — this round's counterexample to "collapse is irreversible"; mocked_http loses ck7 and
  recovers at ck8 in the same shape.
- **Variance reference**: recli's official-round 3/8 is the fifth point of a five-run series
  (2/8 → 3/8 → 3/8 → 8/8 → 3/8) — a quantified warning that k=1 results are bimodal; no single-problem
  reading here is a capability conclusion.

---

## 6. Official comparison and comparability boundaries

| Comparison line | Reading | Comparability |
|---|---|---|
| scbench.ai leaderboard v1.0 (19 models/agents) | Top GPT 5.5/Codex **28.1%** Isolated Solve; GPT 5.4 25.5%; Opus 4.6/Claude Code 20.9% | **Not directly comparable**: the leaderboard metric (Isolated Solve) is not aligned with this round's iterative checkpoint solved (pass_policy=any) |
| Paper strict pass | Opus 4.6 17% / GPT-5.4 11%; HumanLayer's Opus 5 retest 24% (4/17, all early checkpoints) | Directional reference only: this round's 30.6% fully-solved-problems is above every listed entry, but strict vs any is not aligned |
| This round, checkpoint level | 134/196 = 68.4% (any metric, same-session iterative) | Reference only — not for the leaderboard, not extrapolated |

Three boundaries: ① **k=1** — recli's five-run series (2/8 → 3/8 → 3/8 → 8/8 → 3/8) quantifies the
uncertainty of a single-run reading; ② **single snapshot** — in-service friction (0ct's `.gsa`
read-direction interception / the 0cq family's 5 write-control false blocks, etc.) is honestly counted
in, so a rerun after the fixes would be expected to rise, not fall, but this round makes no such
claim; ③ **what the model's price tier means** — deepseek-v4-flash is a flash-tier model, and the fact
that readings sit above frontier-model leaderboard entries is **attributed to the harness first**
(session continuity + compression management + blackboard), but with no same-model/different-harness
controlled experiment this round makes no single-cause claim.

**Outward form** = the Harbor friction-artifact publication (batch 205; dual tags + a README
non-official-scores statement), unchanged. **Submission-path correction (batch 208 re-verification)**:
officially there is **no documented submission process** (checked across README / FAQ /
docs/evaluation / docs/metrics / contributing — and no attempt-count threshold either, so batch 206
narrowed "no public channel" to "no documented process"). The **de facto route onto the leaderboard
is a public Harbor run** (the leaderboard page aggregates public runs by Model × Harness — "Showing
best version by % Checkpoints"; the problems themselves are distributed as a Harbor dataset). The
silverwhite conversion tasks are already published, so a future orz run through the Harbor runtime
with `--public` would constitute a leaderboard-visible entry — a new decision, pending user
adjudication; Discord is the human-coordination channel.

---

## 7. Friction ledger summary (main line; zero harness-side failures, zero reruns)

### 7.1 Write control (all 9 mechanical blocks autopsied)

Per-day review counts (`write_control_review` events): 314 / 970 / 334 / 372 / 509 / 438. All 274
warn cases were logged without blocking and without reaching the model surface (mostly prose-matching
false positives such as `rc=$?` and `*args`). There were **9 blocks**:

| # | Day / problem | Verdict | Family |
|---|---|---|---|
| 1 | D1 dag_execution | `//` path collapse → root-ancestor arm (conservative-arm semantics; a true positive) | 0cq family ① prose path collapse |
| 2–3 | D2 dynamic_buffer ×2 | `format` tokens inside heredoc code (`std::string format;` / a `"format"` key) picked up by verb scanning | 0cq family ② heredoc body |
| 4 | D2 dynamic_config_service_api | multi-target `rm` including `.gsa/rollback`; whole command rejected (legitimate parts rejected too — a designed cost of the conservative hard boundary) | by design (write-preserved surface) |
| 5 | D3 file_merger | bare `/` in single-quoted echo prose entered path candidates → root-ancestor arm | 0cq family ③ quoted prose |
| 6 | D4 l2m | writing `.gsa/runs/RUN-*/events.jsonl` (the journal itself) | by design (write-preserved surface) |
| 7 | D4 meshctl | `cp .gsa/rollback/*.bak /tmp/…` — a **read-direction copy** — was blocked | **escalated to 0ct** (§7.2) |
| 8 | D5 recli | `format` variable name inside a heredoc Python patch | 0cq family ② heredoc body (3rd case) |
| 9 | D6 test_translator | writing a self-made `.bak` into `.gsa/rollback/…` | by design (the in-service proof that 0ct preserves the write direction) |

**0cq family candidate ledger: 6 cases total** (prose path collapse 1 + heredoc body 3 + quoted prose
1 + `.gsa` read-direction 1, escalated to 0ct). The "flat prose shattering" shape fixed in batches
185/186 showed **zero recurrence** across the round (positive evidence that the fix works). Every case
cost the model exactly **one round of rerouting**, and no problem failed because of write control.
Candidate fixes (heredoc-body scan exemption / quoted-string token exemption / `//` normalization)
await user adjudication in later 0cq batches.

### 7.2 `.gsa` read-direction interception → 0ct chartered (the round's single most important friction)

In D4, meshctl's read-direction `cp` was blocked as a whole command. The user's ruling: "reading is not
writing… the `.gsa` ledger is fully open; fix this first after the 36 problems." → 0ct chartered
(60 → 61). Batches 204 landed S1/S2 (reads fully open, the two-stage gate turned confirmatory, and all
8 write-direction preservation nails passing); the S3 carrier rides the next rebuild batch. The
readings in this round include this in-service friction (single-snapshot discipline).

### 7.3 0cs tool-name suggestion (S3 live verification)

Exactly **1** `Tool not found` event all round (D2 dynamic_buffer: the phantom name `run_cmd` → the
envelope suggested `did you mean "run_terminal_cmd"?` → the model corrected on its **very next
action, one round**); zero samples on D1/D3–D6. Consistent with, and slightly better than, the batch
188 baseline (≈1 round of trial-and-error per occurrence without the suggestion surface), but n=1 is
not sufficient evidence (recorded as such).

### 7.4 RLI (live readings with the full 0.8.14 mechanism in service)

- **94 streak fires** = 91 single-channel, all carrying a forward-projection segment (≤329–343 B,
  within the 400 B budget) + 3 dual-channel without a projection segment (the first live sample of
  batch 184's "no projection segment on multi-fire" design nail: D4 layered_config `stall×3 slow×3`
  crossing on one line — a designed shape, not a defect). Fire density tracks difficulty
  (env_manager's perfect run: 0 fires; dynamic_buffer's collapse run: 5).
- 1,323 notices (spike entry / migration confirmed / coverage_gap / streak); the domain machine mostly
  oscillates normal ↔ low_progress (the same family as batch 188).
- **Consumption surface**: the 0ck annotation and the 0bd ⑦ threshold push surface were both in
  service; this round set no consumption-rate criterion (the 0cj line is closed and the criteria
  transfer is on the record), so the reading waits for the RLI promotion line (0am, suspended) to
  resume.

### 7.5 Compression surface and 0bz S4 (the round's most valuable mechanical result for the framework)

186 `context_compressed` events all round (mode = a mix of `model_selected` and
`context_scale_window`; zero mechanical forced compression). 0bz S4 offline reconciliation (batch 205,
all 244 deduplicated journals across the 36 problems):

- **All 137 of the +2 events diverge exactly at the tail slot** (stable ≡ count − 3), with **zero**
  divergences before the tail slot. Batch 192's fix nail
  (`d4_rerender_diverges_only_at_its_tail_slot`, moving the D4 mechanical block to the tail) holds
  **100%** across the official round's whole corpus; the pre-fix shape (full-prefix repricing at the
  head slot, +2 hit collapsing to ~12.8K) appears **0 times**.
- **Median +2 hit: 115,968 tokens** (min 43K, max 265K) — the whole history is cache-reused and only
  the tail slot's 3 messages are refreshed by design: the cache economics of a long-horizon session.
- 2 no-op compressions all round, zero whole-window divergence; 15 heuristic spontaneous-collapse
  candidates (mixed with legitimate head changes at checkpoint switches — not a hard criterion,
  recorded as-is).

### 7.6 Transport / permission / resources

`transport_retry`: **6** all round (D2 1 / D3 2 / D4 3), **all self-healed with no repeat**.
`resource_denied`: 0. The permission surface auto-allowed everything with zero denials across the
round. `counterexample_gate`: 196 (one per checkpoint, a designed constant).

### 7.7 Framework guidance surface (evidence that it works as designed)

The `&` background operator → `is_background=true` guidance, workspace-gate denials
(`grep`/`read_file` outside the workspace), and argument-shape denials (`search_replace` missing
`old_string`, etc.) were all model-side semantic failures or by-design guidance: **zero
framework-side faults, zero interfering friction**.

### 7.8 Harness events (none during the run window; all four during the S3 conversion window converged)

The six run days had **zero harness-side failures and zero reruns** (the harness-rerun clause never
triggered). Four events during the S3 close-out window (conversion/publication, not the run) are
recorded as-is: the problems list's CRLF line endings caused 34 empty runs (Windows `write_text` line
endings; converged on rerun), a Docker Desktop engine 500 (recovered by restart, 29.6.2), a loop 2
stop that killed dag_execution's oracle stage (`--force` rerun, rc=0), and two transient apt network
drops on forge (third round rc=0) — **none of which affects any run reading**.

---

## 8. Conversion-fidelity footnotes (a by-product of the full-set Harbor conversion, 36/36)

The official `scb_to_harbor.py` pipeline (pin `38d627e`) converted the full set: 34 problems fully
rc=0; 6 problems carry oracle-defect footnotes (rc=4 — **oracle = tests, not the reference solution**;
the footnote records conversion fidelity, not an agent score):

| Problem | Nature | Detail |
|---|---|---|
| dynamic_buffer / eve_market_tools | known (KNOWN_ISSUES) | expected footnotes, delivered as predicted |
| **env_manager** | **newly found** | ck3 strict 0.984 (reference solution short by ~2 tests; the agent's official round is 5/5 all green as a contrast) |
| **file_backup** | **newly found** | ck2 0.84 / ck3 0.68 (agent 4/4 all green as a contrast) |
| **mvvault** | **newly found** | ck5 strict 0.9946 (short by 1 test; agent 4/6) |
| **test_translator** | **newly found** | strict 0.69–0.75 across checkpoints (the reference solution fails two to three tenths of the tests; the agent's 2/8 corroborates the difficulty) |

The three KNOWN_ISSUES problems (eve_industry / file_merger / execution_server) did not surface at this
pin (rc=0) — fixed after `38d627e` (fix/harbor-port-issues-30-31) or environment-dependent. The four
problems where the agent is all green while the reference solution fails (env_manager / file_backup /
execution_server ◆ck6 / file_merger ◆ck2–3) are the accumulated evidence for the "tests are the
authority" clause. **The 4 newly found defects are upstreamable** (a candidate, pending user
adjudication).

---

## 9. Data integrity and ledger corrections (differences from earlier accounts)

Because every per-problem reading is mechanically reproducible (`readings_*.txt`), the earlier accounts
were checked against them and four places were corrected (**this document's numbers take precedence**):

1. **Batch 202's "Easy 6 / Medium 1 / Hard 4"**: inconsistent with the per-day table's difficulty
   column — mechanically recomputed as **Easy 7 / Medium 2 / Hard 2** (§3.2).
2. **Batches D3–D5's running count "9 / 12 / 12 all-checkpoint problems"**: counted "one checkpoint
   short" problems in; the final mechanical list is **11 problems** (batch 202 §1's named list governs;
   §3.2/§4 here recompute consistently).
3. **Batch 202's D6 total row "1567/2984"**: does not match the row-by-row recomputation of
   **1621/3184** (the row-by-row figures govern).
4. **"Six days ≈41 h"**: that is the batch basis (including launch/monitoring/collection and daytime
   overhead); the per-problem wall-clock sum is **≈36.5 h** (§4 table). Both bases are kept on the
   record.

---

## 10. Disclosure checklist

| # | Requirement | Status |
|---|---|---|
| 1 | k=1 screening-round positioning; not an official score submission; outward form = Harbor friction artifact (dual-tagged) | header / §6 |
| 2 | Single snapshot: 0.8.14 `fc990a8a…` + deepseek-v4-flash unchanged; no in-round fix deferral | §1 / §2 |
| 3 | Official parameters and problem order: seed 42 / any / one_shot off / full alphabetical set of 36 | §2 |
| 4 | Per-problem readings mechanically extracted and reproducible; differences from earlier accounts corrected and recorded | §4 / §9 |
| 5 | Footnote problems (reference-solution defects) run and reported as-is; oracle = tests, not the reference solution | §4 ◆ / §8 |
| 6 | All harness events recorded (none in the run window; four converged in the conversion window) | §7.8 |
| 7 | In-service friction disclosed as counted into the readings (0ct / 0cq / 0cs) | §1 / §7 |
| 8 | k=1 variance (the recli five-run series) and the not-directly-comparable statement | §5 / §6 |

---

## 11. Boundaries and non-extrapolation

1. k=1 single attempt: carries no variance information; recli's five-run bimodality (2/8–8/8) directly
   quantifies how much a single-problem reading can move.
2. Single snapshot: the readings are the combined result of "0.8.14 + deepseek-v4-flash + in-service
   friction (0ct/0cq/0cs)"; the post-fix shape was not measured, so no "framework ceiling" inference.
3. Official leaderboard / paper readings are not directly comparable (isolated vs iterative, strict vs
   any); "above the listed entries" is a directional observation only.
4. No controlled harness-attribution experiment (no same-model/different-harness run), so no
   single-cause claim.
5. Wall clock includes container start/stop and the evaluation stage, so it is not pure agent runtime;
   API cash is not booked by the harness.
6. 0cs consumption/recovery reading is n=1; this round set no RLI consumption criterion.

---

## 12. Reproduction entry points

| Purpose | Location |
|---|---|
| Frozen manifest (36 problems alphabetical + 196 ckpt) | `D:/tb-eval/scbench/0cr_official/manifest.json` + batch 196 §3 |
| Per-problem official readings (authoritative) | `D:/tb-eval/scbench/0cr_official/readings_<problem>.txt` (36 files; produced by `scan_0cr.py`) |
| Official multi-dimensional per-checkpoint records | each run directory's `checkpoint_results.jsonl` (strict/core/isolated pass rate + scb-check 0.1.3 verbosity/erosion); aggregate `0cr_official/multidim_aggregate.py` / `multidim_aggregate.json` (batch 210) |
| Consumption and billing breakdown | DeepSeek console per-day breakdown (user-provided; batch 215, recorded in §14: hit/miss/output + ¥157.49 actual charge) |
| Per-problem run logs | `D:/tb-eval/scbench/0cr_official/logs/<problem>_run1_0cr.log` |
| Journals and trajectory volumes | the 36-problem output tree `outputs/deepseek-v4-flash/orz_just-solve_none_2026100*/` (`.gsa/runs/RUN-*/events.jsonl` is the only machine-readable record) |
| 0bz S4 reconciliation artifacts | `0cr_official/bz_s4_round_full.txt` (per-compression lines) + `bz_s4_round_files.txt` (corpus list) |
| Harbor publication receipts | `0cr_official/publish.log` / `publish_dataset.log`; dataset `hub.harborframework.com/datasets/silverwhite/slopcodebench-friction` |
| Six-day run ledger | batches 197–202 (per-day §1 scorecard + §2 friction ledger) |

> This document is what batch 205's Harbor dataset README points to with "see round report"
> (in-repo path `docs/SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md`; English edition
> `docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md`).

---

## 13. Official multi-dimensional aggregate (offline back-accounting; batch 210, zero reruns)

> Data source = the official harness's own output: `checkpoint_results.jsonl` in each of the 36 run
> directories (per-checkpoint strict/core/isolated pass rate + the scb-check composite). Both the
> judge and the quality checker ship with the official pipeline and are collected as the run goes;
> this section is a pure offline aggregation (`0cr_official/multidim_aggregate.py`, semantics aligned
> to the official `metric.py` dataset-level aggregation). Alongside §4's binary checkpoint solved
> (any metric), these are the complete readings in the same dimensions as the official leaderboard.
> **"Multi-dimensional data exists as soon as the round finishes" also means: the multi-dimensional
> gap was a reporting-choice gap, not grounds for a rerun.**

### 13.1 Official metric definitions (source-verified, pin `31ceea3`)

- `strict_pass_rate` = share of all tests passed; `core_pass_rate` = share of the Core group passed;
  **`isolated_pass_rate` = share passed among the current checkpoint's own tests (Regression
  excluded)** (`metrics/checkpoint/extractors.py`).
- The official dashboard's binary checkpoint pass = strict perfect ∨ isolated perfect ∨ all tests
  passed (`dashboard/data.py` `passed_chkpt`).
- `verbosity` / `erosion` = `scb-check==0.1.3` composite scores (version-pinned, recorded per
  checkpoint; not comparable across versions).

### 13.2 Whole-round readings (36 problems / 196 checkpoints)

| Metric | Value | Official reference |
|---|---|---|
| **Isolated fully-passed share** (isolated_pass_rate ≥ 1.0) | **60/196 = 30.6%** | Top entry GPT 5.5/Codex "Isolated Solve" 28.1% — directionally higher under the closest available alignment (the leaderboard's aggregation formula is not documented; this is the most reasonable alignment, pending official confirmation) |
| Strict fully-passed share (≥ 1.0) | 44/196 = 22.4% | Paper strict: Opus 4.6 17% / GPT-5.4 11% — same shape |
| Core fully passed (any metric = §3's headline) | 134/196 = 68.4% | this repository's existing account |
| Isolated pass-rate mean (partial credit) | 79.8% | Not directly comparable with the binary metric; both are given to prevent misreading |
| Strict / core pass-rate means | 83.0% / 83.4% | as above |
| **Verbosity mean** (lower is better) | **0.289** | top entry GPT 5.5 0.269 / GPT 5.4 0.193 — same order |
| **Erosion mean** (lower is better) | **0.613** | top entry 0.494 / GPT 5.4 0.278 — higher, recorded as-is |
| Verbosity growth rate (per-checkpoint transitions) | 58.75% | the official degradation signal (the paper notes ~80% upward trajectories as the norm — this round is below that norm) |
| Erosion growth rate | 65.0% | as above |

### 13.3 Difficulty surface (pass-rate means)

| Difficulty | Checkpoints | Core | Strict | Isolated | Verbosity | Erosion |
|---|---|---|---|---|---|---|
| Easy | 67 | 96.3% | 90.6% | 91.1% | 0.285 | 0.552 |
| Medium | 57 | 80.2% | 80.9% | 76.1% | 0.259 | 0.622 |
| Hard | 72 | 73.7% | 77.6% | 72.1% | 0.322 | 0.666 |

### 13.4 Collapse-signal surface (isolated, first → last checkpoint, per problem)

- **Perfect first checkpoint, lower at the last** (the benchmark's "degrade with checkpoints" showing
  clearly): eve_route_planner 1.00 → 0.38, dynamic_buffer 0.93 → 0.18, dag_execution 0.85 → 0.00,
  file_query_tool 1.00 → 0.60, dynamic_config_service_api 1.00 → 0.56, database_migration 1.00 →
  0.70, l2m 0.98 → 0.62, recli 1.00 → 0.68, migrate_configs 1.00 → 0.67.
- **Reverse (the last checkpoint rises)**: env_manager 0.36 → 0.88, file_backup 0.59 → 0.86,
  mocked_http 0.83 → 0.94, etl_pipeline 0.90 → 1.00, cfgpipe 0.89 → 1.00 — under the official metric,
  non-monotone recovery holds too (mutually corroborating §5's datagate shape).

### 13.5 Boundaries

1. The official "Isolated Solve" leaderboard aggregation formula is not pinned in any documentation
   (the metrics docs at our pin have no isolated entry) — the 13.2 comparison uses "isolated
   fully-passed share" as the most reasonable alignment, **pending official confirmation**; both mean
   and binary metrics are given in full to prevent misreading.
2. Erosion/verbosity come from the scb-check 0.1.3 rule set and are not comparable across versions
   (the version is recorded per checkpoint).
3. This section is an offline aggregation: zero new runs, zero model calls; the aggregation script and
   its output live outside the repo under `0cr_official/` (see §12).

---

## 14. Cost and consumption (DeepSeek console breakdown, reconciled; batch 215, zero reruns)

> Data source = the DeepSeek official console's per-day breakdown (user-provided, 2026-10-07):
> **hit 693,859,598 / miss 606,333,056 / output 87,526,542** (of the console's four columns, the last
> two are both output sub-items), totalling **1,447,719,196 tokens**; actual charge **¥157.49** (the
> run window 10-05 – 10-07 fell entirely inside the National Day holiday off-peak pricing; official
> off-peak rates: hit $0.003 / miss $0.15 / output $0.60 per 1M). The harness's own cost/token fields
> are all zero (orz does not report them back), so the console is the only authority.

### 14.1 Consumption structure

| Column | Tokens | Share of total | Share of bill (off-peak rates) |
|---|---|---|---|
| Input (cache hit) | 693,859,598 | 47.9% | **1.4%** (¥2.08) |
| Input (cache miss) | 606,333,056 | 41.9% | **62.5%** (¥90.95) |
| Output | 87,526,542 | 6.0% | **36.1%** (¥52.52) |
| Total | 1,447,719,196 | — | **¥145.55 (computed) / ¥157.49 (charged)** |

Efficiency readings: **315.2K tokens per step** (input 283.1K + output 19.06K — the output side is the
flash model's thinking-chain shape), **40.2M tokens per problem**; cache hit is **53.4%** of input.

### 14.2 Bill reconciliation and one documentation discrepancy

- Recomputed at the official off-peak rates (USD figures charged as the same numeric value in RMB):
  ¥2.08 + ¥90.95 + ¥52.52 = **¥145.55**; charged ¥157.49 = **×1.082** (+8.2%, billing granularity /
  rounding scale) — **the reconciliation holds**.
- Discrepancy on the record: the official Chinese documentation page's rates (miss ¥1–2 / output
  ¥4–8 per 1M) are about 6× off from what the console actually charged — the console governs; that
  documentation page needs checking (the console's line-item unit-price column would settle it).
- Back-solved effective unit prices ≈ hit ¥0.0035 / miss ¥0.176 / output ¥0.71 per 1M.

### 14.3 What makes up the 46.6% miss share ("hit and miss of the same order")

Miss is not cache failure; it is three sources combined: ① **the irreducible lower bound of each
step's new segment** — the API is stateless and every request resends the whole context, so the tool
results and messages appended step by step in a long session are all billed as miss; ② **the prefix
surgery of 186 compressions** — after a compression or blackboard rewrite that segment is re-billed
as miss (the 0bz tail-move fix has already pushed that cost down to "only the tail slot's 3 messages";
without it the figure would be higher); ③ **36 cold starts, one per problem**. Hit and miss of the
same order is the normal reading for a "long session + frequent context surgery" shape; a miss share
well above half (say 70%+) would be the signal of a broken cache mechanism.

### 14.4 Cost performance (against the paper's Table 1)

| Metric | This round | Paper reference (Table 1) |
|---|---|---|
| Per run | **¥157.49 ≈ $22.0** | cheapest GPT 5.3 Spark **$84.46**; field range $84–423 → **1/3.8** |
| $/CKPT (total cost ÷ 196 checkpoints) | **$0.112** | cheapest $0.91 (GPT 5.3 Spark) → **1/8.1**; most expensive $4.55 → 1/40 |
| $/isolated fully-passed checkpoint | $0.37 (60 checkpoints) | — |

One sentence on the cost shape: **cache discounting turns 53% of the input into 1.4% of the bill (a
4.7× effective discount); the money goes to miss segments and deep-thinking output**. The cash cost is
1/8 of the paper's cheapest entry (k=1, flash price tier, single-snapshot protocol). The "$500/run"
figure comes from the official contributing page, not the paper (the paper's measured $84–423 is the
same order of magnitude).

### 14.5 Boundaries

1. The console breakdown is user-provided and transcribed without independent verification (the
   harness-side metering is zero); the fourth column is a second output sub-item (the two output
   columns sum to 87.5M).
2. The exchange rate 7.15 is approximate; the National Day off-peak pricing applied to the whole run
   window (10-05 to 10-07 all within the holiday).
3. The ~6× discrepancy between the official Chinese documentation page's rates and the actual charge
   is unresolved (pending the console's unit-price column); the "$500" figure is a contributing-page
   number only and is not used as a bar.

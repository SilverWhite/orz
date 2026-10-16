# SCBench V4 Four-Way Full-Round Report (r4 clean-room official-protocol round · r3／0cw／0cr comparison; 36 problems／196 checkpoints, k=1) (2026-10-16)

> **Type**: round-results and attribution final-verdict report (**0dc S4 deliverable**, batch 269).
> **Scope**: r4 = 2026-10-12 13:25 – 2026-10-16 12:14 (clean-room official-protocol round, 36/36
> problems fully completed); the three comparison rounds = r3 (2026-10-11/12, **stopped at 14/36**,
> /tmp instruction-leak form), 0cw (2026-10-08/10, 36/36, **.gsa cross-conversation leak · hybrid
> form**), 0cr (2026-10-05/06, 36/36, continuous-session form).
> **Authority chain**: initiation [`batch 253`](../audits/253_R3_FULL_ROUND_STOP_TMP_LEAK_R4_PROJECT_2026-10-12.md)
> → S1/S2 [`254`](../audits/254_0DC_S1_S2_TMP_INSTRUCTION_BURN_AND_SURFACE_ASSERTION_2026-10-12.md)
> → S3 launch ([`255`](../audits/255_R4_S3_RUNNER_START_2026-10-12.md)／[`265`](../audits/265_SCB_R4_RESUME_2026-10-13.md))
> → close-out and incidents [`batch 268`](../audits/268_R4_DISKFULL_ENGINE_DEATH_AND_INFRA_GUARDS_2026-10-14.md)
> + [`r4 full-round score analysis`](../audits/R4_ROUND_SCORE_ANALYSIS_2026-10-16.md) → S4 = this document.
> Comparison baselines = [`SCB V1 official-round full-round report`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md) (0cr)／
> [`SCB V2 reset-form full-round report`](../SCB_V2_RESET_FORM_FULL_ROUND_REPORT_2026-10-11.md) (0cw, including the
> batch-235 contamination-correction note).
> **Machine readings**: `D:/tb-eval/scbench/v4_four_way_readings.json` (the mechanical recomputation output behind
> every aggregate number in this document; sources = `jobs-r4/r4_checkpoint_ledger.jsonl` +
> `jobs-r3/r3_checkpoint_ledger.jsonl` + `jobs-0cw/s3_checkpoint_ledger.jsonl` +
> `0cr_official/merged_checkpoint_results.jsonl`).
> **Boundary**: k=1 on all four sides; the score is a by-product and friction is the main line (batch 152
> adjudication); **not an official score submission**; **this document is the SCB line's closing verdict**
> (user decree 2026-10-16, "SlopCode Bench ends here").
> **Chinese authority**: [`docs/SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md).

LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.

---

## 1. Headline summary (four-way in one table; one common definition)

| Item | **r4 clean-room official form** | r3 (/tmp leak, 14 problems) | 0cw (.gsa leak · hybrid) | 0cr (continuous form) | Paper／leaderboard (reset queue) |
|---|---|---|---|---|---|
| Problems／checkpoints | **36／196** | 14／68 | 36／196 | 36／196 | 36／196 |
| Checkpoints solved (core fully green) | **144/196 = 73.5%** | 50/68 = 73.5% | 131/196 = 66.8% | 134/196 = 68.4% | Core max 67.3% (Opus 4.6); GPT 5.5 66.8% |
| Strict fully-passed checkpoints | **66/196 = 33.7%** | 29/68 = 42.6% | 61/196 = 31.1% | 44/196 = 22.4% | 14.8% (best: GPT 5.5) |
| Isolated fully-passed checkpoints | **88/196 = 44.9%** | 37/68 = 54.4% | 82/196 = 41.8% | 60/196 = 30.6% | 28.1% (best: GPT 5.5) |
| Partial (≥1 checkpoint strictly fully passed) | **21/36 = 58.3%** | 8/14 = 57.1% | 20/36 = 55.6% | 18/36 = 50.0% | 50.0% (best: GPT 5.5) |
| core mean (official dataset semantics = problem-equal weight) | **0.8846** | 0.8496 | 0.8235 | 0.8343 | — |
| strict mean | **0.8573** | 0.8571 | 0.8275 | 0.8303 | — |
| isolated mean | **0.8416** | 0.8216 | 0.8021 | 0.7976 | — |
| **End-to-end full solve** (all checkpoints strictly perfect = the paper's "no agent fully solves" definition) | **4/36** | 3/14 | 5/36 | 1/36 | **0/36** |
| All-checkpoint core perfect (broad definition; the V1／V2 "problem-level all-checkpoint solved" column) | 16/36 = 44.4% | 8/14 = 57.1% | 15/36 = 41.7% | 11/36 = 30.6% | — |
| Phase curve Core (paper five-phase mapping, Start→Final) | **94.3%→85.4% (−9.0pp)** | 87.3→80.9 (−6.4pp) | 92.0→74.5 (−17.5pp) | 94.0→77.7 (−16.3pp) | body macro 64.6→**35.5**; **figure 65.9→32.3** (§4.3.1) |
| Core／Iso ratio (Start→Final) | 1.06→1.04 (**no widening**) | 0.98→1.11 | 1.04→0.98 | 1.06→1.03 | **2.5×→5.4× (widens)** |
| Final-checkpoint perfect／final-checkpoint zero | **21／0** | 9／1 | 20／5 | 18／2 | — |
| erosion mean | 0.536 | 0.578 | 0.525 | 0.613 | 0.49 (best: GPT 5.5) |
| verbosity mean | 0.343 | 0.312 | 0.304 | 0.289 | 0.32 (best: GPT 5.5) |
| erosion net-rise trajectories (final > first) | **27/36 = 75.0%** | 10/14 = 71.4% | 21/36 = 58.3% | 29/36 = 80.6% | **77%** |
| verbosity net-rise trajectories | **28/36 = 77.8%** | 12/14 = 85.7% | 21/36 = 58.3% | 28/36 = 77.8% | **75.5%** |
| Checkpoint-level transition rate, erosion／verbosity | 54.4%／56.2% | 61.1%／55.6% | 48.8%／47.5% | 65.0%／58.8% | — |
| Cost | ¥215.69 | ≈¥77.8 (including aborted artifacts) | ¥212.41 | ¥157.49 | $/CKPT best 0.66 (GPT 5.3 Codex) |
| ¥／checkpoint | 1.10 | 1.14 | 1.08 | 0.80 | — |
| Checkpoint-time total／per checkpoint | 50.4h／15.4 min | 17.5h／15.4 min | 42.7h／13.1 min | 31.3h／9.6 min | 6.0±2.4 min (best: GPT 5.5) |
| Write-control block／transport_retry／run_failed | 52／24／2 | 13／6／0 | 53／105／20 | 9 (V1 §7.1)／—／0 | — |

**Definition note (corrected by this batch)**: the paper's "collapse" is a **five-phase queue mean**
(15 models × 36 problem trajectories pooled; paper §3 states explicitly that "raw checkpoint indices
are not directly comparable across problems", hence first checkpoint = Start, last checkpoint = Final,
and the middle split into thirds as Early／Mid／Late), **not a per-problem collapse curve**; the paper's
**per-trajectory** definition is used only for the quality axis (erosion 77%／verbosity 75.5%). The
phase rows above have been recomputed for all four rounds under that same mapping — see §4.3／§4.6 and
§6 propositions 4／11.
**(batch 271 addition)** That paper figure has since been **digitized point by point from the arXiv
source vectors**: the figure's three curves are Core 65.9→32.3／Iso 26.5→6.0／**Strict 26.5→1.3**
(four-way cross-validation in §4.3.1); the paper body's macro surface gives Core 64.6→35.5 separately
(the two surfaces differ slightly; this document takes the figure surface as authoritative). **For the
leaderboard definition note** see the "definition note (batch 271)" at the end of §6.

**In one sentence**: the clean-room form (r4) is the best on every axis of the **same-problem-set
(36 problems) three-round comparison** — score (core 0.8846, **+0.0503** over the runner-up 0cr
0.8343), fully-passed definitions (strict 33.7%, isolated 44.9%, both above 0cw's 31.1%/41.8% and
0cr's 22.4%/30.6%), and phase (94.3%→85.4%, **0 problems score zero at the final checkpoint**);
on the quality axis (erosion 0.536, second-lowest of the three rounds; **verbosity 0.343, highest of
the three rounds = verbosity is the only axis on which this round is worse than the comparison**). r3 covers
only a 14-problem subset; its higher strict 42.6%／isolated 54.4% ratios come from a **different
problem set and do not enter the comparison** (it serves only as a leak-form baseline).
**Two of the paper's core claims are reversed by this round**: ① "no agent fully solves any problem
end-to-end" — under the paper's **own definition** (all tests including regressions, all checkpoints
passing) the four rounds solve **4／5／3／1** problems end-to-end (the broad "all-checkpoint core
perfect" reading gives 16／15／8／11 and is not the target of this proposition); ② "reset means
collapse" — all four rounds' Core phase curves decline only gently (drops of 6.4–17.5pp, final
checkpoint 74.5–85.4%), whereas the paper's queue falls 29.1pp under the same definition and its
**Core/Iso ratio widens from 2.5× to 5.4×**, while all four rounds hold that ratio near **1.0
throughout, with no widening**.
Only the **quality-degradation axis agrees with the paper**: erosion net-rise trajectories 75.0% (paper
77%), verbosity 77.8% (paper 75.5%) — the separation phenomenon of "scores hold while quality still
degrades" is reproduced in this round.

---

## 2. Definition and setup (four-way)

| Item | r4 | r3 | 0cw | 0cr |
|---|---|---|---|---|
| Session form | **fresh process + fresh session per checkpoint** (clean room) | same (but with the /tmp leak) | same (but with the .gsa leak) | continuous session (one run spans checkpoints) |
| Workspace | persistent across checkpoints (official definition) | same | same | same |
| Leak channel | **none** (S1 both-side burn + S2 per-checkpoint assertion) | `/tmp/orz-instruction-{N}.md` accumulating across checkpoints (hit on 14/14 problems) | `.gsa` session volume／blackboard ledger readable across checkpoints (ledger read back at 188/189 checkpoints, prior-checkpoint session read back at 82) | none (continuity itself is the form) |
| Model | `deepseek-flash` (V4.1-Flash) | same | same | `deepseek-v4-flash` (same base model, `0cv` interface-name verification in batch 208) |
| Carrier | orz 0.9.0 | 0.9.0 | 0.8.15 | 0.8.14 |
| Parameters | k=1／zero retries／7200s per-checkpoint cap (**never triggered**)／`--upload --public` per problem | same | same | k=1／no cap (test_translator 189m) |
| Judging | official metric.py (pytest oracle, pass_policy=any); per-checkpoint `verifier/reward.json` eight metrics | same | same | same |
| Checkpoint-count consistency | 196 checkpoints (the p18 incident run handled under dual definitions) | 68 checkpoints (14 problems) | 196 checkpoints | 196 checkpoints |

**Comparability discipline**: the **only intended difference across the four rounds is form and leak
surface**; confounding factors on record = carrier differences (0.8.14→0.9.0) and cap differences (0cr
had no cap). r3 and 0cw are both **leak forms**, so the two are A/B comparable; r4 and 0cr are
separated by a whole carrier generation, so no single-cause attribution is made between them. Against
the paper the comparison has **three incomparabilities** = harness (native CLI vs orz) + model price
tier (flash vs frontier) + k and budget surface; the whole table supports **directional** reading only.

---

## 3. r4 closing tally (main round)

- **Completion**: all 36/36 problems `completed rc=0`; 37 hub jobs (36 final states plus the coexisting
  p18 incident artifact); zero errored; 36/37 rows accepted (only the p18 incident run contributed 1
  row, handled under dual definitions).
- **Dual definitions**: standard definition (p18 taken as the rerun 0.975) gives problem-level **0.8846**／
  checkpoint-level 0.8968; hard-strict definition (p18 = [1.0＋0×4]/5 = 0.2) gives problem-level **0.8631**／
  checkpoint-level 0.8770. **This report uses the standard definition throughout** (the settlement
  definition, user adjudication 2026-10-16).
- **Cost**: official definition ¥215.69 (= ledger ¥217.04 minus the ¥1.35 p18 aborted-incident row);
  checkpoint-level time 50.4h, problem-level wall clock 58.6h, 13,940 rounds; spend before 10-14 was
  fully refunded through the DeepSeek compensation credit.
- **Incident ledger (three incidents, three fixes)**: ① 04:53 the C drive filled → the Docker engine
  self-terminated (p18/ckpt2 frozen with the VM for 113min, hitting the 7200s per-checkpoint cap) ⇒
  launcher three infra guards + self-healing mount wait; ② 13:22 an incidental codex reflash killed the
  runner by pointing at its PID ⇒ supervisor stale-process cleanup and self-heal; ③ the supervisor's
  first version "exited as soon as the window closed", leaving the 10-15 midday window idle ⇒ changed to
  distinguish "window close-out" from "problem list finished". Details in
  [`batch 268`](../audits/268_R4_DISKFULL_ENGINE_DEATH_AND_INFRA_GUARDS_2026-10-14.md).

---

## 4. Four-way aggregate comparison (four layers)

### 4.1 Layer 1 · per checkpoint (fully-passed definition)

| Definition | r4 | r3 | 0cw | 0cr | Difference (r4 − best comparison) |
|---|---|---|---|---|---|
| Checkpoints solved (core fully green) | **144/196 = 73.5%** | 50/68 = 73.5% | 131/196 = 66.8% | 134/196 = 68.4% | **+10 checkpoints** (vs 0cr) |
| Strict fully passed | **66/196 = 33.7%** | 29/68 = 42.6% | 61/196 = 31.1% | 44/196 = 22.4% | +5 checkpoints (vs 0cw; r3's ratio is higher but covers only 14 problems) |
| Isolated fully passed | **88/196 = 44.9%** | 37/68 = 54.4% | 82/196 = 41.8% | 60/196 = 30.6% | +6 checkpoints (vs 0cw) |
| Partial (≥1 checkpoint strictly fully passed) | **21/36 = 58.3%** | 8/14 = 57.1% | 20/36 = 55.6% | 18/36 = 50.0% | +1 problem (vs 0cw) |

**How to read it**: the fully-passed definitions (strict／isolated) sit above the continuous form for
two consecutive rounds starting from 0cw, and this round raises them further; the morphological
conclusion "a fresh session per checkpoint ⇒ losses land more cleanly but recovery is faster" holds
across all four rounds.

### 4.2 Layer 2 · per problem (core mean; the five-tier scale is in §5)

| Pairing | Rise／flat／fall | Largest rise | Largest fall |
|---|---|---|---|
| r4 vs 0cr | **14 up, 13 flat, 9 down** | eve_jump_planner +0.667, trajectory_api +0.442, eve_market_tools +0.375, recli +0.296 | layered_config_synthesizer −0.347, eve_industry −0.167, rejector −0.140 |
| r4 vs 0cw | **12 up, 15 flat, 9 down** | meshctl +1.000, recli +0.675, test_translator +0.388, env_manager +0.350, eve_route_planner +0.333 | dynamic_buffer −0.206, sith −0.179, eve_industry／eve_jump_planner −0.167 each |

**Extremes coexist on both sides** (+0.667 and −0.347): the round-to-round total-score gap is a **net
figure from violent per-problem variance cancelling out**, not a small uniform difference.
**Least stable samples** = meshctl (four rounds 0.958／—／0.000／1.000) and recli (0.704／—／0.325／1.000) —
quantified samples of k=1 single-run uncertainty; no single-problem reading supports a capability claim.

### 4.3 Layer 3 · phase (paper five-phase mapping; headline)

**Mapping definition (paper §3, verbatim)**: raw checkpoint indices are not comparable across problems
(each has 3–8 checkpoints), so the first checkpoint = Start, the last = Final, and the rest split evenly
as Early／Mid／Late; the paper's figure 2 left is the **cross-model queue mean** under exactly this
definition.

| Round | Core Start→Early→Mid→Late→Final | Iso same order | Strict same order |
|---|---|---|---|
| **r4 clean room** | **94.3 → 91.3 → 92.2 → 86.3 → 85.4** | 89.3→90.6→89.5→80.9→82.5 | 89.3→90.7→88.9→86.8→82.0 |
| r3 (14 problems) | 87.3 → 92.6 → 90.5 → 89.6 → 80.9 | 88.7→96.7→89.3→85.3→72.9 | 88.7→96.2→90.0→88.1→80.9 |
| 0cw | 92.0 → 82.7 → 82.7 → 76.6 → 74.5 | 88.2→82.9→79.1→72.3→76.3 | 88.2→83.4→78.3→78.6→78.4 |
| 0cr | 94.0 → 88.7 → 86.7 → 81.7 → 77.7 | 88.8→87.8→83.4→77.9→75.1 | 88.8→89.2→85.7→82.5→78.6 |
| Paper queue (15 models) | **64.6 → … → 35.5** (the paper gives no intermediate phase readings) | Start≈25.8→Final≈6.6 (**back-derived from the paper's 2.5×→5.4× ratio**) | not given |

**Three structural differences**:

1. **Magnitude**: the four rounds' Core drop is 6.4–17.5pp (starting at 87–94%); the paper drops
   29.1pp (starting at 64.6%) — even at the first checkpoint more than a third of the paper queue's
   checkpoints fail functionality the problem statement spells out, placing it in a low band
   **dominated by cascading failures**.
2. **Class structure**: the same paper paragraph gives Functionality 62.7%→57.7% (−5.0pp) and Error
   80.1%→62.2% (−17.9pp) ⇒ the paper's collapse is **not concentrated in the current-spec functionality
   surface** but in the error class and the regression chain (the paper itself notes "a mistake at C2
   can zero out later checkpoints"); its **Core/Iso ratio 2.5×→5.4×** means the hidden functionality
   surface collapses far harder. Across the four rounds that ratio stays near 1.0 throughout
   (1.06→1.04／0.98→1.11／1.04→0.98／1.06→1.03) = **no relative collapse**.
3. **Multi-phase non-monotonicity**: r4's Core／Iso／Strict curves all sit higher at Mid than at Early
   (91.3→92.2 and so on), so the queue level is not monotone either; the four rounds show **later
   rebound** inside the trajectory for 47%／14%／36%／50% of problems respectively (§4.6).

**Final checkpoint**: r4 has 21 perfect and 0 zero; 0cw 20／5; 0cr 18／2; r3 9／1.

#### 4.3.1 Vector per-phase readings of the paper's figure 2 left (digitized in batch 271; not pixel eyeballing)

The paper's arXiv source (e-print, including the vector figure `figures/overview_solve_cost.pdf`) has
been retrieved, and **parsing that figure's content stream directly** gives the **exact five-phase
coordinates** of the three curves (y-axis calibration: tick labels equally spaced at 27.005pt = 20pp,
zero taken automatically from the plot-box bottom edge (y≈21.65pt); phase ticks x = 31.3→195.5pt,
equally spaced):

| Phase | **Core** | **Iso** | **Strict** |
|---|---|---|---|
| Start | 65.9% | 26.5% | 26.5% |
| Early | 57.1% | 23.1% | 9.6% |
| Mid | 53.1% | 10.9% | 2.5% |
| Late | 56.3% | 20.1% | 0.6% |
| Final | 32.3% | 6.0% | **1.3%** |

**Reading-recomputation artifact (migrated in batch 272)**:
`D:/tb-eval/scbench/scb-paper-2603.24755/scripts/digitize_figure2.py` (emits the values in this table;
quantization difference ≲0.1pp).

**Four-way cross-validation (evidence that the readings are trustworthy)**: ① **Start-phase Iso ≡ Strict
(both 26.5%)** — paper §2.4 states that "C₁ has no regression tests", so with no regression tests at the
first checkpoint the ISO and strict sets coincide, which is **structurally necessarily equal**;
② **Strict Final = 1.3%** matches the paper macro `\abFigStrictSolveFinal{1.3}` **digit for digit**;
③ the **Core/Iso ratio** = Start 2.49／Final 5.47, matching the paper macros
`\abFigCoreIsoRatioMin{2.5}`／`Max{5.4}` ("widens from 2.5× to 5.4×") **digit for digit**; ④ the
figure's "**42pp gap**" annotation = Mid-phase Core−Iso = 53.1−10.9 = **42.2pp** ✓.

**Two points that must be read together with the numbers**:

1. **The paper's own two surfaces are not identical**: the body/macro surface (`overall_stats.tex`,
   sourced from `data/runs.csv` plus `data/checkpoints.csv`) gives **Core 64.6%→35.5%** (macros
   `EarlyPct`／`LatePct` = first/last phase), while the figure surface gives **65.9%→32.3%**; the cost
   panel is about 1.25→2.15 USD/checkpoint in the figure (≈1.7×) against the macro
   `\abFigCostGrowth{2.2}`. **Phase comparisons in this line always use the figure readings** (per-phase
   readable, and passing four-way cross-validation); body macro values are separately attributed to
   their source surface whenever the paper's own wording must be quoted.
2. **The paper's queue curve is also non-monotone**: Core and Iso rebound at the Late phase
   (53.1→56.3, 10.9→20.1) while Strict keeps falling (2.5→0.6) — **non-monotonicity is not unique to
   this line**; it is present on the paper's queue-mean surface too (a change in the phase-group
   composition between Mid and Late is the prime suspect; the paper does not discuss it).

#### 4.3.2 Four-way same-phase comparison (paper figure 2 left definition)

| Round | Metric | Start | Early | Mid | Late | Final |
|---|---|---|---|---|---|---|
| **Paper queue (15 models)** | Core | 65.9 | 57.1 | 53.1 | 56.3 | **32.3** |
| | Iso | 26.5 | 23.1 | 10.9 | 20.1 | **6.0** |
| | Strict | 26.5 | 9.6 | 2.5 | 0.6 | **1.3** |
| **r4 clean room** | Core | 94.3 | 91.3 | 92.2 | 86.3 | **85.4** |
| | Iso | 89.3 | 90.6 | 89.5 | 80.9 | 82.5 |
| | Strict | 89.3 | 90.7 | 88.9 | 86.8 | 82.0 |
| **0cw** | Core | 92.0 | 82.7 | 82.7 | 76.6 | 74.5 |
| | Iso | 88.2 | 82.9 | 79.1 | 72.3 | 76.3 |
| | Strict | 88.2 | 83.4 | 78.3 | 78.6 | 78.4 |
| **0cr** | Core | 94.0 | 88.7 | 86.7 | 81.7 | 77.7 |
| | Iso | 88.8 | 87.8 | 83.4 | 77.9 | 75.1 |
| | Strict | 88.8 | 89.2 | 85.7 | 82.5 | 78.6 |

(r3 covers only 14 problems and is omitted: Core 87.3→92.6→90.5→89.6→80.9; Iso 88.7→96.7→89.3→85.3→72.9;
Strict 88.7→96.2→90.0→88.1→80.9. **All four rounds sit far above the paper queue, and the three curves'
relative structures differ**: the paper's Iso/Strict collapse into the single digits, while the four
rounds' final checkpoints remain at 72.9–85.4%.)

**How to read it**: all four orz rounds' phase curves land in the "gentle decline" band and are **not
the same form** as the paper queue's collapse; but the **levels differ** (paper = 15-model pooled mean,
four rounds = one model over four rounds), so this document makes a form comparison only and no
capability claim (§10).

### 4.4 Layer 4 · trajectory level (quality-degradation axis)

**The definition must be unified (first unified in this document; see the §8 correction)**: three
definitions are laid out side by side —

| Definition | Meaning | r4 | r3 | 0cw | 0cr | Paper |
|---|---|---|---|---|---|---|
| **Net-rise trajectory** (primary) | share of problems whose final checkpoint > first checkpoint | **75.0%** | 71.4% | 58.3% | 80.6% | **77%** |
| Rise at any checkpoint | share of problems with a rise anywhere in the trajectory | 94.4% | 100.0% | 86.1% | 100.0% | — |
| Checkpoint-level transition rate | Σ rising checkpoints／Σ transitions (official scb-check semantics) | 54.4% | 61.1% | 48.8% | 65.0% | — |

**verbosity** (same three definitions): net rise **77.8%**／85.7%／58.3%／77.8%, paper **75.5%**;
rise at any checkpoint 100%／85.7%／83.3%／94.4%; transition rate 56.2%／55.6%／47.5%／58.8%.

**How to read it**: under the **unified net-rise definition**, the four rounds' quality-degradation axis
falls inside a 58–86% band, **in the same direction** as the paper's 77%／75.5%; r4's erosion (75.0%) is
slightly below the paper's and its verbosity (77.8%) slightly above — **the separation phenomenon of
"scores hold while quality degrades" also holds under the clean-room form**.

### 4.5 Difficulty surface (problem-equal-weight core mean)

| Difficulty | r4 | r3 | 0cw | 0cr | r4 full solves |
|---|---|---|---|---|---|
| Easy (12 problems／67 checkpoints) | **0.9814** (61/67) | 0.9229 (5 problems) | 0.9630 (59/67) | 0.9635 (59/67) | 9/12 |
| Medium (12 problems／57 checkpoints) | **0.8805** (40/57) | 0.9817 (5 problems) | 0.8476 (38/57) | 0.8025 (34/57) | 4/12 |
| Hard (12 problems／72 checkpoints) | **0.7919** (43/72) | 0.5929 (4 problems) | 0.6600 (34/72) | 0.7371 (41/72) | 3/12 |

**The Hard surface is where the four rounds differ most**: on Hard, r4 is **+0.132** above 0cw and
+0.055 above 0cr — once clean-room discipline (no /tmp leak, no .gsa leak) is combined with long-horizon
retention, the Hard surface gains the most; r3's Hard (0.5929) is doubly limited by its 4-problem sample
and the /tmp leak form and is not read as a trend.

### 4.6 Layer 5 · per-problem trajectory morphology (added by this batch; common definition across the four rounds)

**Definition**: take each problem's core sequence in checkpoint order — "monotone non-rising" = never
rose; "contains later rebound" = after falling below its earlier maximum, it later rose back above the
point it had fallen under; "late-drop ≥0.5" = the difference between the first-half maximum and the
second-half minimum is ≥0.5 (a coarse collapse criterion).

| Round | Monotone non-rising | Monotone non-falling | **Contains later rebound** | Late drop ≥0.5 |
|---|---|---|---|---|
| r4 (36 problems) | 18 | 17 | **17 (47%)** | 6 |
| r3 (14 problems) | 10 | 9 | 2 (14%) | 2 |
| 0cw (36 problems) | 21 | 17 | **13 (36%)** | 11 |
| 0cr (36 problems) | 18 | 11 | **18 (50%)** | 11 |

**Typical rebound samples**: 0cr datagate (loses points in the first three checkpoints → steadies in the
last three), mocked_http (loses at ck7, recovers at ck8), r4 dag_execution (0.356→0.606), trajectory_api
(0.458→0.900), meshctl (0cw all eight checkpoints zero → r4 8/8).

**Conclusion**: **the correctness surface = a gentle overall decline + violent per-problem fluctuation
(including rebounds)** — "monotone non-rising" covers only about half (18/21/18/10 problems), and at
most 11/36 genuinely drop ≥0.5 late; whereas **"overall degradation" holds on the quality axis** (§4.4,
86–100% of problems under the rise-at-any-checkpoint definition) — the two axes must be read
separately. This table is morphology work added by the four-way batch (V1/V2 gave only a qualitative
"non-monotone recovery" and did not quantify the later-rebound share).

---

## 5. Per-problem four-class attribution (36 problems; classification definition per the batch-253 adjudication)

**Four classes**: **F = framework fault** (measurement loss caused by the setup／mechanical layer)／
**U = model untested** (no workspace output for that checkpoint, so its score cannot support a capability
claim)／**T = model test drift** (substantive work done, but direction or semantics diverge)／
**D = problem defect** (oracle／reference-solution defect, undocumented requirement, evaluation
semantics inconsistent with the problem statement). One problem may carry several classes at once.

| # | Problem | Difficulty | Checkpoints | r4 | r3 | 0cw | 0cr | Fully solved in | Attribution (evidence) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | cfgpipe | E | 6 | 1.000 | 1.000 | 1.000 | 1.000 | all four | — (full solve in all four rounds, capability surface) |
| 2 | circuit_eval | M | 8 | 1.000 | 1.000 | 0.985 | 0.985 | r4/r3 | — (one test short at the final checkpoint) |
| 3 | code_search | E | 5 | 0.975 | 1.000 | 0.975 | 0.975 | r3 | — (first-run build failure was rerun, zero cost) |
| 4 | dag_execution | H | 3 | 0.606 | 0.356 | 0.478 | 0.356 | — | **T＋D**: self-tests 21/21 green → official red (agent-vs-official semantic gap) + the `sleep()` built-in task is undocumented (253 §4) |
| 5 | database_migration | M | 5 | 0.800 | 1.000 | 0.767 | 0.833 | r3 | — (cross-round variance) |
| 6 | datagate | E | 7 | 0.909 | 0.964 | 0.964 | 0.909 | — | — (non-monotone recovery sample) |
| 7 | dynamic_buffer | H | 4 | 0.717 | 0.722 | 0.922 | 0.710 | — | **D**: reference-solution defect footnote (KNOWN_ISSUES, ◆ck4, V1 §8) |
| 8 | dynamic_config_service_api | M | 4 | 0.908 | 0.908 | 0.908 | 0.771 | — | — (lift across rounds) |
| 9 | env_manager | E | 5 | 1.000 | 0.650 | 0.650 | 1.000 | r4/0cr | **D**: reference solution ck3 strict 0.984; the agent side is fully green (V1 §8) |
| 10 | etl_pipeline | E | 5 | 1.000 | 1.000 | 1.000 | 1.000 | all four | — (full solve in all four rounds) |
| 11 | eve_industry | H | 6 | 0.833 | 1.000 | 1.000 | 1.000 | r3/0cw/0cr | **U＋F**: ckpt5 sentinel halt on reasoning repetition (`rounds=0`; adjudication = the guard was correct and the model's occasional fluctuation is an environmental fact, 268 §7) |
| 12 | eve_jump_planner | M | 3 | 0.833 | 1.000 | 1.000 | 0.167 | r3/0cw | — (one of the largest cross-round swings, variance surface) |
| 13 | eve_market_tools | H | 4 | 0.544 | 0.294 | 0.575 | 0.169 | — | **D＋T**: reference-solution defect footnote (◆ck1–3, KNOWN_ISSUES) + persistently low across four rounds |
| 14 | eve_route_planner | M | 3 | 1.000 | 1.000 | 0.667 | 1.000 | r4/r3/0cr | — (0cw low in a single round) |
| 15 | execution_server | E | 6 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | **D** (reference-solution side): ◆ck6 the reference solution fails while the agent is fully green (V1 §8) |
| 16 | file_backup | E | 4 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | **D** (reference-solution side): ◆ck2/ck3 the reference solution falls short |
| 17 | file_merger | M | 4 | 0.911 | — | 0.911 | 0.934 | — | **D** (reference-solution side): ◆ck2–3 the reference solution fails while the agent is fully green |
| 18 | file_query_tool | M | 5 | 0.975 | — | 1.000 | 0.925 | 0cw | — (r4 includes a post-incident rerun; dual definitions on record) |
| 19 | forge | E | 8 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | — (full solve on an 8-checkpoint long chain) |
| 20 | l2m | E | 5 | 0.893 | — | 1.000 | 0.893 | 0cw | — (cross-round variance) |
| 21 | layered_config_synthesizer | M | 4 | **0.467** | — | 0.411 | 0.814 | — | **T**: ck2–4 collapse in a chain (0.2/0.33/0.33) and never fully recovered across four rounds, the round's lowest |
| 22 | log_query | M | 5 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | — (full solve) |
| 23 | meshctl | H | 8 | 1.000 | — | 0.000 | 0.958 | r4 | **T** (0cw side): all eight checkpoints zero is a single-round form; r4／0cr normal — **the largest single-problem flip on the variance surface** |
| 24 | metric_transform_lang | H | 5 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | — (full solve) |
| 25 | migrate_configs | E | 5 | 1.000 | — | 1.000 | 0.800 | r4/0cw | — (0cr loses points at one checkpoint) |
| 26 | mocked_http | H | 8 | 0.834 | — | 0.859 | 0.901 | — | **T＋F**: ckpt7 spent 147 rounds and 47min pushing in the wrong direction, core 0 (same-family behavior as 268 §7); two checkpoints recovered immediately on retry after provider decode errors |
| 27 | mvvault | M | 6 | 1.000 | — | 1.000 | 0.937 | r4/0cw | **D** (reference-solution side): ◆ck5 the reference solution is 1 test short |
| 28 | pwd_manager | M | 5 | 0.771 | — | 0.705 | 0.805 | — | **T**: includes a single-checkpoint collapse (final checkpoint around 0.17) |
| 29 | recli | H | 8 | 1.000 | — | 0.325 | 0.704 | r4 | **T** (0cw side): a persistently off-course form; five-run variance series (2/8→3/8→3/8→8/8→3/8) plus 8/8 this round |
| 30 | rejector | H | 5 | 0.553 | — | 0.553 | 0.693 | — | **F＋U＋T**: ckpt2 provider decode errors exhausted the retry window and the checkpoint was ruled dead (`rounds=0`) + ckpt4 stopped after 1 round in 2 seconds; a near-dead checkpoint (ckpt2 core 0.1) |
| 31 | sheeteval | H | 7 | 0.786 | — | 0.786 | 0.905 | — | **T**: a seven-checkpoint band of low scores (fluctuating 0.667–1.0, no monotone form) |
| 32 | sith | H | 6 | 0.821 | — | 1.000 | 0.890 | 0cw | — (cross-round variance) |
| 33 | test_translator | H | 8 | 0.808 | — | 0.421 | 0.559 | — | **D＋T**: the reference solution scores strict 0.69–0.75 on every checkpoint (◆) + 2,069 tests in the problem statement = a capability-boundary sample |
| 34 | textdrop | E | 6 | 1.000 | — | 1.000 | 1.000 | r4/0cw/0cr | — (full solve) |
| 35 | trajectory_api | M | 5 | 0.900 | — | 0.817 | 0.458 | — | **T** (0cr side): a single-checkpoint collapse form; r4 improves +0.442 |
| 36 | xjq | E | 5 | 1.000 | — | 0.967 | 0.985 | r4 | — (full solve in r4) |

**Attribution summary**: **D** (problem／reference-solution defect) = 9 problems (dag_execution,
dynamic_buffer, env_manager, eve_market_tools, file_backup, file_merger, mvvault, test_translator,
execution_server); **T** (model test drift) = 11 problems; **F** (framework fault) = 3 problems
(eve_industry sentinel, mocked_http provider, rejector provider, **all mechanical-side accounting with
zero problems losing points to a framework fault**); **U** (model untested) = 2 problems (one
`rounds=0` checkpoint each in eve_industry／rejector); **18 problems in total carry an attribution
signal** (de-duplicated by problem where classes coexist) and the other **18 problems carry no failure
attribution signal** (capability-surface readings).
**Framework-fault conclusion = zero scoring impact**: all four rounds' write-control blocks
(52／13／53／9 instances) rerouted within a single round, with zero problems failing because of write
control; every transport_retry (24／6／105／—) self-healed; r4's 2 `run_failed` instances trace to
provider stream corruption and a sentinel halt respectively, neither a framework defect (268 §7／§12).

---

## 6. Paper same-direction／reverse final verdict (proposition by proposition)

| # | Paper proposition (source) | Paper reading | Four-way reading | **Verdict** |
|---|---|---|---|---|
| 1 | "no agent fully solves any problem end-to-end" (abstract §1; definition = **all tests including regressions, all checkpoints passing**) | **0/36 end-to-end full solves** | All-checkpoint strict perfect: r4 **4/36**, 0cw 5/36, r3 3/14, 0cr 1/36 (the broad "all-checkpoint core perfect" 16／15／8／11 is not this proposition's target) | **Reverse** (all four rounds have end-to-end full solves; the magnitude is far below the broad reading, and the three incomparabilities are on record) |
| 2 | "the best agent passes 14.8% of checkpoints" (abstract) | 14.8% strict (GPT 5.5) | r4 strict 33.7%／isolated 44.9%／core 73.5% | **Reverse** (materially higher under the isomorphic definition; harness + model price tier confounded) |
| 3 | "structural erosion rising in 77% of trajectories and verbosity in 75.5%" (abstract §1, RQ2) | 77%／75.5% | Net-rise definition: r4 75.0%／77.8%, r3 71.4%／85.7%, 0cw 58.3%／58.3%, 0cr 80.6%／77.8% | **Same direction** (all four rounds land in the same band; r4 slightly lower on erosion, slightly higher on verbosity) |
| 4 | "Core pass rate falls from 64.6% to 35.5%" (§3.1／figure 2 left; **five-phase queue-mean** definition, 15 models pooled) | **Figure points (batch-271 vector digitization)**: Core 65.9→57.1→53.1→**56.3**→**32.3**; Iso 26.5→23.1→10.9→20.1→**6.0**; **Strict 26.5→9.6→2.5→0.6→1.3**; body macro surface Core 64.6→35.5／Core-Iso ratio 2.5×→5.4×／Functionality −5.0pp, Error −17.9pp | Four-round Core phases: 94.3→85.4／87.3→80.9／92.0→74.5／94.0→77.7; Iso final 82.5／72.9／76.3／75.1; **Strict final 82.0／80.9／78.4／78.6**; Core/Iso ratio ≈1.0 throughout | **Reverse (magnitude + structure)**: the paper's Strict collapses to **1.3%** and its Iso to 6.0%, while the four rounds' corresponding final checkpoints sit at 78–82%; the four rounds show no relative collapse and their losses are not concentrated in the error／regression chain (§4.3.1–4.3.2／§4.6) |
| 5 | "as we do not provide the prior conversation's context" (§2 setup) | no conversational memory as a design premise | the r4 clean-room form **realizes that premise** (no /tmp leak, no .gsa leak); 0cw／r3 violate it respectively (via .gsa／/tmp) | **Same direction (form meets the premise)** — r4 is the only one of the four rounds that satisfies the paper's memory premise |
| 6 | "Each run has a two-hour wall-clock limit" (§2 setup) | 2h／run cap | r4／0cw／r3 impose the 7200s per-checkpoint cap (**never triggered in r4**) | **Same direction** (the cap is in service; it did not take effect in r4) |
| 7 | "agent code is 2.3× more verbose and 2.0× more eroded than 473 repositories" (RQ3) | human-repository comparison | **untested** (requires a human-repository baseline; this line does not do it) | **Untested** (no extrapolation) |
| 8 | "Quality-aware prompts reduce initial degradation ... do not slow degradation" (RQ4) | prompt comparison | **untested** (no prompt-comparison arm was run) | **Untested** (no extrapolation) |
| 9 | "Per checkpoint, agent verbosity grows 6.6× and erosion grows 5.0× faster than human code" (§2) | human-repository comparison | **untested** | **Untested** (no extrapolation) |
| 10 | leaderboard "ranked by isolated solve rate" (§3 metrics) | leaderboard sorted by isolated | r4 isolated fully passed 44.9% vs the top entry's 28.1% (isomorphic projection) | **Directionally higher** (aggregation formula pending official confirmation; no capability claim) |
| 11 | Whether the paper reports **per-problem (trajectory-level) correctness** collapse／rebound (**item not reported**) | — (Table 4/5 are problem-statement and design-decision notes, Table 8 is a run summary; the only "monotone" instance in Appendix B.2 belongs to the quality surface: Opus 4.7 forge `main()` CC rising monotonically 13→92) | Four-way per-problem morphology in §4.6: contains later rebound 47%／14%／36%／50%; monotone non-rising only about half | **Level clarification**: the paper's collapse is a **queue-mean definition** while per-problem morphology is an addition by this line; the two **cannot be inferred from each other** |

**Definition note (batch 271) · the leaderboard, strict, and end-to-end full solve are not the same
quantity**: the paper's Table 1 and the scbench.ai leaderboard are **ranked by Iso (isolated solve
rate)** (the figure caption reads "ranked by isolated solve rate with core, strict, and \$/checkpoint as
tiebreakers"), and all three solve rates are **checkpoint-level** using a **fixed 196-checkpoint
denominator** (missing counts as unsolved); `Partial` alone is **problem-level** (≥1 checkpoint strictly
solved). **End-to-end full solve** (the abstract's "no agent fully solves any problem") is likewise
**problem-level**, but its bar is every checkpoint passing every test — which is **not the same value**
as strict solve: strict solve answers "what share of checkpoints pass all tests" (paper 14.8%, this line's
r4 33.7%), while end-to-end full solve answers "how many problems pass all tests at every checkpoint"
(paper 0/36, this line's r4 4/36). The relationship is that the former is the latter's necessary
condition surface: a problem-level full solve necessarily entails strict passes at all its checkpoints,
but not conversely. **The same model's two numbers can differ by more than an order of magnitude**
(this line's r4: 33.7% vs 4/36 = 11.1%), so any comparison must first declare its level and denominator.

**Verdict summary**: **3 reverse** (end-to-end full solve, best pass rate, reset-means-collapse — all
recomputed under the paper's own definitions, see propositions 1／2／4), **3 same direction** (the
quality-degradation axis, the no-conversational-memory premise, the 2h cap), **4 untested** (2
human-repository comparisons, 1 prompt comparison, 1 detailed \$/CKPT accounting), **1 level
clarification** (proposition 11).
**The most robust conclusion** is proposition 3: the quality-degradation axis **does not move with the
harness, the session form, or the leak surface** — the only proposition the four rounds agree with the
paper on quantitatively, and this line's strongest evidence for the "score vs quality separation".
**Must be read together with the numbers**: the reverse magnitudes in propositions 1／4 are bounded by a
double confound of model price tier × harness (paper = frontier model × native CLI; this line = flash
price tier × orz); the levels and definitions are aligned, but **capability attribution is not
extrapolated**.

---

## 7. r3 leak-form registration and bias statement (explicit, per 253 §3)

- **Channel**: the adapter uploaded each checkpoint's problem-statement instructions to
  `/tmp/orz-instruction-{N}.md` and **never cleaned them up**, so prior checkpoints' instructions piled up;
  a full sweep of r3 hit this channel on **14/14 problems** (150 read events; p04 demonstrated
  `cat /tmp/orz-instruction-1.md /tmp/orz-instruction-2.md`).
- **Bias direction**: compared with the paper's strict-reset definition, **scores from ckpt2 onward may be
  mildly and systematically high** — the model could read back prior checkpoints' problem statements,
  which is equivalent to having "this checkpoint's spec" and "the prior checkpoint's spec" in front of it
  at the same time, lowering the difficulty of "understanding the incremental spec evolution". r3
  readings therefore **must not** be used to claim a strict official reset definition externally.
- **Comparable boundary**: r3 and 0cw are both leak forms (different writers: the host artifact `/tmp`
  vs orz's own `.gsa`), so **the two are A/B comparable**; r3 and r4／0cr **are not comparable on form**
  (different leak surfaces).
- **Fix loop closed**: r4 burns on both sides (the driver deletes after reading + the adapter clears old
  files before uploading) plus a per-checkpoint `/tmp` surface assertion (the right checkpoint file exists
  at the start, the directory is empty at the end, `burned=true`); all 36/36 problems pass that assertion
  in the r4 ledger ⇒ **the channel is sealed**.
- **Upstream**: the finding was reported faithfully as
  [SprocketLab/slop-code-bench#44](https://github.com/SprocketLab/slop-code-bench/issues/44) (253 §8).

---

## 8. Correction and definition unification (the V2 batch's degradation-trajectory definition)

**Correction (first unified in this document; dated documents are not rewritten)**: in the single line
for "share of erosion／verbosity rising trajectories" in V2 report §1／§4.4, **the 0cw side used "a rise at
any checkpoint"** (31/36 = 86.1%, 30/36 = 83.3%, equivalent to the ledger's native `erosion_increased`
boolean), while **the 0cr side and the paper comparison used "final > first"** (29/36 = 80.6%,
28/36 = 77.8%). The two definitions were mixed on one comparison line, which makes V2 §4.4's
**direction** — "the reset form's erosion surface rises slightly (+5.5pp)" — **invalid**:

| Definition | 0cw | 0cr | Difference |
|---|---|---|---|
| Rise at any checkpoint (V2's 0cw side) | 86.1% | 100.0% | −13.9pp |
| **Final > first (unified definition)** | **58.3%** | **80.6%** | **−22.3pp** |

**Conclusion under the unified definition**: 0cw (the .gsa leak · hybrid form) has an erosion net-rise
trajectory **lower** than 0cr (the continuous form) by 22.3pp and verbosity lower by 19.5pp — V2's
**direction** for "the quality-degradation axis barely moves with the form" **should be reversed to "the
hybrid form produces fewer degradation trajectories"**; the difference most likely comes from the .gsa
leak (the model can read back the ledger and prior-checkpoint sessions, effectively gaining cross-checkpoint
memory and reducing the opportunity for "the same function being patched again and again"), but because
the carrier generation and model price tier are confounded, **this document makes no single-cause
attribution**. From §4.4 onward this report lays all three definitions out side by side.

---

## 9. Friction-surface four-way comparison (main line)

| Surface | r4 | r3 | 0cw | 0cr (V1 §7) |
|---|---|---|---|---|
| Write-control reviews, total | 21,707 | 8,343 | 44,736 | — |
| of which block | **52** | 13 | 53 | 9 |
| Block family | mostly `broad-destructive` false positives plus `carrier-write` (the .gsa archive surface, by design) | same | same | 0cq family 6 + 3 by design |
| Rerouted rounds | 1 round per instance, zero problem failures | same | same | same |
| transport_retry | 24 (self-healed) | 6 | 105 (self-healed) | — |
| run_failed | 2 (1 provider stream corruption + 1 sentinel halt) | 0 | 20 | 0 |
| New friction charters | 0 (batch 268 ruled the sentinel implementation correct and the model fluctuation an environmental fact) | /tmp leak → 0dc chartered | .gsa leak → 0cy chartered | 0ct／0cq family |

**How to read it**: the leak forms (r3／0cw) turn the "cross-checkpoint information surface" into a
friction source, while the clean-room form **structurally zeroes out** that class of friction; r4's
remaining blocks are all **mechanical-layer false positives plus by-design retained surfaces**, with zero
scoring impact — **the friction main line converges under the clean-room form**.

---

## 10. Boundaries and non-extrapolable items

1. **k=1 on four sides**: meshctl (0.000↔1.000), recli (0.325↔1.000), eve_jump_planner (0.167↔1.000)
   quantify single-run uncertainty across three rounds; no single-problem reading supports a capability
   claim.
2. **Single snapshot**: each round keeps its own carrier (0.8.14／0.8.15／0.9.0) and model name throughout;
   cross-round differences include carrier generations.
3. **Three incomparabilities** (vs the paper／leaderboard): different harness, different model price tier,
   different k and budget surfaces — the whole table supports directional reading only.
4. **r3 bias**: 14 problems plus a leak form, serving only as a leak baseline, not as a strict
   official-definition reading (§7).
5. **0cw qualitative correction**: batch 235 already ruled it a "context reset + disk-memory recovery"
   hybrid form, so V2's "official reset form" characterization does not hold; this document carries
   forward the corrected characterization.
6. **Untested surfaces**: human-repository quality comparison (RQ3), prompt comparison (RQ4), and detailed
   \$/CKPT accounting were not done and are not extrapolated.
7. **Score submission**: none of the four rounds is an **official score submission**; a public Harbor run
   is the factual leaderboard-entry channel (batch 208 determination).
8. **Paper level alignment (batch 270 correction)**: the paper's "collapse" is a five-phase **queue mean**
   (15 models pooled) and it **does not report per-problem correctness morphology**; §4.3／§4.6 here have
   been recomputed under the same mapping and extended with per-problem morphology — the two **cannot be
   inferred from each other**, and any "paper collapse vs our rebound" comparison must first declare its
   level. Also: the paper's "end-to-end full solve" definition = all checkpoints passing all tests
   including regressions, which differs from "all-checkpoint core perfect" (both are listed as separate
   rows in §1 here).

---

## 11. Reproduction entry points

- Raw ledgers for the four rounds: `D:/tb-eval/scbench/jobs-r4/r4_checkpoint_ledger.jsonl`／
  `jobs-r3/r3_checkpoint_ledger.jsonl`／`jobs-0cw/s3_checkpoint_ledger.jsonl`／
  `0cr_official/merged_checkpoint_results.jsonl`
- Four-way aggregate recomputation artifact: `D:/tb-eval/scbench/v4_four_way_readings.json` (the mechanical
  output behind every number in §1／§4／§5 of this document)
- Per-problem five-checkpoint readings and hub URLs: each round's `*_problem_ledger.jsonl`
- Paper text: `.tmp-scb-paper-text.txt` (abstract／§2／§3 Table 7 definition rows); **paper source and
  digitization/recomputation scripts** = `D:/tb-eval/scbench/scb-paper-2603.24755/`
  (`src/` arXiv source + `scripts/digitize_figure2.py` + `scripts/v4_phase_morphology.py` + `out/`;
  migrated in batch 272)

---

## 12. Line closure (user decree 2026-10-16, "SlopCode Bench ends here")

- **All four rounds are on the ledger**: 0cr (continuous form, V1)／0cw (hybrid form, V2 plus the batch-235
  correction)／r3 (leak form, stopped in batch 253)／r4 (clean-room official form, this V4 document).
- **This line will not open new rounds**; unfinished items are registered faithfully = paper propositions
  7–9 (human-repository comparison, prompt comparison) untested, leaderboard aggregation did not happen
  automatically (verified 10-11), and the 6 reference-solution defects (◆) can be upstreamed (candidate,
  not started).
- **Related-item handling**: 0dc (the r4 charter) closes with this S4 delivery; 0cw (the reset-form round)
  closes with the line (its readings are retained as a hybrid-form archive).

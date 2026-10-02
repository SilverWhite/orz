# Terminal-Bench 2.1 (V4.1) Full-Round Rerun — 89 Tasks: Score and Situation Report (2026-10-01)

> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.
> Source: [`TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md`](../TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) (Chinese).
> The 89-row score table is copied cell-for-cell from the source; only the prose columns
> ("shape", "notes") are translated. Terminology: [TRANSLATION_GLOSSARY.md](TRANSLATION_GLOSSARY.md).

> Type: round score & situation report (external disclosure draft / internal closeout record).
> Scope: 2026-09-28 18:32 – 2026-10-01 14:23, the official frozen Terminal-Bench 2.1 set, all
> 89 tasks, one full round (including two targeted reruns).
> Authority chain: round kickoff & first-half account
> [`TB21_V41_FULL_RERUN_START_2026-09-28.md`](../TB21_V41_FULL_RERUN_START_2026-09-28.md)
> (§1–§11) + second-half adjudication & annex
> [`TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md`](../TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)
> (both Chinese).
> No per-task reading in this report comes from a secondhand account: all were extracted
> mechanically, attempt by attempt, from the `reward_stats.reward` field of each job's
> `D:/tb-eval/jobs-official/<job>/result.json` (extraction scripts in §12).
> Boundary: a **k=1 screening round**, **not a leaderboard score** (the official leaderboard
> protocol requires ≥5 attempts per task).

---

## 1. Summary

| Item | Reading | Notes |
|---|---|---|
| **Final task-level score** | **73/89 = 82.0%** | Each task counted exactly once; the two targeted reruns replaced the original attempts |
| First-pass account (excluding rerun3) | 72/87 = 82.8% (80.9% over all 89) | 2 tasks were job-level failures with no attempt reading |
| Attempt-level score | 85/110 = 77.3% | All official job attempts, including the replaced originals |
| Batch distribution (final) | B1 13/16｜B2 14/17｜B3 15/19｜B4 13/18｜B5 18/19 | — |
| Comparison: R1-generation same-protocol full round | 58/89 = 65.2% | Same dataset, same k=1; different carrier generation |
| Comparison: officially published value | 90.6 (DeepSeek-V4.1-Flash, not k=1 same-protocol) | Reference line only, not a pass/fail criterion |
| No-wall-clock control (rerun3) | 18 tasks = 13 pass / 5 fail, **two in, two out, net effect exactly zero** | See §6 |
| Actual time | 114 jobs, 33.8 h cumulative wall clock, 17.8 min average | Main round 27.5 h + rerun3 series 6.1 h |

Three things that must be read together with the numbers:

1. **The round spans two release generations**: 0.8.4 (`87941130…`) and 0.8.7 (`3332b38f…`);
   the generation switch was driven by a write-control defect fix, not by score tuning
   (disclosure in §9).
2. **Both reruns replaced the original attempt — no cherry-picking**: rerun2 selected 5 tasks
   by the mechanical threshold "a 0-score task with ≥5 commands intercepted by mechanical
   write control"; rerun3 selected 18 tasks as the complete wall-clock-exposure surface.
3. **82.0% is not a product of "reading the clock"**: every task whose surface exposed the
   wall clock was rerun **without the agent-side wall clock**; the net effect over 18 tasks
   was exactly zero (§6). That study set is the complete exposure surface, not a sample of the
   round, and the finding does not extrapolate to the other 71 tasks.

---

## 2. Protocol and apparatus

| Item | Value | Evidence location |
|---|---|---|
| Harness | Official terminal-bench 2.1 (harbor 0.20.0) | `*-round.log` plan lines per job |
| Dataset pin | `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a` | same |
| Frozen task order | `evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json` + `run_official_2.1.sh` BATCH1–BATCH5 (16/17/19/18/19) | freeze manifest |
| Model | `deepseek-v4-flash` (user-informed 2026-09-30: fully routed to the V4.1 Flash generation) | plan lines |
| Attempts / concurrency | k=1; one job per task; `-n 1`, serial | driver |
| Wall clock | Official per-task agent timeout passed through via `--ak max_wallclock` (900–12000 s, per-task values in §4); **all 18 rerun3 tasks ran with no agent-side wall clock**, the official task-level timeout remaining the only outer boundary | plan lines / rerun3 condition lines |
| Upload | Per-task `--upload --public` (Harbor public jobs) | plan lines |
| Qualifier | k=1 screening round, not a leaderboard score | schedule §3 |
| Carrier | 0.8.4 `87941130…` (the 40 first-half tasks whose original attempts were kept) / 0.8.7 `3332b38f…` (the other 49, including 5 rerun2 replacements); one 0.8.5 `ecf1d665…` run as a lineage note; 0.8.6 never served | plan lines per job |
| Adapter | `tb_agents/orz.py` = `6d55c26e…` (unchanged throughout) | plan lines |
| Execution apparatus | first-half batch driver → second-half single-task driver → `run_official_v41_inspected.py` (single-task harvest) → `run_puller_control.py` (rerun3, per-task) | round logs |

Scale: 89 main-round jobs + 5 rerun2 + 18 rerun3 + 1 verification run on 0.8.5 + 2 end-to-end
probes = 115 job directories; 114 contain attempt readings, 1 (the b1-01 first-start skeleton)
has none. One probe job (`official-v41-rerun3-probe-aptfix-qemu-startup`, not uploaded) counts
toward no score.

---

## 3. Final account

### 3.1 Task level (each task counted once; reruns replaced originals)

| Batch | Passed | Tasks | Pass rate |
|---|---|---|---|
| B1 | 13 | 16 | 81.3% |
| B2 | 14 | 17 | 82.4% |
| B3 | 15 | 19 | 78.9% |
| B4 | 13 | 18 | 72.2% |
| B5 | 18 | 19 | 94.7% |
| **Total** | **73** | **89** | **82.0%** |

### 3.2 First-pass account (official round, excluding rerun3)

87 tasks had attempt readings: 72 passed (82.8%), 15 did not; `qemu-startup` /
`qemu-alpine-ssh` were environment-level job failures (the adapter's dependency install hit
Debian security repo 404s, dying in 22–38 s with zero model rounds — no attempt readings).

### 3.3 Attempt level

All official job attempts, 110 total: 1.0 × 85, 0.0 × 25 ⇒ 77.3%. Attempt-level sits below
task-level because of **rerun replacement** (the replaced original attempts still count as
attempts, including several 0.0s) — not because of new failures.

### 3.4 Distance to targets

- Pre-round soft goal "this generation k=1 ≥ R1 (65.2%), reaching past 80% if possible":
  **met** (82.0%).
- 8.6 pp below the officially published 90.6: **different protocol** (official = multiple
  attempts / different harness form); no same-protocol conclusion is drawn this round. The
  earlier model-generation attribution has been retracted; any real gap should be attributed
  to the harness side.

---

## 4. Per-task score table (final readings)

How to read: the **shape** column = how the task's final attempt ended (normal delivery /
at-the-wall pass = killed by the official timeout after delivery was already complete and
scored pass / timeout kill = killed without passing / self-completed judged fail = the model
declared completion but the verifier scored 0 / abnormal exit). The **notes** column = whether
the attempt is a rerun replacement, the job's carrier hash (`87941130` = 0.8.4,
`3332b38f` = 0.8.7), and any apt-source mount deviation. rerun3 rows ran on 0.8.7 (per-task
identity-gate verified; the log has no `plan` line, so it is omitted in the table).

| # | Task | Batch | Score | Time (min) | Official limit (s) | Shape | Notes |
|---|---|---|---|---|---|---|---|
| 1 | build-pov-ray | B1 | 1.0 | 14.1 | 12000 | normal delivery | rerun replacement / 3332b38f |
| 2 | schemelike-metacircular-eval | B1 | 1.0 | 40.5 | 2400 | normal delivery | 87941130 |
| 3 | llm-inference-batching-scheduler | B1 | 1.0 | 27.1 | 1800 | normal delivery | 87941130 |
| 4 | feal-linear-cryptanalysis | B1 | 1.0 | 4.7 | 1800 | normal delivery | 87941130 |
| 5 | dna-assembly | B1 | 1.0 | 26.2 | 1800 | normal delivery | 87941130 |
| 6 | feal-differential-cryptanalysis | B1 | 1.0 | 9.1 | 1800 | normal delivery | 87941130 |
| 7 | polyglot-c-py | B1 | 1.0 | 9.6 | 900 | normal delivery | 87941130 |
| 8 | qemu-startup | B1 | 0.0 | 14.8 | 900 | self-completed, judged fail | rerun replacement / apt-source mount deviation |
| 9 | sqlite-db-truncate | B1 | 1.0 | 7.6 | 900 | normal delivery | 87941130 |
| 10 | vulnerable-secret | B1 | 1.0 | 1.9 | 900 | normal delivery | 87941130 |
| 11 | build-cython-ext | B1 | 1.0 | 12.2 | 900 | normal delivery | 87941130 |
| 12 | configure-git-webserver | B1 | 0.0 | 15.8 | 900 | timeout kill | rerun replacement |
| 13 | fix-git | B1 | 1.0 | 12.4 | 900 | normal delivery | 87941130 |
| 14 | headless-terminal | B1 | 1.0 | 13.9 | 900 | normal delivery | rerun replacement |
| 15 | merge-diff-arc-agi-task | B1 | 1.0 | 12.5 | 900 | normal delivery | 87941130 |
| 16 | git-multibranch | B1 | 0.0 | 16.2 | 900 | timeout kill | rerun replacement |
| 17 | sam-cell-seg | B2 | 1.0 | 24.8 | 7200 | normal delivery | 87941130 |
| 18 | portfolio-optimization | B2 | 1.0 | 13.3 | 3600 | normal delivery | 87941130 |
| 19 | video-processing | B2 | 1.0 | 14.1 | 3600 | normal delivery | 87941130 |
| 20 | mcmc-sampling-stan | B2 | 1.0 | 23.1 | 1800 | normal delivery | 87941130 |
| 21 | path-tracing-reverse | B2 | 1.0 | 18.3 | 1800 | normal delivery | 87941130 |
| 22 | mteb-retrieve | B2 | 1.0 | 15.8 | 1800 | normal delivery | rerun replacement |
| 23 | code-from-image | B2 | 1.0 | 14.8 | 1200 | normal delivery | 87941130 |
| 24 | break-filter-js-from-html | B2 | 1.0 | 20.8 | 1200 | at-the-wall pass | 87941130 |
| 25 | sanitize-git-repo | B2 | 1.0 | 13.5 | 900 | normal delivery | 87941130 |
| 26 | sparql-university | B2 | 1.0 | 10.7 | 900 | normal delivery | 87941130 |
| 27 | tune-mjcf | B2 | 0.0 | 16.5 | 900 | timeout kill | 87941130 |
| 28 | git-leak-recovery | B2 | 1.0 | 9.0 | 900 | normal delivery | 87941130 |
| 29 | cobol-modernization | B2 | 1.0 | 13.3 | 900 | normal delivery | 87941130 |
| 30 | fix-code-vulnerability | B2 | 0.0 | 1.3 | 900 | abnormal exit | 87941130 |
| 31 | gpt2-codegolf | B2 | 0.0 | 16.1 | 900 | timeout kill | rerun replacement |
| 32 | log-summary-date-ranges | B2 | 1.0 | 2.6 | 900 | normal delivery | 87941130 |
| 33 | openssl-selfsigned-cert | B2 | 1.0 | 5.3 | 900 | normal delivery | 87941130 |
| 34 | mteb-leaderboard | B3 | 1.0 | 26.1 | 3600 | normal delivery | 87941130 |
| 35 | reshard-c4-data | B3 | 1.0 | 20.6 | 3600 | normal delivery | 87941130 |
| 36 | winning-avg-corewars | B3 | 0.0 | 43.0 | 3600 | self-completed, judged fail | rerun replacement / 3332b38f |
| 37 | caffe-cifar-10 | B3 | 0.0 | 60.9 | 3600 | timeout kill | rerun replacement |
| 38 | rstan-to-pystan | B3 | 1.0 | 24.5 | 1800 | normal delivery | rerun replacement |
| 39 | extract-moves-from-video | B3 | 0.0 | 31.6 | 1800 | timeout kill | rerun replacement / 3332b38f |
| 40 | custom-memory-heap-crash | B3 | 1.0 | 10.8 | 1800 | normal delivery | 87941130 |
| 41 | constraints-scheduling | B3 | 1.0 | 6.3 | 1200 | normal delivery | 87941130 |
| 42 | pytorch-model-recovery | B3 | 1.0 | 14.4 | 900 | normal delivery | rerun replacement |
| 43 | prove-plus-comm | B3 | 1.0 | 2.7 | 900 | normal delivery | 87941130 |
| 44 | raman-fitting | B3 | 1.0 | 10.9 | 900 | normal delivery | 87941130 |
| 45 | torch-pipeline-parallelism | B3 | 1.0 | 20.9 | 900 | at-the-wall pass | rerun replacement / 3332b38f |
| 46 | adaptive-rejection-sampler | B3 | 0.0 | 16.7 | 900 | timeout kill | 3332b38f |
| 47 | cancel-async-tasks | B3 | 1.0 | 11.0 | 900 | normal delivery | 3332b38f |
| 48 | db-wal-recovery | B3 | 1.0 | 5.3 | 900 | normal delivery | 3332b38f |
| 49 | password-recovery | B3 | 1.0 | 7.4 | 900 | normal delivery | 3332b38f |
| 50 | kv-store-grpc | B3 | 1.0 | 8.7 | 900 | normal delivery | rerun replacement |
| 51 | multi-source-data-merger | B3 | 1.0 | 4.7 | 900 | normal delivery | 3332b38f |
| 52 | modernize-scientific-stack | B3 | 1.0 | 3.7 | 600 | normal delivery | 3332b38f |
| 53 | install-windows-3.11 | B4 | 1.0 | 33.8 | 3600 | normal delivery | rerun replacement |
| 54 | fix-ocaml-gc | B4 | 1.0 | 31.6 | 3600 | normal delivery | rerun replacement |
| 55 | train-fasttext | B4 | 0.0 | 61.4 | 3600 | timeout kill | 3332b38f |
| 56 | circuit-fibsqrt | B4 | 1.0 | 60.9 | 3600 | at-the-wall pass | 3332b38f |
| 57 | path-tracing | B4 | 1.0 | 7.5 | 1800 | normal delivery | 3332b38f |
| 58 | mailman | B4 | 1.0 | 27.7 | 1800 | normal delivery | rerun replacement |
| 59 | crack-7z-hash | B4 | 1.0 | 5.5 | 1800 | normal delivery | 3332b38f |
| 60 | large-scale-text-editing | B4 | 1.0 | 12.0 | 1200 | normal delivery | 3332b38f |
| 61 | pytorch-model-cli | B4 | 0.0 | 15.8 | 900 | self-completed, judged fail | 3332b38f |
| 62 | pypi-server | B4 | 0.0 | 5.0 | 900 | self-completed, judged fail | 3332b38f |
| 63 | regex-log | B4 | 1.0 | 12.8 | 900 | normal delivery | 3332b38f |
| 64 | torch-tensor-parallelism | B4 | 1.0 | 21.2 | 900 | at-the-wall pass | rerun replacement |
| 65 | make-doom-for-mips | B4 | 0.0 | 16.8 | 900 | timeout kill | 3332b38f |
| 66 | chess-best-move | B4 | 0.0 | 16.6 | 900 | timeout kill | 3332b38f |
| 67 | extract-elf | B4 | 1.0 | 10.1 | 900 | normal delivery | 3332b38f |
| 68 | write-compressor | B4 | 1.0 | 13.0 | 900 | normal delivery | 3332b38f |
| 69 | largest-eigenval | B4 | 1.0 | 15.9 | 900 | normal delivery | 3332b38f |
| 70 | nginx-request-logging | B4 | 1.0 | 9.9 | 900 | normal delivery | 3332b38f |
| 71 | regex-chess | B5 | 1.0 | 56.4 | 3600 | normal delivery | 3332b38f |
| 72 | distribution-search | B5 | 1.0 | 8.0 | 3600 | normal delivery | 3332b38f |
| 73 | bn-fit-modify | B5 | 1.0 | 11.3 | 3600 | normal delivery | 3332b38f |
| 74 | compile-compcert | B5 | 1.0 | 20.9 | 2400 | normal delivery | 3332b38f |
| 75 | make-mips-interpreter | B5 | 1.0 | 24.7 | 1800 | normal delivery | 3332b38f |
| 76 | filter-js-from-html | B5 | 1.0 | 33.5 | 1800 | at-the-wall pass | 3332b38f |
| 77 | dna-insert | B5 | 1.0 | 13.3 | 1800 | normal delivery | 3332b38f |
| 78 | protein-assembly | B5 | 1.0 | 17.5 | 1800 | normal delivery | rerun replacement |
| 79 | financial-document-processor | B5 | 1.0 | 21.7 | 1200 | normal delivery | 3332b38f |
| 80 | polyglot-rust-c | B5 | 1.0 | 8.9 | 900 | normal delivery | 3332b38f |
| 81 | query-optimize | B5 | 1.0 | 21.3 | 900 | at-the-wall pass | 3332b38f |
| 82 | sqlite-with-gcov | B5 | 1.0 | 9.0 | 900 | normal delivery | rerun replacement |
| 83 | qemu-alpine-ssh | B5 | 1.0 | 15.9 | 900 | at-the-wall pass | rerun replacement / apt-source mount deviation |
| 84 | build-pmars | B5 | 1.0 | 9.5 | 900 | normal delivery | 3332b38f |
| 85 | count-dataset-tokens | B5 | 1.0 | 10.3 | 900 | normal delivery | rerun replacement |
| 86 | gcode-to-text | B5 | 1.0 | 15.3 | 900 | normal delivery | 3332b38f |
| 87 | hf-model-inference | B5 | 0.0 | 3.5 | 900 | self-completed, judged fail | 3332b38f |
| 88 | model-extraction-relu-logits | B5 | 1.0 | 16.0 | 900 | at-the-wall pass | 3332b38f |
| 89 | overfull-hbox | B5 | 1.0 | 12.1 | 750 | normal delivery | 3332b38f |

> Time = job-level start-to-end difference (including image pre-pull and wrap-up), not pure
> agent runtime.

---

## 5. Rerun and replacement ledger

### 5.1 Round one (rerun2): targeted rerun after an apparatus defect fix, 5 tasks

Inclusion rule = "0-score tasks with ≥5 commands intercepted by mechanical write control"
(the interception tax crowds out effective rounds; <5 interceptions are judged model-side,
and rerunning those would just re-roll the model dice). Carrier 0.8.7, full official protocol
(including `--ak max_wallclock` and public upload).

| Task | Intercepted | Original attempt | rerun2 | Result |
|---|---|---|---|---|
| build-pov-ray | 12 | 0.0 (structural interception) | **1.0** | turnaround |
| torch-pipeline-parallelism | 6 | 0.0 | **1.0** | turnaround |
| extract-moves-from-video | 10 | 0.0 | 0.0 | no flip |
| git-multibranch | 8 | 0.0 | 0.0 | no flip |
| winning-avg-corewars | 5 | 0.0 | 0.0 | no flip |

2/5 turned. build-pov-ray's three-generation lineage is fully archived: structural 0 on 0.8.4
→ still 0 on the 0.8.5 verification run (`official-v41-rerun-build-pov-ray`; that run directly
triggered the deeper fix project) → turnaround to 1.0 on 0.8.7.

### 5.2 Round two (rerun3): the complete wall-clock-exposure surface, 18 tasks

Inclusion rule = the **complete wall-clock exposure surface** from a full-archive scan (argv /
env / journal / session-surface reads), plus the 2 setup-level-failed qemu tasks (closed in the
same batch). Conditions: **no `--ak max_wallclock` passed** (orz's internal wall-clock gate
off; the session surface has no limit/remaining readings; the startup command line has no
leak channel), with the official task-level timeout still the outer boundary; independent job
per task; a single-task harvest check as soon as each finished.

rerun3 vs original = **same count as original, 12/18, two in and two out**:

| Direction | Task | Original | rerun3 |
|---|---|---|---|
| Up | pytorch-model-recovery | 0.0 | 1.0 |
| Up | kv-store-grpc | 0.0 | 1.0 |
| Down | configure-git-webserver | 1.0 | 0.0 (hard-killed at 16.0 min; 8080 still 404 at kill time) |
| Down | caffe-cifar-10 | 1.0 (natural finish at 47.4 min) | 0.0 (hard-killed at 61.2 min) |
| Reading added | qemu-startup | no attempt (job-level failure) | 0.0 |
| Reading added | qemu-alpine-ssh | no attempt (job-level failure) | **1.0** (after the apt-source fix) |

⇒ Over the 18-task set, the final count is **13 pass / 5 fail / 0 unscored**.

### 5.3 Net effect

rerun3 moved two tasks up and two down in opposite directions, so **the task-level total did
not change when the wall clock was removed** (72/87 → 73/89; the entire increment is
qemu-alpine-ssh, unrelated to the wall clock). "Results are not lifted by a visible wall
clock" holds within this study set.

---

## 6. Wall-clock visibility special study

### 6.1 Concern and design

Concern: several passing tasks had read the blackboard `session` partition (including
`WALLCLOCK_LIMIT` / `REMAINING` lines); if the model decided when to deliver by "checking the
watch", part of the score would come from wall-clock visibility. The existing data could not
distinguish "clock-driven convergence" from "would have converged anyway".

Design: **all** tasks on the wall-clock exposure surface were rerun without the agent-side
wall clock, replacing the original attempts.

### 6.2 Condition verification (18/18 held)

- Launch args and env channel: zero `--max-wallclock` residue;
- Full-archive literal scan: zero limit-value reads, zero remaining-time reads, zero
  round-conversion lines in the `journal`;
- Zero model self-reports of time.

Residual surface (honestly disclosed): on one qemu-alpine-ssh occurrence the model read the
`session` surface once and saw `WALLCLOCK_ELAPSED: 0s` and
`WALLCLOCK_LIMIT: none (evaluation wall clock not imposed…)` ⇒ **a residual render surface is
really reachable, but with no limit, no remaining, and elapsed = 0 it constitutes no usable
time signal**; the two-surface teardown (0cg) should still proceed.

### 6.3 Causal boundary (no single-cause attribution)

The rerun3 carrier was 0.8.7, while the original attempts mixed 0.8.4/0.8.5/0.8.7 ⇒ the
cross-generation difference and the "no wall clock" condition were **not separated
experimentally**. This report claims only "conditions held" and "the in/out readings" — not
the single-cause conclusion "scores are unaffected by the wall clock".

---

## 7. Failure dissection (16 final failures)

| Shape | Count | Tasks |
|---|---|---|
| Official-timeout kill (AgentTimeoutError, no delivery / no pass) | 10 | configure-git-webserver, git-multibranch, tune-mjcf, gpt2-codegolf, caffe-cifar-10, extract-moves-from-video, adaptive-rejection-sampler, train-fasttext, make-doom-for-mips, chess-best-move |
| Self-completed, judged fail (model declared completion, verifier scored 0) | 5 | qemu-startup, winning-avg-corewars, pytorch-model-cli, pypi-server, hf-model-inference |
| Abnormal exit (NonZeroAgentExitCodeError; model stream degradation cut off by the sentinel) | 1 | fix-code-vulnerability |

Comparison: **8 at-the-wall passes** (hard-killed by the official timeout, but delivery was
already complete and scored pass) — break-filter-js-from-html, torch-pipeline-parallelism,
circuit-fibsqrt, torch-tensor-parallelism, filter-js-from-html, query-optimize,
model-extraction-relu-logits, qemu-alpine-ssh. The 8 at-the-wall passes and the 10 timeout
kills point at the same thing: **time-budget allocation**.

Historical unsolved pool (8 members of the previous generation's unsolved15 pool entered this
round): 7 turned around; only make-doom-for-mips did not — the pool label has real predictive
power, but the overall pessimistic prior was falsified.

---

## 8. Apparatus friction and fix history

1. **Mechanical write control too broad** (first half: 215 commands intercepted across 43/44
   tasks; build-pov-ray's required `install to /usr/local/bin` and its `+O/dev/null` check
   were structurally intercepted even though the model had already compiled and rendered
   successfully) → 0.8.5 backstop rewrite (catastrophic block rules as a closed enumeration)
   → 0.8.6/0.8.7 narrowed container-side carrier self-protection into the **host-machine
   catastrophic backstop** (rule 5's container face retired; the `.gsa` host bind-mount face
   kept). The causal chain was verified by the 5-task targeted rerun (2 turnarounds).
2. **Debian security repo index inconsistent with the package pool** (`bullseye-security`
   advertises packages like `curl_…deb11u16` that no longer exist in the pool; neither a
   fresh USTC index nor snapshot archives helped) → **no adapter change needed**: mount a
   corrected `/etc/apt/sources.list` at batch time (that one file only); end-to-end probe
   `orz install check: orz-ready`, a full attempt at 10m25s, 0 anomalies ⇒ the two qemu tasks
   went from "job-level failure" to real attempts (0.0 / 1.0). This mount was the round's
   **only environmental deviation** (§9).
3. **Backstop granularity mismatch** (proven by rerun3): 11/18 tasks showed "device-directory
   writes refused" (apt/dpkg/git/sshd all need to write `/dev/null`, while the L3 grant table
   excluded the whole `/dev` subtree even though L1/L2 explicitly exempt `/dev/null`); 3 tasks
   added "cannot create at root" (`mkdir /git` EACCES — the L3 grant table was generated by
   enumerating `/`'s top-level entries, so `/` itself was absent). Filed as **0ch** (backstop
   precision, two sub-items in the 0.8.8 generation window).
4. **Scoring / completion detection defects**: harbor 0.20 job summaries carry no `trials`
   array; the early "file exists = done" check misjudged zombie jobs (b1-01, b4-14); replaced
   with "a `reward` entity must exist"; a rerun-job authority mapping was added.
5. **Procedural mis-starts** (b1-01 ≈1 min, b4-14 twice, b4-15 ≈3 min) were all stopped and
   cleaned immediately, **produced no official results, and count toward nothing**.

---

## 9. Disclosure checklist (checked item by item)

| # | Requirement | Status |
|---|---|---|
| 1 | k=1 screening-round qualifier; not a leaderboard score (leaderboard needs ≥5 attempts/task) | §1 / §2 |
| 2 | Apparatus defect fixed mid-round; reruns selected by a mechanical threshold and **replacing** originals; originals fully archived | §5 / §8 |
| 3 | Per-job identity hashes (carrier / adapter / dataset pin) archived, cross-generation verifiable | per-job `*-round.log` plan lines + §2 |
| 4 | **Two carrier generations mixed**: 0.8.4 (40 tasks) + 0.8.7 (49 tasks); one 0.8.5 run as lineage note; 0.8.6 never served | §2 / §5.1 |
| 5 | **Two rerun rounds**: rerun2 (5 tasks, mechanical threshold) + rerun3 (18 tasks, complete wall-clock exposure surface); both replacements, not best-of | §5 |
| 6 | **Environmental deviation**: the container's `/etc/apt/sources.list` was replaced at batch time for the two qemu tasks (that one file only; image content, task files, tests, pins untouched); the other 16 (of the 18-task set) have no such deviation | §5.2 / §8.2 |
| 7 | Evaluation protocol: k=1 no cherry-picking, single-task serial, official dataset pin, no environment hardening, task-level and attempt-level both reported | §2 / §3 |
| 8 | Job-level failures and undecided cases listed honestly (two environment-level failures, zombie summaries voided and rerun, mis-starts with no results) | §3.2 / §8.4 / §8.5 |
| 9 | Execution-form evolution (batch → single-task harvest → rerun3 per-task check) and peak/off-peak billing windows disclosed | §2 / §8 |

---

## 10. Data integrity and account corrections (this report vs earlier accounts)

Every per-task reading here comes from the `reward` entities in `result.json` and is
mechanically recomputable. Checking earlier accounts against it surfaced **one systematic
deviation**:

- Adjudication doc §9 (2026-09-30 morning) "56 decided tasks = 46/56 = 82.1%" matches this
  report's mechanical recount;
- But from §11.5 (second-window close) onward, the rolling tallies **double-counted** 2
  turnaround tasks already included in §9, making §11.5's "53/63 = 84.1%" and §11.9's
  "74/87 = 85.1% (83.1% over 89)" each **2 tasks too high**;
- Mechanically verified values: first-pass account **72/87 = 82.8% (80.9% over all 89)**;
  final **73/89 = 82.0%**.

Disposition: the closeout batch corrected the two figures in adjudication §11.5/§11.9 and the
round doc §11, with the note "per-job reward entities govern"; **this report's numbers take
precedence**.

---

## 11. Boundaries and non-extrapolations

1. k=1, single attempt: no variance information; leaderboard-grade k≥5 scores cannot be
   inferred from it.
2. The rerun3 study set = the **complete** wall-clock exposure surface + 2 merged tasks,
   **not a sample of the round**; its "net effect exactly zero" does not extrapolate to the
   other 71 tasks, nor does it constitute the general claim "wall-clock visibility never
   affects scores" (§6.3).
3. Cross-generation comparison does not hold: the 0.8.4 → 0.8.7 differences co-vary with the
   "apparatus fixes"; no separation experiment was run.
4. The official 90.6 and a third-party ~87.9 are both different protocols; reference lines
   only.
5. Time statistics include image pre-pull and wrap-up, not pure agent runtime; the container
   memory boundary (Docker VM ≈7.7 GB vs 8192 MB for memory-heavy tasks) was mitigated by
   serialization, per the round-0 precedent, without changing host system configuration.

---

## 12. Reproduction entry points

| Purpose | Location |
|---|---|
| Frozen task order & 89-task manifest | [`evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json`](../../evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json) + `D:/tb-eval/run_official_2.1.sh` |
| Per-task job readings (authoritative) | `D:/tb-eval/jobs-official/<job>/result.json` (`stats.evals.*.reward_stats.reward`) |
| Per-task apparatus identity & wall clock | `plan` line JSON in `D:/tb-eval/jobs-official/<job>-round.log` |
| Trajectories & journal volumes | `D:/tb-eval/gsa-volumes/<job>/` (`events.jsonl` is the only machine-readable record) |
| Main-round accounts | `D:/tb-eval/jobs-official/official-v41-full-round.log`, `official-v41-second-half-round.log` |
| rerun3 verdicts & checks | `D:/tb-eval/jobs-official/official-v41-rerun3-round.log` (`[rerun3-verdict]` / `[rerun3-check]`) |
| rerun3 driver & checker | [`scripts/run_puller_control.py`](../../scripts/run_puller_control.py) |
| apt-source fix mount file | [`scripts/aptfix-bullseye-sources.list`](../../scripts/aptfix-bullseye-sources.list) |
| This report's extraction scripts (working-tree temporaries) | `.tmp-report-89.py` / `.tmp-report-build.py` / `.tmp-scan-all-jobs.py` / `.tmp-report-time.py` (`.tmp-*` not committed) |

> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Task Checklist Announcement Supplement v0.1

Status: design supplement; intended to extend `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`, `PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md`, and the existing Orientation Runtime Guard without changing hard gate semantics.

## 1. Purpose

This supplement freezes the user-facing and model-facing task checklist design for the main CLI conversation view.

The goal is to give the primary AI a stable external task memory while keeping the user interface light. The checklist should help the model track the approved plan, current step, soft constraints, and runtime notes without turning the CLI into a rigid workflow engine.

The design follows the current main-window direction: a chatroom-like primary conversation area with a persistent top announcement strip. The announcement strip shows the active task checklist in compact form; expanded details expose the plan annotations and runtime records.

## 2. Non-Goals

- Do not introduce a new hard constraint system.
- Do not replace task contract, GPS, run journal, or Orientation Runtime Guard.
- Do not ask the model whether it is wrong, biased, drifting, or stuck.
- Do not infer task completion from checklist display state alone.
- Do not expose provider-private reasoning or hidden chain-of-thought.
- Do not require every task to use the same number of steps.

## 3. Concept

The checklist has two synchronized views:

1. Compact announcement view: always visible near the top edge of the main conversation window.
2. Expanded checklist page: opened by user action and available to the model/runtime as structured task memory.

The compact view shows only the current work items:

```text
[Task]
GAK-01 done  Survey existing orientation guard
GAK-02 doing Freeze checklist supplement
GAK-03 todo  Register index entry
GAK-04 todo  Verify document references
```

The expanded view includes annotations:

```text
GAK-02 Freeze checklist supplement
status: doing
source: approved plan revision 3
acceptance_refs: UI-ANNOUNCE, PLAN-ANNOTATIONS, ORIENT-NEUTRAL
soft_constraints:
- Do not add hard gates in this slice.
- Keep Orientation Runtime Guard neutral.
runtime_records:
- Existing orientation checkpoint already forbids correctness/drift questions.
- Stagnation guard uses public-output-only repetition signals.
next_review_trigger: after step completion or before declaring task complete
```

## 4. Step IDs

Each checklist item must have a stable, short step ID:

```text
<task-part-code>-<two-digit-sequence>
```

Examples:

- `GAK-01`
- `UI-02`
- `PLAN-03`
- `GPS-04`
- `LBR-05`

The task-part code should come from the current task area, component ID, or locally meaningful abbreviation. The sequence should be stable for the current plan revision. If a step is split or replaced, create a new plan revision instead of silently reusing the old ID for a different meaning.

## 5. Status Vocabulary

The UI may localize the labels, but the underlying state vocabulary should stay small:

```text
todo | doing | done | blocked | deferred | replanned
```

Suggested display mapping:

```text
todo      .
doing     >
done      done
blocked   !
deferred  ~
replanned *
```

The checklist state is a work coordination signal. It is not evidence of final correctness, verification, or claim strength.

## 6. Plan Annotations

After Plan mode completes and the user approves the plan, the runtime should derive checklist items and attach plan annotations to each item.

Annotations should preserve:

- approved plan revision
- source section or source message digest
- user-visible objective served by the step
- acceptance references
- soft constraints
- assumptions
- deferred decisions
- expected verification or review trigger

Annotations should not preserve:

- hidden chain-of-thought
- provider-private reasoning
- unverifiable guesses about model confidence
- hard permissions that were not explicitly granted elsewhere

Plan annotations are soft task memory. They help the AI orient itself, but they do not override system policy, user instructions, permissions, evidence gates, or source-visibility rules.

## 7. User Visibility

The compact checklist is always user-visible when a task is active. The detailed annotations are collapsed by default, but they are not secret.

The user must be able to expand the checklist page and inspect:

- full step titles
- status history
- soft constraints
- runtime records
- plan revision
- source links or artifact references
- unresolved notes

This keeps the normal interface calm without creating hidden control text that the user cannot audit.

## 8. Relationship to Orientation Runtime Guard

The existing mechanism is called **Orientation Runtime Guard**. Its two relevant parts are:

- **orientation checkpoint**: a neutral task-position checkpoint
- **runtime stagnation guard**: a public-output-only repetition detector

This supplement keeps that mechanism. The checklist becomes one source of neutral orientation context; the guard remains the review action.

The orientation checkpoint may read compact task memory such as:

- current step ID and title
- current task position
- next output target
- available tool acknowledgement

It must not ask:

- whether the current action is correct
- whether the model is biased
- whether the task has drifted
- whether the model is stuck

It must not emit:

- counterexample candidates
- claim disposition
- claim promotion
- hard constraint changes

The current Orientation Runtime Guard design already matches this boundary: `asks_model_if_stuck` remains false, `claim_strength_effect` remains none, and counterexample/claim-disposition fields are forbidden.

## 9. Relationship to Runtime Stagnation

Runtime stagnation handling should stay mechanical and low frequency.

It may act when public outputs show:

- consecutive repeated content above threshold
- repeated N-gram loops above threshold
- explicit mention of a known unavailable tool name

It should not act merely because:

- the model spends several turns on one checklist item
- the user-approved plan has many steps in one area
- the model chooses a different implementation route within the approved soft constraints
- a step annotation says a review is useful

For normal task progress, the outcome is a record or reminder, not a restart.

## 10. Relationship to GPS

Global Progress Sentinel remains the structured audit/progress layer. The checklist is the user-facing and model-facing workboard layer.

Recommended mapping:

```text
checklist item id      -> GPS step_id
task-part code         -> direction_id candidate
status                 -> progress item state
acceptance refs        -> acceptance_coverage refs
runtime records        -> source_refs / artifact refs when registered
plan revision          -> GPS plan_revision
```

GPS may derive warnings from registered events and plan state. The checklist should not synthesize GPS warnings by itself.

## 11. Update Rules

The checklist should update at task boundaries:

- after plan approval
- when a step starts
- when a step completes, fails, blocks, defers, or is replanned
- before destructive or external side-effect actions
- before handoff
- before declaring the task complete

It should not update after every trivial internal action. Excessive updates would create warning fatigue and weaken the chatroom-style interface.

## 12. UI Behavior

Compact announcement strip:

- fixed near the top edge of the main conversation view
- shows 3-7 active items by default
- prioritizes doing, blocked, and next todo
- supports keyboard focus and click/enter expansion
- does not wrap into a large wall of text

Expanded checklist page:

- shows full worklist
- groups replanned/deferred items below active items
- exposes plan annotations and runtime records
- links to conversation segments, artifacts, and run events when available
- allows review without leaving the current task context

Finished task behavior:

- compact checklist may collapse into a final summary row
- expanded checklist remains available from the task history
- verification failures, blockers, and unresolved notes must stay visible

## 13. Implementation Slices

Recommended order:

1. Checklist view model: define in-memory item/status/annotation structures.
2. Plan mode bridge: derive checklist items from an approved plan result.
3. Announcement strip projection: render compact active items in the main conversation view.
4. Expanded checklist page: show annotations, status history, and runtime records.
5. Orientation context bridge: expose current step and next output target to orientation checkpoint generation.
6. Journal/artifact registration: persist checklist revisions and status changes.
7. GPS mapping: map checklist IDs and status changes into GPS-compatible progress inputs.

## 14. Frozen Decisions

- The checklist is a soft workboard, not a hard constraint engine.
- The compact checklist is user-visible; detailed annotations are collapsed, not secret.
- Plan approval produces checklist items plus plan annotations.
- Step IDs use a task-part code plus a stable sequence number.
- Orientation Runtime Guard stays enabled and neutral.
- The system must not ask the model whether it is biased, wrong, drifting, or stuck.
- Runtime stagnation remains public-output-only and mechanical.
- The checklist assists AI task memory and user review without reducing the model to a fixed script.

## 15. Deferred Decisions

- Exact visual density of the announcement strip.
- Whether the compact strip shows symbolic or localized textual statuses by default.
- Exact schema name for persisted checklist revisions.
- Whether checklist edits by the user are direct edits or plan-revision requests.
- How much checklist history remains visible after long multi-session handoff.

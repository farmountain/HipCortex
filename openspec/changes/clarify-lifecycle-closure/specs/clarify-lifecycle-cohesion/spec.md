# Spec: Clarify-Lifecycle Cohesion

## Overview

The clarifying-clarity lifecycle is implemented across three layers — the T0→T3 ladder
(`clarify_engine.rs`), the four lifecycle phases (`agent_guidance.rs`), and the policy text
in `install/SKILL.md`. Each layer is individually sound; the defects are at the seams, so
no single file read in isolation reveals them.

Three seam defects are closed here:

1. **Phase 1 output is rejected by the engine's own gate.** `clarify_goal` returns
   `suggested_success_factors` only. `harness.md` requires non-empty `acceptance_criteria`
   or `POST /goal/:id/clarify` blocks the loop, and `ClarifyTrigger::EmptyAC` fires on
   empty while `UntestableAC` fires when no factor carries an `observation_pattern`. Every
   task routed through the guidance layer stalls on iteration one.
2. **Uncertainty is detected and dropped.** `check_progress` computes
   `uncertainty_detected`; `ClarifyTrigger` has no uncertainty variant. The requested
   lifecycle names *"unknown/uncertainty"* explicitly.
3. **The self-prompt-first ordering is asserted nowhere.** It is a consequence of control
   flow in `clarify_engine.rs`, and untested. Compounding it, `install/SKILL.md` step 1
   says `(if uncertainty_flags non-empty: ask user before proceeding)` — instructing the
   agent to ask the user before any self-prompt tier runs.

This capability makes the lifecycle cohesive: one source of truth per decision, and the
ordering invariant asserted rather than assumed.

## Requirements

- **Phase 1 emits decidable acceptance criteria.** `clarify_goal` returns
  `acceptance_criteria: Vec<AcceptanceCriterion>`, one per suggested success factor, each
  with a non-empty `observation_pattern`. Unrecognised factors fall back to the factor name
  as their pattern — **never** the empty string, since an empty pattern re-opens
  `UntestableAC`, the exact defect being closed.
- **Additive only.** `GoalClarification` gains one field carrying `#[serde(default)]` so
  payloads persisted before this change still deserialize. No existing field changes name,
  type, or meaning. `plan_validation`'s output shape is unchanged: the classifier used for
  `observation_pattern` is a deliberate, documented duplication, because the two
  classifiers answer different questions (`plan_validation` → *how to verify*;
  `acceptance_criteria` → *what token proves it*).
- **Uncertainty routes, it does not trigger.** A `route_uncertainty` function maps
  `uncertainty_detected` plus the ladder state onto
  `ClarifyRoute { NoClarification | SelfPrompt | AskUser | DeclineAsk }`. It is **not** a
  fifth `ClarifyTrigger`: a trigger implies a new class of ambiguity entering the ladder,
  when uncertainty is the ordinary signal that the existing tiers should be walked.
- **The gate is shared, not reimplemented.** `route_uncertainty` imports `COST_OF_ASKING`,
  `P_UNRESOLVABLE_AFTER_LADDER`, `P_UNRESOLVABLE_INCOMPLETE`, `MAX_CLARIFY_TIERS` and
  `ClarifyTier::search_rungs()` from `clarify_engine`. `search_incomplete` selects which
  probability applies, mirroring the engine's own distinction between *searched and empty*
  and *never searched*.
- **Self-prompting has priority; the user is asked only when critical.** For every state in
  which a search rung is unspent, the route is `SelfPrompt` — independent of cost. Asking is
  considered only once the ladder is spent, and only if
  `P(unresolvable) × cost_of_wrong_execution > COST_OF_ASKING`.
- **The ordering rule is stated in the policy.** `install/SKILL.md` replaces
  `(if uncertainty_flags non-empty: ask user before proceeding)` with an explicit ordered
  rule: walk the tiers, then `plan_validation`, then ask the user **one** question only if
  the ladder is spent *and* it is critical.
- **Exit bounds are untouched.** At most `MAX_CLARIFY_TIERS` rungs, at most
  `MAX_CLARIFY_CYCLES_PER_GOAL` invocations, T3 terminal, `substrate_daemon`'s per-tick
  re-invocation bounded. `clarify_engine.rs` is changed by **zero** characters.

## Acceptance Criteria

- `clarify_goal` returns non-empty `acceptance_criteria` for all five task classes (vague,
  open-ended, dev, web, default), and every `observation_pattern` is non-empty. Verified by
  test.
- A goal built from `clarify_goal(...)`'s criteria satisfies `UntestableAC`'s precondition
  being false — every factor carries an `observation_pattern`. Verified by test.
- For all `tiers_spent < MAX_CLARIFY_TIERS` and a cost grid spanning far below and far
  above the gate threshold, `route_uncertainty` returns `SelfPrompt`, never `AskUser`.
  Verified by test.
- At `tiers_spent == MAX_CLARIFY_TIERS`, the route is `AskUser` when the gate fires and
  `DeclineAsk` when it does not — so the ordering test cannot pass vacuously. Verified by
  test.
- `uncertainty_detected == false` returns `NoClarification`; `tiers_spent` beyond the rung
  count yields no additional rung. Verified by test.
- The package template contains the ordering rule. Verified by test.
- `clarify_engine.rs`'s existing tests pass **unmodified**.

## Non-Regression

- `GoalClarification`'s existing fields keep their names, types, and meanings. Callers
  reading only `suggested_success_factors` are unaffected.
- Deserializing a `GoalClarification` persisted without `acceptance_criteria` succeeds and
  yields an empty vector.
- No test is deleted, `#[ignore]`d, or relaxed. The Rust test count strictly increases.
- No constant in `clarify_engine.rs` is changed. In particular `MAX_CLARIFY_TIERS`,
  `MAX_CLARIFY_CYCLES_PER_GOAL`, `COST_OF_ASKING`, `P_UNRESOLVABLE_*` and the
  `default_b0() = MAX_CLARIFY_CYCLES_PER_GOAL × C_MIN` self-consistency are untouched.

## Dependencies

- Consumes `clarify_engine`'s public constants and `ClarifyTier` (both already `pub`).
- Consumed by `harness-skill-freshness`, which detects when the policy text is stale — the
  two are complementary: this spec makes the mechanism correct, freshness makes its
  absence visible.
- `docs/harness.md` describes the ladder and the clarify gate; the ordering rule added to
  the SKILL is consistent with it and does not contradict it.

## Risks & Mitigations

- **The duplicated factor classifier drifts from `plan_validation`'s.** Accepted and
  documented rather than silent. They answer different questions and are separately tested;
  unifying them would rescope `plan_validation`'s asserted output. If they ever must agree,
  the place to unify is a shared classifier both call — a separate change.
- **A stricter route function could start refusing to ask.** Mitigated by the negative
  direction test: the gate must still produce `AskUser` when it fires.
- **Adding a field to a `Deserialize` struct.** Mitigated by `#[serde(default)]` and by the
  fact that the struct has a single construction site.
- **Scope creep into `clarify_engine.rs`.** Mitigated by the Non-Goal being stated in both
  `proposal.md` and the task list, and by verification being a zero-character diff on that
  file.

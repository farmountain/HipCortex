# Proposal: Clarify-Lifecycle Closure

## Why

HipCortex claims a *clarifying-clarity* lifecycle: a vague task is made decidable by
**self-prompting before the user is ever asked**, and only *critical* questions reach a
human. Two artifacts already implement parts of that claim:

- `src/clarify_engine.rs` — the T0→T3 strictly-descending ladder, with an ask-cost gate
  (`T3 asks iff P(unresolvable) × cost_of_wrong_execution > cost_of_asking`) and six
  enumerated exit guarantees.
- `src/agent_guidance.rs` — four lifecycle phases (`clarify_goal`, `plan_validation`,
  `check_progress`, `should_exit`).

Exploration of the shipped tree found **four concrete defects where the claim and the code
diverge**. Each is verifiable by reading two files; none is a matter of taste.

### D1 — Phase 1 manufactures goals the clarify gate then rejects

`agent_guidance::clarify_goal` returns `suggested_success_factors` and nothing else. It
never emits `acceptance_criteria`.

`docs/harness.md` states: *"`acceptance_criteria` must be non-empty or
`POST /goal/:id/clarify` will block the loop."* `ClarifyTrigger::EmptyAC` fires on exactly
that. `ClarifyTrigger::UntestableAC` fires when AC exists but no factor carries an
`observation_pattern`.

So the guidance layer's output is rejected by the engine layer's own gate, on the first
iteration, for every task. The self-prompting pipeline cannot start.

### D2 — `unknown/uncertainty` has no route into the ladder

The requested lifecycle covers *"unknown/uncertainty"*. `check_progress` computes
`uncertainty_detected`, and `ClarifyTrigger` enumerates `EmptyAC`, `UntestableAC`,
`RepeatedVeto`, `PreSuccess` — **there is no uncertainty trigger**. Uncertainty is detected
and then dropped. Nothing consumes it.

### D3 — the self-prompt-before-user ordering is asserted nowhere

This is the user's stated invariant: *"self prompting (higher priority than clarifying,
when self prompting can resolve then self prompting, only those critical questions should
clarify with users)"*.

`clarify_engine.rs` is *designed* to honour it — the ladder descends T0→T1→T2→T3 and only
T3 spends human attention. But **no test asserts the ordering property**. The existing
tests assert tier monotonicity and exhaustion; none asserts that `ask_or_decline` is
unreachable while a lower tier can still advance. An invariant that is only a consequence
of control flow, and is untested, is one refactor away from silent loss.

### D4 — the installed skill is stale, so the mechanism is not on the running surface

`sdk/python/hipcortex/install/SKILL.md` (the template) contains a
`## Lifecycle self-prompting` section: `clarify_goal` → `plan_validation` → the ReAct loop →
`should_exit`, with four hard exit rules.

The installed `~/.claude/skills/hipcortex/SKILL.md` on this machine **does not contain that
section at all**. The capability is documented, implemented, tested — and absent from the
surface where the agent actually reads its policy.

## What Changes

Four surgical edits. No new subsystem, no new abstraction layer.

1. **`agent_guidance.rs`** — `clarify_goal` additionally emits machine-decidable
   `acceptance_criteria` (each with an `observation_pattern`), so Phase 1 output survives
   `EmptyAC` **and** `UntestableAC`.
2. **`agent_guidance.rs`** — a `route_uncertainty` function that turns
   `ProgressCheck.uncertainty_detected` into a ladder position, reusing
   `clarify_engine`'s ask-cost gate constants so the two layers cannot drift.
3. **tests** — the ordering invariant, and the exit bounds, asserted directly.
4. **`install/SKILL.md`** — the ordering rule stated explicitly, plus a freshness
   mechanism so a stale install is detectable rather than silent.

## Non-Goals

- Rewriting the ladder. `clarify_engine.rs` is correct and its tier logic is untouched.
- Changing any constant in `clarify_engine.rs`. `MAX_CLARIFY_TIERS`,
  `MAX_CLARIFY_CYCLES_PER_GOAL`, `COST_OF_ASKING`, `P_UNRESOLVABLE_*` are load-bearing and
  already self-consistent (`default_b0()` is defined from `MAX_CLARIFY_CYCLES_PER_GOAL`).
- Loosening any exit guarantee. The six in the `clarify_engine.rs` module docstring stay
  exactly as they are.
- Rescoping `plan_validation`'s output shape, which is asserted by existing tests.

## Success Criteria (Karpathy Goal-Driven)

Every criterion is binary and machine-checkable.

1. `clarify_goal(task).acceptance_criteria` is non-empty for every task class the function
   classifies (vague, dev, web, open-ended, default), and every emitted criterion carries a
   non-empty `observation_pattern`.
2. A goal built from `clarify_goal(...).acceptance_criteria` does not trigger `EmptyAC`.
3. `route_uncertainty` returns a ladder tier (never T3) while a lower tier remains
   unspent, and returns the ask decision only when the ask-cost gate fires or the ladder is
   exhausted.
4. A test asserts: for every reachable ladder state in which a lower tier can still
   advance, the returned route is a self-prompt tier, not `AskUser`.
5. The exit bounds remain: at most `MAX_CLARIFY_TIERS` rungs, at most
   `MAX_CLARIFY_CYCLES_PER_GOAL` invocations, T3 terminal.
6. `install/SKILL.md` states the self-prompt-first ordering rule explicitly.
7. `cargo test` passes with no test weakened, deleted, or `#[ignore]`d. Test count strictly
   increases.
8. No existing public signature in `agent_guidance.rs` changes; only additions.

## Impact

- **Blast radius:** additive. `GoalClarification` gains a field — struct-literal
  construction sites are the only breakage risk and are enumerated in `design.md`.
- **Behavioral:** none for existing callers that read only `suggested_success_factors`.
- **Freshness:** the SKILL freshness item is the one with operational value beyond the
  repo — it is what turns D4 from a repeated incident into a detected condition.

## Open Questions (None Remaining)

Resolved by exploration, not by asking the user:

- *Is the ladder the right shape?* Yes — `clarify_engine.rs`'s docstring records that the
  predecessor (`MAX_CLARIFY_ROUNDS`, "the v3.6 shape") re-ran the same belief search three
  times, so the counter was decorative. Monotone descent is the fix and is already in.
- *Should uncertainty become a fifth `ClarifyTrigger`?* No. A trigger implies a new ladder
  entry point; the uncertainty signal belongs at the *route* layer, where it can select an
  already-existing tier. Adding a trigger would change `clarify_engine.rs`, which is out of
  scope by Non-Goals.
- *Is a hard `<HARD-GATE>`-style user-approval step required?* The user is unavailable and
  instructed autonomous completion with self-prompting having priority. Questions that the
  substrate can settle are settled here; the remainder are stated as assumptions in
  `design.md` §Assumptions.

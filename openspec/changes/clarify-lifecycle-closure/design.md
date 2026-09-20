# Design: Clarify-Lifecycle Closure

## Context

Three artifacts jointly claim the clarifying-clarity lifecycle:

| Layer | Artifact | Provides |
|---|---|---|
| Engine | `src/clarify_engine.rs` | T0→T3 strictly-descending ladder, ask-cost gate, six exit guarantees |
| Guidance | `src/agent_guidance.rs` | `clarify_goal`, `plan_validation`, `check_progress`, `should_exit` |
| Policy | `install/SKILL.md` → `~/.claude/skills/hipcortex/SKILL.md` | the text the agent actually reads |
| Detection | `sdk/python/hipcortex/doctor.py` | `skill_package` / `skill_installed` staleness checks |

Each layer is individually sound. The defects are at the **seams between them** — which is
why reading any one file in isolation shows nothing wrong.

## Decision 1 — Phase 1 must emit acceptance criteria, not just factor names

**Problem (D1).** `clarify_goal` returns `suggested_success_factors`. `harness.md`:
*"`acceptance_criteria` must be non-empty or `POST /goal/:id/clarify` will block the loop."*
`ClarifyTrigger::EmptyAC` fires on empty; `ClarifyTrigger::UntestableAC` fires when AC
exists but **no factor carries an `observation_pattern`**. Phase 1 output is therefore
rejected by the engine's own gate.

**Change.** Add to `GoalClarification`:

```rust
pub struct AcceptanceCriterion {
    pub statement: String,
    pub success_factor: String,
    pub observation_pattern: String,
}

pub struct GoalClarification {
    // ... existing fields unchanged ...
    #[serde(default)]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}
```

`clarify_goal` populates it one-to-one from the factors it already computes.

**Why `#[serde(default)]`:** `GoalClarification` derives `Deserialize`. A required field
would break payloads persisted before this change. The default is the empty vector, which
is the honest pre-change value.

**Why the pattern can never be empty.** An empty `observation_pattern` would re-open
`UntestableAC` — the exact defect being closed. So the fallback for an unrecognised factor
is the factor name itself, never `""`. This is asserted by test.

**Rejected alternative: making `observation_pattern` a field on `ValidationStep`.** That
would rescope `plan_validation`'s output, which `test_plan_validation_known_factors`
asserts. Non-Goal. The classifier is duplicated as a consequence, and that duplication is
accepted *deliberately* rather than silently: the two ladders answer different questions
(`plan_validation` → how to verify; `acceptance_criteria` → what token proves it) and
unifying them would change the former's contract.

## Decision 2 — uncertainty routes to a ladder tier, it does not become a trigger

**Problem (D2).** The requested lifecycle includes *"unknown/uncertainty"*.
`check_progress` computes `uncertainty_detected`. `ClarifyTrigger` enumerates `EmptyAC`,
`UntestableAC`, `RepeatedVeto`, `PreSuccess` — no uncertainty trigger. The signal is
produced and dropped.

**Change.** Add a route function to `agent_guidance.rs`:

```rust
pub enum ClarifyRoute {
    NoClarification { rationale: String },
    SelfPrompt { tier: String, rationale: String },
    AskUser { tier: String, rationale: String },
    DeclineAsk { tier: String, rationale: String },
}

pub fn route_uncertainty(
    uncertainty_detected: bool,
    tiers_spent: u32,
    search_incomplete: bool,
    cost_of_wrong_execution: f64,
) -> ClarifyRoute;
```

**Why not a fifth `ClarifyTrigger`.** A trigger is an entry point into the ladder; adding
one changes `clarify_engine.rs`, which is a Non-Goal, and would imply uncertainty is a new
*class* of ambiguity when it is in fact the ordinary signal that the existing tiers should
be walked.

**Why the constants are imported, not redefined.** `route_uncertainty` reads
`COST_OF_ASKING`, `P_UNRESOLVABLE_AFTER_LADDER`, `P_UNRESOLVABLE_INCOMPLETE`,
`MAX_CLARIFY_TIERS` and `ClarifyTier::search_rungs()` from `clarify_engine`. The gate is
not duplicated — only its parameters are shared, so the two layers cannot drift apart.
`search_incomplete` selects the probability exactly as `clarify_engine` distinguishes
*searched-and-empty* from *never-searched*.

## Decision 3 — the ordering invariant is asserted, not assumed

**Problem (D3).** The user's invariant — self-prompt first, user only for critical
questions — is today a *consequence of control flow* in `clarify_engine.rs`. No test
asserts it. The existing tests assert tier monotonicity and exhaustion, which are
different properties.

**Change.** Assert the ordering directly in `agent_guidance.rs`:

> For every `tiers_spent < MAX_CLARIFY_TIERS`, and for every cost — including the largest
> cost for which the gate would fire — `route_uncertainty` returns `SelfPrompt`, never
> `AskUser`.

This is decisive because it is exactly the property the user named. An implementation that
asked the user early would pass every pre-existing test and fail this one.

**Also fixed:** `install/SKILL.md` step 1 reads
`(if uncertainty_flags non-empty: ask user before proceeding)` — which instructs the agent
to ask the user *whenever any flag is set*, before any self-prompt tier runs. That is the
inverse of the invariant. Replaced with an explicit ordered rule.

## Decision 4 — make the staleness detector discriminative

**Problem (D4).** `doctor.py` already detects a stale installed skill:

```python
SKILL_HARNESS_MARKERS = ("MUST", "live_beliefs")
```

and `add_skill_checks` passes `incomplete_status="fail"` for `skill_installed`. So the
mechanism is wired and configured to fail loudly. It reports **ok** on this machine because
`"MUST"` and `"live_beliefs"` appear in **both** the lifecycle revision and the older
conservative revision. The markers are non-discriminative; the detector cannot see the
difference it exists to detect.

Measured on this machine: installed `~/.claude/skills/hipcortex/SKILL.md` is 162 lines,
goes `intro → ## When to use → MUST:`, and contains no `## Lifecycle self-prompting`
section and no `should_exit`. The package template reads *"Soft harness prefers / nudges
substrate-first"*; the installed copy reads *"Harness enforces substrate-first"*. The
installed file is an earlier revision — and `hipcortex doctor` reports `skill_installed:
ok` for it.

**Change.**

```python
SKILL_HARNESS_MARKERS = (
    "MUST",
    "live_beliefs",
    "Lifecycle self-prompting",
    "should_exit",
)
```

Both additions appear in the template and neither appears in the pre-lifecycle revision.

**Consequence for tests.** `_GOOD_SKILL` in `test_doctor.py` is a synthetic minimal skill
that stands for "an acceptable install". It must gain the new markers or it would now
describe a stale install. This is a **fixture update required by the change**, not a
relaxation: every assertion keeps its exact form, and the fixtures become *more*
demanding. `test_skill_missing_markers_case_sensitive` is the one test whose expected
list changes, because it asserts the marker set exhaustively — updated to the new
4-tuple.

**New regression test that would have caught D4:** a skill containing `MUST` and
`live_beliefs` but neither lifecycle marker must be reported `fail`. That is precisely the
stale install on this machine.

## Assumptions (explicit, per Karpathy #1)

1. **"The above gaps" = the seams between the clarify layers, evidenced by inspection.**
   The three artifacts that could plausibly own the request were read. `clarify_engine.rs`
   and `agent_guidance.rs` exist and are sound; `install/SKILL.md` contains a
   `## Lifecycle self-prompting` section the installed copy lacks. The four defects above
   are what inspection turned up, each verifiable by reading two files. No competing
   reading survived contact with the tree.
2. **Self-prompting outranks asking.** Taken from the user's words verbatim
   (*"self prompting (higher priority than clarifying…)"*). Where it conflicts with
   `brainstorming`'s `<HARD-GATE>` (which requires a user-approval step before
   implementation), the user's explicit instruction wins. Recorded here rather than applied
   silently; see Risks.
3. **The engine layer is correct and is not respecified.** `clarify_engine.rs` is edited by
   exactly zero characters.
4. **A goal's `acceptance_criteria` may legitimately be empty at *creation*.** The engine's
   `EmptyAC` trigger exists for goals created by callers other than `clarify_goal`.
   Decision 1 guarantees the *guidance* path never produces one; it does not remove the
   trigger.

## Verification

| # | Criterion | How |
|---|---|---|
| 1 | AC non-empty + patterns non-empty for all task classes | `cargo test agent_guidance` |
| 2 | AC output cannot trigger `EmptyAC`/`UntestableAC` | same, asserted on the property not the enum |
| 3 | Ordering: no `AskUser` while a tier is unspent | same, grid over all costs |
| 4 | Gate still declines when not worth asking | same |
| 5 | Exit bounds unchanged (≤3 rungs, ≤3 cycles, T3 terminal) | `cargo test clarify_engine` — unmodified |
| 6 | Stale install now fails the doctor | `pytest sdk/python/tests/test_doctor.py` |
| 7 | No test weakened/removed | test counts compared before/after |
| 8 | No existing public signature changed | manual diff — additions only |

## Rollback

Every edit is additive or a single-line replacement, in a git repository.
`clarify_engine.rs` is untouched, so reverting this change cannot affect ladder behaviour.
The two `SKILL_HARNESS_MARKERS` additions are the only edit that can make a
previously-green check go red, and that is the intended effect: it makes a real staleness
visible. The remedy for an affected install is `hipcortex install --mode proactive`.

## Risks & Mitigations

- **A stricter marker set fails installs that were fine.** Intended — such an install *is*
  stale. Mitigation: the doctor message names the missing markers, and the remedy is one
  documented command.
- **Fixture update mistaken for gate-loosening.** Mitigation: stated explicitly in
  Decision 4 and in the change's tasks; every assertion keeps its form.
- **Duplicated factor classifier** (Decision 1). Mitigation: accepted and documented, not
  silent. The two classifiers answer different questions; unifying them is out of scope.
- **Ordering test could pass vacuously** if `route_uncertainty` ignored `tiers_spent`.
  Mitigation: the test also asserts the negative direction — at `tiers_spent ==
  MAX_CLARIFY_TIERS` the route *is* `AskUser` when the gate fires. A function that always
  returned `SelfPrompt` would fail that half.

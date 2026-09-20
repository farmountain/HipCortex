# Tasks: Clarify-Lifecycle Closure

Change: `clarify-lifecycle-closure`
Specs: `clarify-lifecycle-cohesion`, `harness-skill-freshness`

Verification for this change is `cargo test` (Rust) and
`pytest sdk/python/tests/test_doctor.py` (Python). No task is complete until its own
criterion has been re-run in this session with verbatim evidence.

## 1. Acceptance criteria from Phase 1 (spec: clarify-lifecycle-cohesion, D1)

- [ ] 1.1 Add `AcceptanceCriterion { statement, success_factor, observation_pattern }` to
      `src/agent_guidance.rs`, deriving `Debug, Clone, Serialize, Deserialize, PartialEq`.
- [ ] 1.2 Add `#[serde(default)] pub acceptance_criteria: Vec<AcceptanceCriterion>` to
      `GoalClarification` (additive; no existing field touched).
- [ ] 1.3 Add `observation_pattern_for(factor) -> Option<&'static str>` — the keyword
      classifier, returning `None` for unrecognised factors.
- [ ] 1.4 Populate `acceptance_criteria` in `clarify_goal`, one criterion per suggested
      factor, with the fallback pattern set to the factor name so it can never be empty.
- [ ] 1.5 Test: for each of the five task classes (vague, open-ended, dev, web, default),
      `acceptance_criteria` is non-empty and every `observation_pattern` is non-empty.
      **Criterion 1.**
- [ ] 1.6 Test: a goal whose criteria come from `clarify_goal` carries an
      `observation_pattern` on every factor, so `UntestableAC`'s precondition does not
      hold. **Criterion 2.**

## 2. Uncertainty routes into the ladder (spec: clarify-lifecycle-cohesion, D2 + D3)

- [ ] 2.1 Add `ClarifyRoute { NoClarification, SelfPrompt, AskUser, DeclineAsk }`.
- [ ] 2.2 Add `route_uncertainty(uncertainty_detected, tiers_spent, search_incomplete,
      cost_of_wrong_execution) -> ClarifyRoute`, importing `COST_OF_ASKING`,
      `P_UNRESOLVABLE_AFTER_LADDER`, `P_UNRESOLVABLE_INCOMPLETE`, `MAX_CLARIFY_TIERS` and
      `ClarifyTier::search_rungs()` from `clarify_engine`. No gate constant is redefined.
- [ ] 2.3 Test — **the ordering invariant**: for all `tiers_spent < MAX_CLARIFY_TIERS` and
      for every cost in a grid spanning well below and well above the gate threshold, the
      route is `SelfPrompt`. **Criterion 3.**
- [ ] 2.4 Test — the negative direction: at `tiers_spent == MAX_CLARIFY_TIERS` the route is
      `AskUser` when the gate fires and `DeclineAsk` when it does not. Prevents 2.3 passing
      vacuously. **Criterion 4.**
- [ ] 2.5 Test: `uncertainty_detected == false` returns `NoClarification`.
- [ ] 2.6 Test: `tiers_spent` above the rung count does not produce a further rung; T3 is
      terminal. **Criterion 5** (with `clarify_engine`'s own tests, unmodified).

## 3. Policy text states the ordering rule (spec: clarify-lifecycle-cohesion, D3)

- [ ] 3.1 Replace `(if uncertainty_flags non-empty: ask user before proceeding)` in
      `sdk/python/hipcortex/install/SKILL.md` — it instructs asking whenever any flag is
      set, before any self-prompt tier runs, which is the inverse of the invariant.
- [ ] 3.2 Add an explicit "Clarification order (HARD — self-prompt first)" block: walk the
      tiers, then `plan_validation`, then ask the user `ONE` question only if the ladder is
      spent *and* it is critical. **Criterion 6.**
- [ ] 3.3 Test: the package template contains the ordering rule text (pins it so a future
      template edit cannot silently drop it).

## 4. Discriminative staleness detection (spec: harness-skill-freshness, D4)

- [ ] 4.1 Extend `SKILL_HARNESS_MARKERS` in `sdk/python/hipcortex/doctor.py` with
      `"Lifecycle self-prompting"` and `"should_exit"` — present in the lifecycle revision,
      absent from the earlier one.
- [ ] 4.2 Correct the two stale prose references in `doctor.py` that say the check is
      "MUST + live_beliefs" (`check_skill_file` docstring and its ok message).
- [ ] 4.3 Update the `_GOOD_SKILL` fixture in `sdk/python/tests/test_doctor.py` to carry
      the new markers. **This is a fixture update the change requires, not a relaxation** —
      every assertion keeps its exact form; the fixture becomes more demanding.
- [ ] 4.4 Update `test_skill_missing_markers_case_sensitive`, the one test that asserts the
      marker set exhaustively, to the new 4-tuple.
- [ ] 4.5 Add the regression test that D4 lacked: a skill containing `MUST` **and**
      `live_beliefs` but neither lifecycle marker is reported `fail`. That file is the
      stale install measured on this machine. **Criterion 6.**
- [ ] 4.6 Confirm `test_skill_package_template_has_harness_markers` still passes against
      the real template — it uses `skill_missing_markers` on the shipped file, so it is the
      independent check that 4.1 and 3.2 agree.

## 5. Close out

- [ ] 5.1 `cargo test` — all green. Record the test count before and after; it must
      strictly increase and nothing may be `#[ignore]`d. **Criterion 7.**
- [ ] 5.2 `pytest sdk/python/tests/test_doctor.py -q` — all green.
- [ ] 5.3 Diff review: `agent_guidance.rs` is additions plus one new struct field;
      `clarify_engine.rs` is **zero** characters changed. **Criterion 8.**
- [ ] 5.4 Record in `CHANGELOG.md` under `[Unreleased] → Fixed`: the four seam defects, and
      that `hipcortex doctor` now fails a pre-lifecycle install.
- [ ] 5.5 Operator remedy, if this machine's install is confirmed stale by 5.2:
      `hipcortex install --mode proactive`. Not performed by this change — it rewrites a
      file outside the repository.
- [ ] 5.6 `openspec archive clarify-lifecycle-closure` once 5.1–5.4 are green.

## Open item (deliberately not in scope)

- **Is the guidance layer reachable from the MCP surface?** The template instructs the
  agent to call `clarify_goal(task)` and `plan_validation(...)`. Whether those names are
  registered in `sdk/mcp/server.py`'s `TOOLS` was **not verified** in this change — the
  file is outside the token budget available for this pass. If they are absent, the
  guidance layer is documented but unreachable, which is a fifth seam defect of the same
  family as D4. Tracked here as a follow-up so it is not lost, and explicitly *not*
  claimed as fixed.

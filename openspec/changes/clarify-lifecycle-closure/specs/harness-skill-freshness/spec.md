# Spec: Harness Skill Freshness

## Overview

The lifecycle policy lives in `sdk/python/hipcortex/install/SKILL.md` and is copied to
`~/.claude/skills/hipcortex/SKILL.md` by the installer. The installed file is the surface
where the agent reads its policy — the template is the unit of truth, but the *installed*
copy is what governs behaviour.

`doctor.py` already ships a detector for this:

- `package_skill_path()` → the shipped template; `installed_skill_path()` → the installed copy
- `skill_missing_markers(text)` → markers absent from a skill file
- `add_skill_checks(...)` → `skill_package` fails when bad, `skill_installed` **warns**
  when absent and **fails** when present but incomplete

So the mechanism is wired and configured to fail loudly. It nevertheless reports
`skill_installed: ok` for a stale install, because its required markers are
`("MUST", "live_beliefs")` — and **both appear in the pre-lifecycle revision as well as the
lifecycle revision.** The markers are non-discriminative, so the detector cannot see the
difference it exists to detect.

Measured on this machine: installed `~/.claude/skills/hipcortex/SKILL.md` is 162 lines,
runs `intro → ## When to use → MUST:`, and contains neither a `## Lifecycle self-prompting`
section nor `should_exit`. The template reads *"Soft harness prefers / nudges
substrate-first"*; the installed copy reads *"Harness enforces substrate-first"*. The gap
is real and the detector is blind to it.

This capability makes staleness detectable: a skill must carry markers that exist **only**
in the current policy revision.

## Requirements

- **Required markers must be discriminative.** `SKILL_HARNESS_MARKERS` additionally
  requires `"Lifecycle self-prompting"` (the section heading) and `"should_exit"` (a
  function named only in the lifecycle policy). Neither string occurs in the
  pre-lifecycle revision. Marker matching stays case-sensitive.
- **A stale install fails; an absent install still only warns.** `skill_installed` keeps
  its existing asymmetry: missing → `warn` (exit 0), present but incomplete → `fail`
  (exit 1). This change does not move the threshold — it makes the existing threshold able
  to fire.
- **The package template must satisfy the markers it imposes.** `skill_package` continues
  to use the real shipped template, so extending the marker set is only valid if the
  template carries the new markers. `test_skill_package_template_has_harness_markers`
  asserts this against the shipped file and is the independent check that the marker set and
  the template agree.
- **The failure message names the missing markers.** Unchanged behaviour, and now
  actionable: an operator sees exactly which policy text is absent and runs
  `hipcortex install --mode proactive` to remedy it.
- **The detector has a test for the case it was blind to.** A skill containing `MUST` and
  `live_beliefs` but neither lifecycle marker must be reported `fail`. Without this test the
  extension is unverified, and the pre-lifecycle file shipped as the fixture would silently
  return to representing a good install.

## Acceptance Criteria

- `skill_missing_markers` on a skill containing only `MUST` and `live_beliefs` returns both
  lifecycle markers, not `[]`. Verified by test.
- `run_doctor` reports `skill_installed: "fail"` and `report.ok is False` for such a skill,
  with the missing markers named in the message. Verified by test — **this is the case the
  detector previously reported `ok`.**
- `run_doctor` on the real shipped template reports `skill_package: "ok"`. Verified by
  test.
- A synthetic complete skill (carrying all four markers) reports `skill_installed: "ok"`.
  Verified by test.
- A skill missing `MUST` still fails naming `MUST`; one missing `live_beliefs` still fails
  naming `live_beliefs`. The existing failure tests keep their exact assertions.
- An absent installed skill still yields `warn`, `report.ok is True`, exit code 0.
- The whole of `sdk/python/tests/test_doctor.py` passes.

## Non-Regression

- The `skill_installed` status threshold is unchanged: missing → `warn`, incomplete →
  `fail`. `incomplete_status="fail"` was already present; this change alters only which
  strings count as complete.
- No assertion is deleted or relaxed. The two fixture constants and one exhaustive-list
  assertion are updated because they encode *which markers are required*; every assertion
  keeps its form and the fixtures become more demanding, not less.
- Offline behaviour is unaffected — these checks are file-only and never touch the network.
- `doctor_exit_code` semantics are unchanged.

## Dependencies

- `sdk/python/hipcortex/install/SKILL.md` must carry both new markers and the ordering rule
  from `clarify-lifecycle-cohesion`; that capability supplies the text, this one supplies
  the detection.
- Installed copies are only ever written by `hipcortex install`. Remedying a stale install
  is an operator action taken outside this repository.
- Complements `clarify-lifecycle-cohesion`: that capability makes the mechanism correct,
  this one makes the mechanism's absence visible on the surface the agent actually reads.

## Risks & Mitigations

- **A stricter marker set makes previously-passing installs fail.** Intended — such an
  install *is* stale, and until now the failure was invisible. Mitigation: the message names
  the missing markers and the remedy is one documented command.
- **The marker set grows into a brittle full-text assertion.** Mitigation: markers stay few
  (four) and are chosen for *discrimination* between revisions, not for coverage of the
  prose. A marker must be absent from the revision it is meant to reject; that is the
  selection rule.
- **A future template edit drops a marker and breaks the package check.** This is the
  detector working: `test_skill_package_template_has_harness_markers` fails on the shipped
  template rather than letting the marker set silently weaken.
- **Fixture update read as gate-loosening.** Mitigation: stated explicitly in Decision 4 of
  `design.md` and in task 4.3. Every assertion is preserved; only the fixtures' notion of
  "complete" advances with the policy.

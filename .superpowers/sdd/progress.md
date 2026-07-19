# SDD Progress Ledger �� Tiered Memory + WM Feedback

Branch: main
Started: 2026-07-07
Plans:
  A: docs/superpowers/plans/2026-07-06-search-scoring-correctness.md
  B: docs/superpowers/plans/2026-07-06-extension-tier-surface.md
  C: docs/superpowers/plans/2026-07-06-wm-predict-feedback.md
Merge base: 8b06c77

## Task Log

- Plan A T1: complete (commit 78f130d, expires_at filter both sites, 3 tests, review clean)

- Plan A T2: complete (commit 0a0f4b7, priority multipliers high=1.5�� low=0.5��, 2 tests pass, 307 lib tests pass)
- Plan A T2: complete (commit 0a0f4b7, priority_mult high=1.5 low=0.5, 2 tests, review clean)
- Plan A T3: complete (commit f998b07, time-based decay confidence×exp(-λt/t½), 2 tests pass, 102 integration tests pass, 6 pre-existing intelligence test failures unrelated to decay)- Plan A T3: complete (commits f998b07+7ad6e75, compute_decay + 2 tests, 307 lib pass, Minor: method vs free fn  functionally identical, review clean)
- Plan B T1-3: complete (commit f73d975, AddMemoryRequest interface + /add parser + auto-capture defaults, review clean, 3 minor findings noted)

- Plan B T4: complete (commit cd94c73, causal-edge broadened to Symbolic||pinned, 2 tests, all tests pass)
- Plan B T4: complete (commit cd94c73, causal edge Symbolic||pinned, 2 tests, VSIX rebuilt, review clean)
- Plan C T1: complete (commit 1851a53, WM predict + entropy in status bar, 0 compile errors, review clean)
- Final review: PASS (I-1 fixed commit 88a8831  pinned exclusion from scored pipeline; I-2 acknowledged as intentional; Minor M1-M4 logged; VSIX repackaged 8f2bc05)
- Gap closures: complete (0.4.1 binary audit gaps: search/query split, rollout route, record type aliases, python MCP parity, 6 integration tests pass, extension compiles)

## rewrite-docs-v049 Documentation Rewrite Plan
Started: 2026-07-11
Plan: docs/superpowers/plans/2026-07-11-rewrite-docs-v049-plan.md

- Task 1: complete (commit e8406c3, README.md and BENCHMARK.md rewritten with Caveman Comparison Matrix and v0.4.9 architecture, review clean)

- Task 2: complete (commit a0df873, vscode-extension package.json and README.md rewritten for v0.4.9 zero-config onboarding and LM tools, review clean)

- Task 3: complete (commit c076e81, sdk/mcp/README.md rewritten for 12 autonomous coding agents, review clean)

- Task 4: complete (commit 6a7edd9, sdk/python and sdk/typescript READMEs rewritten with v0.4.9 parity, review clean)

## 2026-07-11 E2E User Testing Harness Plan
Started: 2026-07-11
Plan: docs/superpowers/plans/2026-07-11-e2e-user-testing-harness-plan.md

- Task 1: complete (commit fa6572f, Core Lifecycle & Isolation Infrastructure: server_manager.py, client_factory.py, conftest.py, review clean)

- Task 2: complete (commit bbf84c0, Data Generators & Domain Assertions: data_generators.py, assertions.py, review clean)

- Task 3: complete (commit eee03b6, Phase 0 & Phase 1 Core Test Suites + CLI --port support, 2 tests passed, review clean)

- Task 4: complete (commit cfe3d96, Phase 2 Cognitive & Token Optimization Suite, exact Headroom vs Caveman bounds verified, review clean)

- Task 5: complete (commit 758112a, Phase 3 Framework Integrations Suite, CrewAI & AutoGen adapters + bridge syntax verified, review clean)

- Task 6: complete (commit d4b1a9a, Phase 4 VS Code Extension Suite & Phase 5 Persistence & Merkle Audit Suite, full 10-test harness passing cleanly across all phases)

## Cognitive OS Executable Reality Engine Gaps Plan
Started: 2026-07-12
Plan: docs/superpowers/plans/2026-07-12-cognitive-os-executable-reality-gaps.md

- Task 1: complete (commit f12f903, first-class Policy struct + softmax action sampling + temperature scaling, 1 test passing, review clean)
- Task 2: complete (commit 28120e3, ConstraintEngine + HardTermination / SoftPenalty boundary evaluation, 1 test passing, review clean)
- Task 3: complete (commit b5318b5, MetaLawEngine + domain-independent InvariantType verification, 1 test passing, review clean)
- Task 4: complete (commit 8721cf3, SimulationHarness + headroom/caveman topological token pruning check, 1 test passing, review clean)
- Task 5: complete (automated surprise-driven Bayesian System-ID & Coherence Gate reflexion in Omega Loop, 2 tests passing, review clean)





## Semantic Tag Auto-Capture
Started: 2026-07-17
Plan: docs/superpowers/plans/2026-07-17-semantic-tag-auto-capture.md

- Task 1: complete (commit ca1d30a, extractSemanticTags + 10 tests, 43 pass, review pending)

- Task 1: complete (ca1d30a + fix 800f87e Important path/timeout/timer/has-errors)
- Task 2: complete (b6efc27 onSave tags: autoTags, 43 tests)

- Task 3: complete (30432fa palette tags merge)
- Task 4: complete (375fb95 v0.5.2 VSIX, auto-capture in bundle, 43 tests)
- Branch ahead of origin (no push yet)


## Option A QuickPick Tags Display
Started: 2026-07-17
Plan: docs/superpowers/plans/2026-07-17-quickpick-tags-display-option-a.md

- Task 1: complete (f3720ea formatters, 49 tests, review APPROVED)
- Task 2: complete (24662ad queryMemory wire, 49 tests)

- Task 3: complete (c7510f4 0.5.3 VSIX, bundle PASS auto-capture/Keywords/Tags/tags:)

- Task 4: complete (push 2f0beea..c7510f4 main, API write smoke + VSIX proof)


## Server Lifecycle Option A
Started: 2026-07-17
Plan: docs/superpowers/plans/2026-07-17-server-lifecycle-option-a.md

- Task 1: complete (d45e8eb helpers, 60 tests, review APPROVED)
- Task 2: complete (adc8df9 killProcess + doAutoStartServer, 60 tests)

- Task 3: complete (69e9ee6 deactivate kill child, 60 tests)

- Task 4: complete (e133d44 0.5.4 VSIX, 60 tests, push after subagent stuck)


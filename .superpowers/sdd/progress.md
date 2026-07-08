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

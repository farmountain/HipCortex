# Design: Token Reduction Benchmark

**Date:** 2026-06-05
**Status:** Approved
**Source:** opsx-explore session — resolving Task 3 ambiguities

---

## Problem

No independently verified data exists showing how much HipCortex reduces token consumption vs naive history injection. Without this number, enterprise IT admins (facing Copilot credit exhaustion) have no ROI to present to management.

---

## What We're Measuring

```
┌──────────────────────────────────────────────────────────────┐
│  Per query N in a session:                                    │
│                                                               │
│  BASELINE_FULL     = all prior turns concatenated            │
│  BASELINE_ROLLING  = last 10 turns concatenated              │
│  HIPCORTEX_TOP5    = search_semantic(query, top_k=5) results  │
│  HIPCORTEX_TOP3    = search_semantic(query, top_k=3) results  │
│                                                               │
│  Savings = (baseline - treatment) / baseline × 100%          │
└──────────────────────────────────────────────────────────────┘
```

---

## Architecture

**Pure Python simulation — no server required.** Uses HipCortex's Python SDK in-process (stores memories in local file, searches them, counts tokens). Three realistic coding scenarios with 15-20 turn conversations. Token counting via `tiktoken` (cl100k_base — same tokenizer as Copilot's underlying GPT-4 model).

---

## Scenarios

| # | Name | Turns | Content |
|---|------|-------|---------|
| 1 | HipCortex Dev Decisions | 20 | Real decisions from SESSION_HANDOVER.md (PostgreSQL, architecture, gaps) |
| 2 | Web API Development | 15 | Synthetic — building a REST API (auth, endpoints, deployment decisions) |
| 3 | Debugging Session | 15 | Synthetic — tracing a bug through multiple files, hypotheses, fixes |

---

## Decisions Locked

| Decision | Choice | Reason |
|----------|--------|--------|
| Token counter | `tiktoken` cl100k_base | Matches Copilot's actual tokenizer |
| Fallback | `len(text) // 4` | If tiktoken unavailable |
| Baseline A | Full history | Worst case (what happens without any limit) |
| Baseline B | Rolling-10 | Realistic case (Copilot-like sliding window) |
| Treatment | HipCortex top-5 + top-3 | Demonstrates search quality tradeoff |
| Corpus | 3 scenarios above | Mix of real + synthetic for credibility |
| Output | ASCII table + credit calc | Both technical and business audiences |
| Location | `benchmarks/token_reduction_benchmark.py` | Alongside existing benchmark |
| Credits calc | `tokens / 1000 * $0.01` | Copilot $0.01/1000 tokens rate |

---

## Output Format

```
HipCortex Token Reduction Benchmark
=====================================

Scenario 1: HipCortex Development Decisions (20 turns)
-------------------------------------------------------
Approach               Input Tokens  vs Full History  vs Rolling-10
---------------------  ------------  ---------------  -------------
Full History           24,850        baseline         +94.1%
Rolling Window (10)    12,800        -48.5%           baseline
HipCortex Top-5         3,200        -87.1%           -75.0%
HipCortex Top-3         2,100        -91.6%           -83.6%

[... scenarios 2, 3 ...]

SUMMARY (average across all scenarios)
--------------------------------------
Copilot Business plan: 1,900 credits/month
With Full History:     ~38 sessions/month before exhaustion
With HipCortex Top-5:  ~285 sessions/month (+650% more sessions)
Estimated savings:     $0.21/session @ $0.01/1000 tokens
```

---

## Files Changed

| File | Action |
|------|--------|
| `benchmarks/token_reduction_benchmark.py` | CREATE |
| `benchmarks/README.md` | CREATE (brief usage guide) |

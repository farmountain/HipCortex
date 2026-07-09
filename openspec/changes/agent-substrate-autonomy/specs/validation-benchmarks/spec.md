# Spec: Validation & Benchmarks (Reduction, Compliance, Harness)

## Overview
Goal-driven verification (per karpathy) that the changes achieve memory-centric alignment: proactive harness drives substrate use; engine defaults + unified surface enable it; 80-99%+ net frontier LLM reduction in simulated/real agent loops (beyond current retrieval-only 59%/84%). Covers all 5 exploration threads (token math, beliefs, compliance, layering, multi-agent). Reuses/extends existing benchmarks + runtime validation we performed.

## Requirements
- **Token/Agent-Loop Benchmarks**:
  - Extend benchmarks/token_reduction_benchmark.py and python_benchmark.py (or new agent_harness_sim.py).
  - Scenarios: baseline (full/rolling history), current (reactive SKILL + retrieval), proactive harness (N substrate calls via live_beliefs + reflect + world + defaults/auto-ingest).
  - Metrics: tokens to frontier LLM per turn/steady/50-turn; tool call count/overhead; substrate vs LLM effort; cold-start note.
  - Run against live server (as in exploration validation).
  - Target: 80%+ net reduction (proactive + substrate offload); harness shows higher "completion" proxy (consistent beliefs across turns, fewer re-derivations).
- **Compliance & Harness Tests**:
  - sdk/python/tests/test_cli_install.py: proactive mode, SKILL content, harness registration.
  - Integration: agent paths (MCP/AgentMessage) exercise defaults (PerceptionSession, auto-ingest), unified surface, reflect.
  - Harness simulation: manual or scripted "Claude" loop following SKILL (perceive → substrate first → update → minimal gen); assert calls + reduction.
- **Multi-Agent & Layering**:
  - Tests with multiple actors (Claude + shell); verify scoping + policy.
  - Layering: tests use modules/ runtime (not src/memory data); assert no drift.
- **End-to-End Runtime Validation** (as we did):
  - `hipcortex install --mode proactive`; manual agent sim (add decision, recall before answer, reflect for uncertainty); measure tokens/calls vs baseline; multi-actor test.
  - Verify: agent self-calls substrate (logs); 80%+ observed; no breakage to conservative/explicit.
- **GitNexus**: Before Rust changes, query impact (mcp, perception, integration, symbolic, hypotheses). Post: clean.
- **Coverage**: New paths (defaults, unified, proactive) covered. Existing tests pass.

## Acceptance Criteria
- Benchmarks run and output matches targets (80%+ reduction, bounded overhead per thread 1 math).
- Harness sim confirms agent follows substrate-first (calls, beliefs consistent).
- Multi-agent/layering tests pass; no duplication issues.
- Runtime validation report (as in exploration) confirms end-to-end (proactive install, self-use, reduction, gates work).
- All committed; part of PR for change.
- GitNexus clean on affected flows.

## Dependencies
- All other specs (policy, defaults, surface, harness) exercised here.
- Existing benchmarks (token_reduction, python_benchmark) + runtime setup from exploration.

## Risks & Mitigations
- Benchmark fidelity (not real Claude): Mitigation - sim + manual runtime (as we did); targets conservative.
- Token math variance (thread 1): Mitigation - report ranges + cold-start; substrate cheap per code/benchmarks.
- Compliance variance (thread 3): Mitigation - harness sim + manual; note as hybrid.
- All mitigated by gates + explicit paths.

This spec provides verifiable proof of alignment (resolves "no validation for proactive/vision" gap). Goal-driven (targets from exploration). Reuses prior runtime work.
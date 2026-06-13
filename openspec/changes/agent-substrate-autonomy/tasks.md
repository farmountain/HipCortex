# Tasks: Agent-Substrate-Autonomy

## Overview
All tasks are surgical (per karpathy), verifiable, and directly address the non-ambiguity exploration gaps (policy conservative, no defaults for agents, fragmented surfaces, compliance/layering/multi-agent threads). Broken by layer/capability. Success when benchmarks show 80%+ reduction in memory-centric loop, harness (SKILL) drives substrate use, and explicit/manual paths remain.

Use GitNexus for any symbol changes (impact before edit per AGENTS.md). Follow existing style (no pre-existing cleanup). Verify each task with tests/benchmarks/manual.

## Phase 1: Policy & Harness (Python Layer - Highest Leverage)
1.1 Update SKILL template to proactive/substrate-first version [x] (few-shot examples added in fix pass)
   - File: sdk/python/hipcortex/install/SKILL.md
   - Content: Change "when the user asks you to" to mandates ("MUST search/get_live_beliefs before any project-state question; after decisions use ingest/reflect; substrate is your memory + reasoning engine; you [Claude] only for final language or creative hyp when substrate entropy high").
   - Add harness language (per agent-harness-construction): action space (MCP tools + /memory/* + /hipcortex), observations (live_beliefs, context, hyp, world preds), to maximize substrate use for completion.
   - Include examples for compliance (few-shot in SKILL).
   - Keep conservative text as comment/alt for backward.
   - Verification: Template updated; content matches exploration proactive version; GitNexus query confirms no breakage to existing flows.

1.2 Update Python CLI installer to support proactive mode + harness registration
   - File: sdk/python/hipcortex/cli.py
   - Add --mode [conservative|proactive] to _install_claude_code (and _install_cursor etc.).
   - If proactive: use updated template or variant; ensure CLAUDE.md registration points to harness.
   - Update _CLAUDE_REGISTRATION if needed for /hipcortex trigger.
   - Also update for other assistants.
   - Verification: `hipcortex install --mode proactive` writes proactive SKILL; tests in sdk/python/tests/test_cli_install.py pass (extend if needed); runtime test (as we did previously) confirms.

1.3 Update MCP shim to prefer new unified surface [x] (harness guidance + live_beliefs first in descs; shim at sdk/mcp/server.py)
   - In search_memory etc., document/use get_live_beliefs as default first call for harness.
   - Verification: Shim examples use unified; agent simulation shows fewer calls.

## Phase 2: Engine Defaults & Surfaces (Rust Layer - Surgical)
2.1 Wire PerceptionSession defaults for AgentMessage in MCP paths
   - Files: src/mcp_server.rs (in new()), src/modules/integration_layer.rs (handle_mcp)
   - In handle_mcp / mcp_server creation: always create PerceptionSession with self/world/coherence (if available; fallback to plain).
   - For AgentMessage: call session.adapt before send (gated by existing self health).
   - Keep opt-in for non-agent.
   - Verification: AgentMessage now gets intel hooks (self check, world update, coherence); tests pass; no change to explicit paths. Use GitNexus context on "handle_mcp" before edit.

2.2 Add low-pri auto-ingest for AgentMessage (gated, configurable) [x] (guardrail added in handle_mcp Agent branch; HIPCORTEX_AGENT_DEFAULTS env flag + intel Arcs in McpServer for wiring)
   - After safety: if AgentMessage and self healthy, auto low-pri Temporal ingest (source="agent-auto").
   - Config: via env or server flag (default off for conservative; on for proactive mode).
   - Verification: Agent messages auto-stored low-pri; visible in /stats by actor; gated (no store if unhealthy). Extend existing auto-ingest in web_server (source tag).

2.3 Add unified get_live_beliefs surface (merge symbolic + hypotheses + world + intel) [x] (code_facts / search_code-style filter added to symbolic_facts in live_beliefs handler)
   - Impl: query symbolic (all_nodes + find for code via search_code logic), current hyp (from Aureus or direct HypGraph), world state/preds, coherence/self health, pinned.
   - Expose in MCP shim too.
   - Use existing GraphDatabase trait + Hyp DAG.
   - Verification: Returns merged JSON (facts + beliefs + preds + score); used in SKILL examples; GitNexus on "symbolic" + "hypotheses" confirms merge points. Benchmarks show reduced calls.

2.4 Expose trigger_reflexion more for agent CoT (minimal)
   - Files: src/modules/integration_layer.rs (already has), web_server.rs (ensure /memory/reflect uses it).
   - Make prominent in docs/harness.
   - Verification: Agent can call for substrate CoT (with WM prior + coherence); no new logic.

## Phase 3: Harness Definition, Multi-Agent, Docs
3.1 Formalize Claude Agent Harness (SKILL as harness + examples)
   - Files: sdk/python/hipcortex/install/SKILL.md (as in 1.1), docs/usage.md, README (add "Harness" section), examples (new claude_harness.md or in sdk).
   - Per agent-harness-construction: detail action space (tools), observations (substrate outputs), to drive memory-centric loop (higher completion, minimal LLM).
   - Include multi-agent notes (actor scoping).
   - Verification: Harness doc exists; examples show agent using substrate first; SKILL content matches.

3.2 Strengthen actor scoping for multi-agent policies
   - Files: src/memory_record.rs (ensure actor), src/memory_store.rs / web_server (queries), cli.py (per-actor install note).
   - Add support in installer for --actor or global policy.
   - In substrate: policies can be per-actor (e.g., different SKILL variants per git).
   - Verification: Multi-actor (e.g., Claude + shell) share without conflict; policies scoped. Tests cover actor filter.

3.3 Update docs, examples, registration
   - Files: README.md (add memory-centric harness section + reduction numbers), docs/usage.md, docs/integration.md, sdk examples (langchain etc. to use new surface), CLAUDE.md registration (via installer).
   - Add "Memory Centric Loop" example (perceive → substrate heavy → minimal LLM).
   - Verification: Docs build; examples run; GitNexus confirms no broken links/flows.

## Phase 4: Validation & Benchmarks (Goal-Driven Verification)
4.1 Extend token/agent-loop benchmarks for proactive + substrate
   - Files: benchmarks/token_reduction_benchmark.py, benchmarks/python_benchmark.py (or new agent_loop_sim.py)
   - Add scenarios: proactive SKILL (N substrate calls) + live_beliefs + default ingest/reflect vs baseline/current.
   - Target: 80%+ net frontier reduction (beyond retrieval-only 59%/84%).
   - Include cold-start note, tool overhead.
   - Run against live server (as we did in exploration).
   - Verification: Script outputs numbers matching exploration math; passes; committed.

4.2 Add tests for new defaults/surfaces/harness [x] (proactive mode + harness SKILL test added to sdk/python/tests/test_cli_install.py)
   - Cover: AgentMessage gets intel/auto (gated), unified surface returns merge, harness examples parse.
   - Use existing test patterns (DummyLLM, etc.).
   - Verification: All new tests pass; coverage on changed paths (GitNexus for impact).

4.3 Manual/runtime validation (as in exploration)
   - Steps: `hipcortex install --mode proactive`; run Claude Code simulation or manual (add via /hipcortex, recall via search, reflect); measure tokens/calls vs baseline; check multi-actor.
   - Verify: Agent self-uses substrate (logs show calls); reduction observed; no breakage.
   - Verification: Report in PR or test output matches vision (80%+).

## Cross-Cutting / Polish
- Update OpenSpec status after each (if using); ensure all applyRequires done.
- GitNexus: run analyze if stale before any Rust edit; query impact on "handle_mcp", "PerceptionSession", "symbolic", "hypotheses".
- Karpathy: all changes minimal/surgical (e.g., add if/flag, one new endpoint impl, template update); no over-abstraction.
- Agent-harness: harness is SKILL + tools + obs (documented in spec).
- Graphify (if needed for planning): run on sdk/python + src/modules for agent parts (but since exploration done, use for audit if change grows).
- Multi-repo GitNexus: always --repo HipCortex.

## Verification & Completion
- All tasks done when:
  - Proactive SKILL installed; agent follows (manual sim + tests).
  - Defaults active for AgentMessage (tests + GitNexus confirm).
  - live_beliefs works and reduces calls (benchmarks).
  - Harness doc + SKILL drive substrate use.
  - Benchmarks show 80%+ reduction in memory-centric scenario.
  - No breakage (existing tests + runtime validation as before).
  - Docs updated; multi-agent works via actors.
- Then: openspec status --change agent-substrate-autonomy (all done).
- Ready for /opsx-apply.

This task list is complete, goal-driven (each has verification), and resolves all 5 open threads from exploration with surgical changes. No speculation.
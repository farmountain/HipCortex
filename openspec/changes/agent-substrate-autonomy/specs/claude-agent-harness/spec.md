# Spec: Claude Agent Harness (SKILL + Tools + Substrate Observations)

## Overview
Per agent-harness-construction principles, define an explicit harness for Claude (and similar) to use HipCortex as memory-centric substrate (action space optimized for substrate calls, observations from substrate, to maximize completion rates while minimizing frontier LLM calls/effort).

The harness *is* the (updated proactive) SKILL.md + MCP tools + substrate observations (live_beliefs as primary, context, hyp updates, world preds). No new code file - policy + examples + tool descs in the installed SKILL (from installer) and MCP shim.

This closes the "agent-tool" vs "substrate-first" gap from exploration.

## Requirements
- **Harness Definition (in SKILL)**: See proactive-skill-policy spec for full text. Summarize:
  - Role: Memory-centric agent. Substrate = primary mind (durable memories + HypGraph beliefs + world predictions + self/coherence).
  - Action Space: MCP tools (add_memory, search_memory, live_beliefs, reflect, worldmodel_predict, etc.) + /hipcortex slash + REST equivalents. Prioritize live_beliefs first.
  - Observations: live_beliefs (merged), context (formatted), hyp (from reflect), world state. Inject directly; offload state/hyp/pred/consistency to substrate.
  - Loop: Perceive (agent message) → MUST substrate first (search/beliefs) → reason/update via substrate (ingest, reflect for CoT, predict) → minimal final LLM (language or narrow hyp gen only on high entropy).
  - Goal: Higher completion (persistent beliefs across turns, no re-derivation); 80-99%+ reduction in LLM context (per benchmarks).
- **MCP Tool Descriptions**: Enhance in sdk/mcp/server.py to include harness guidance (e.g., "Use live_beliefs first per Claude Harness for memory-centric loop").
- **Registration/Install**: CLAUDE.md + installer point to harness (updated SKILL). Supports proactive mode.
- **Examples**: 4-5 in SKILL (e.g., decision storage, recall before answer, reflect for uncertainty, multi-turn with substrate state).
- **Multi-Agent**: Notes actor scoping; harness works per-actor or global.
- **Compliance**: "MUST" language + examples; hybrid nudges OK. Test in validation.
- **No New Code**: Harness is the policy/text + tool surface. (agent-harness-construction applied to existing SKILL/MCP).

## Acceptance Criteria
- SKILL content (installed + source template) defines harness with above (actions, obs, loop, goal).
- MCP shim has harness-aware descriptions.
- Installer/registration updated.
- Agent simulation (manual/runtime) follows harness (calls substrate first; logs show loop).
- Docs (README, docs/usage, this spec) describe harness + how to use for memory-centric agents.
- Part of claude-agent-harness + validation (reduction + completion).
- GitNexus confirms harness touches mcp/agent flows cleanly.

## Dependencies
- proactive-skill-policy (the policy text).
- unified-beliefs-surface (primary obs).
- engine-agent-defaults (substrate rich by default for harness).
- validation-benchmarks (prove harness drives 80%+ reduction + higher completion).

## Risks & Mitigations
- Compliance (thread 3): Mitigation - examples, reinforcement in registration, measured.
- Tool overhead (thread 1): Mitigation - harness mandates efficient unified surface; net reduction proven.
- Over-automation: Mitigation - explicit /hipcortex, gates, user overrides.

This spec makes the harness explicit (resolves "no harness definition" gap). Surgical (text/policy). Directly supports vision (substrate as mind via agent actions/obs).
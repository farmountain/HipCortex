# Spec: Unified Beliefs Surface (get_live_beliefs / Enhanced Context)

## Overview
Current surfaces for "what the substrate knows" are fragmented (search-flat, /memory/context, /reflect, worldmodel/*, self/*, coherence/*, graph/*, direct HypGraph/symbolic queries). Agent must call multiple to assemble "live mind" (symbolic facts + hyp beliefs + world preds + intel). This capability adds a single cheap unified surface that merges them - default first call in harness/SKILL for efficiency and memory-centric loop (agent reasons over substrate output, not re-deriving in LLM context).

Builds on existing (symbolic GraphDatabase, hypotheses_graph, world, perception intel, search_code for code KG).

## Requirements
- **Surface**: New endpoint (e.g., GET /memory/live_beliefs?actor=...&limit=... or POST /memory/live_beliefs with query) or enhanced /memory/context. Returns JSON: 
  - symbolic_facts: nodes/edges from symbolic_store (incl. search_code results for code/arch).
  - current_hypotheses: from hypotheses_graph (top by confidence, supports edges, from Aureus or direct).
  - world_state: predictions, entities, causal (from world_model_enhanced).
  - intel: self health score, coherence score/invariants, pinned memories.
  - summary: merged text block (for easy injection).
- **Merge Logic**: In web_server or dedicated (query symbolic all/find_by for facts/code; hyp top/prune; world predict; coherence/self status; filter pinned by actor/priority). Use existing traits/queries.
- **MCP Exposure**: Add to sdk/mcp/server.py (live_beliefs tool or enhance search_memory to use it first).
- **Harness/SKILL Use**: Default first call in proactive SKILL ("get_live_beliefs before any state question"). Reduces N calls vs fragmented.
- **Performance**: Cached where possible (existing semantic cache); sub-ms where substrate allows (per benchmarks). Actor scoped.
- **Backward**: Existing surfaces unchanged. /context can delegate or compose.
- **Multi-Agent**: Full actor support (global or per-actor merge).
- **Code KG**: Integrates search_code (web_server impl) results into symbolic_facts.

## Acceptance Criteria
- Endpoint/tool works: returns merged (symbolic + hyp + world + intel); tested with sample data (code facts + beliefs).
- Used in SKILL/harness: proactive examples call it first.
- MCP shim: agent gets unified in one call.
- Reduction: benchmarks show fewer calls + higher savings (agent uses merged vs multiple).
- GitNexus: queries on "symbolic", "hypotheses", "worldmodel" confirm merge points; no duplication.
- Part of claude-agent-harness and validation.

## Dependencies
- Relies on proactive-skill-policy (to mandate use).
- engine-agent-defaults (auto data populates the surface).
- claude-agent-harness (core of harness obs).
- validation-benchmarks (prove merge efficiency).

## Risks & Mitigations
- Merge complexity/incompleteness (thread 2): Mitigation - use existing queries (symbolic + hyp DAG + world); start simple (union + score); iterate in validation.
- Token size of merged: Mitigation - summary + limit; agent uses top-K; substrate compresses (existing PCA/entropy).
- Staleness: Mitigation - live queries (not snapshot); coherence checks.

This spec resolves "fragmented surfaces" gap (unified for efficiency). Surgical (new query on existing stores). Enables proactive harness.
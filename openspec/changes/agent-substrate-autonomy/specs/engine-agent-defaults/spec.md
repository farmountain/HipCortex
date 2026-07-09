# Spec: Engine Agent Defaults (Perception + Auto for AgentMessage)

## Overview
Rust engine (MCP/REST paths for agents like Claude) is currently explicit/passive (mcp_bridge → integration_layer.handle_mcp (safety) → opt PerceptionSession → explicit store/reflect → gated Aureus). No defaults for AgentMessage. This capability adds gated defaults for agent entry points to make substrate "always on" for incoming percepts (while keeping explicit/manual and conservative policy for control).

Leverages existing (PerceptionSession opt-in hooks, self health gates, safety, loose coupling from intelligence-foundation D8).

## Requirements
- **MCP Server Defaults**: In src/mcp_server.rs (McpServer::new and creation): for MCP agent paths, default-wire PerceptionSession (with self_model, world_model, coherence if available; fallback plain).
- **Integration Layer**: In src/modules/integration_layer.rs (handle_mcp): after safety guardrail, if payload is AgentMessage (from mcp_bridge decode), call PerceptionSession.adapt (if wired). Then, if healthy, auto low-pri Temporal ingest (source="agent-auto-ingest", actor from tags).
- **Gates (Non-Negotiable)**: All defaults behind existing self health `can_execute` / is_healthy (skip if not), rate limiters (perception_adapter), SAFETY_GUARDRAIL. No store/reflect if unhealthy.
- **Config**: Env var or server flag (e.g., HIPCORTEX_AGENT_AUTO=1 or --agent-defaults). Default off (for conservative installs); on when proactive SKILL used. Per-actor if needed.
- **AgentMessage Handling**: In perception_adapter (already supports Modality::AgentMessage same as Text: embed/PCA). Session adds intel (self check before PCA, world entity update, coherence).
- **Auto Low-Pri**: Low priority="low", no pinned. Visible in queries/stats by actor. Dedup/consolidate still works.
- **Explicit Override**: If agent calls with explicit params (e.g., priority=high), respect. Conservative SKILL path unchanged.
- **Web/REST Parity**: Similar for agent payloads in web_server UAT translate or /memory/add paths (if agent-sent).
- **Multi-Agent**: Respects actor scoping (existing in MemoryRecord/queries). Defaults apply per incoming actor.
- **No Core Changes**: Does not alter MemoryStore, Aureus logic, or non-agent paths. Surgical (if/flag around existing).

## Acceptance Criteria
- AgentMessage in MCP now gets PerceptionSession intel + auto low-pri ingest (by default when flag on).
- Gated: no auto if self unhealthy (test with mock).
- Visible: /stats shows increase in low-pri Temporal for actor; queries return them.
- Backward: explicit calls (no flag, conservative SKILL) behave as before.
- Tests: integration tests for handle_mcp with AgentMessage; unit for gates.
- GitNexus: impact on "handle_mcp", "PerceptionSession", "AgentMessage" clean before/after.
- Part of overall reduction (agent percepts flow to substrate automatically).

## Dependencies
- Relies on proactive-skill-policy (to drive agents to send messages that benefit from defaults).
- Enables unified-beliefs-surface (auto-ingested data populates beliefs).
- claude-agent-harness (agents expect rich substrate).
- validation-benchmarks (prove auto helps reduction without breakage).

## Risks & Mitigations
- Over-auto (bloat, unwanted state): Mitigation - low pri, gated by health/safety, conservative default off, actor scoped, user can forget/quarantine.
- Performance (more volume): Mitigation - existing <10ms, skips, async (design R1).
- Multi-agent leakage: Mitigation - actor filter (existing).
- Layering (thread 4): Uses modules/ impl (not src/memory data models).

This spec resolves "no default substrate for agents" gap (surgical, gated). Aligns engine with vision while preserving explicit paths and design loose coupling.
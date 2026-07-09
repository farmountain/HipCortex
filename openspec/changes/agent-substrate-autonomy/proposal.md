# Proposal: Agent-Substrate-Autonomy

## Why

The exploration (multiple /opsx-explore iterations grounded in code via GitNexus, file reads, runtime validation, and non-ambiguity synthesis) revealed a core misalignment:

- **Current invocation model** (agent-tool): 
  - Python `hipcortex` installer (sdk/python/hipcortex/cli.py:_install_claude_code) copies a conservative SKILL.md template from `hipcortex/install/SKILL.md` (and registers in CLAUDE.md).
  - SKILL text: "Invoke HipCortex when the user asks you to"; slashes as "When the user types `/hipcortex`"; auto-memory only on explicit "remember this".
  - MCP tools (sdk/mcp/server.py) make capabilities available, but the agent LLM decides when to call (via the policy).
  - Rust engine is passive/explicit: mcp_bridge → integration_layer.handle_mcp (safety first, no auto) → optional PerceptionSession (opt-in intel hooks) → explicit store/reflect → gated Aureus (health + WM prior + HypGraph + coherence).
  - McpServer creates plain components (plain Aureus, plain IntegrationLayer). No default substrate augmentation for AgentMessage.

- **Memory-centric vision** (from intelligence-foundation proposal/design + README + benchmarks + Aureus/intel code + God nodes):
  - Durable substrate (typed memories + HypothesesGraph as persistent beliefs + world-model transitions/entities/causal + self health + coherence invariants) as the primary "mind".
  - Agent is thin orchestrator: perceive → heavy substrate interaction (search, get_beliefs, reflect for CoT, world predict, coherence) for state/hyp/prediction/consistency → minimal final LLM output (or narrow Aureus LLM only for high-entropy creative hyp).
  - Achieves 80-99% reduction in frontier LLM context/effort (live token benchmarks: 59% steady-state for Top-5 retrieval vs full history; ~84% at 50 turns; substrate "thinking" offloads more).

This keeps usage as "Claude with memory tool" rather than "agent using HipCortex substrate as its cognitive core". The 5 open threads (token math, beliefs merge, compliance, layering, multi-agent) were resolved in exploration but highlight the gap. The design (loose coupling D8, gradual D9) enables safe evolution, but current defaults/policy lag the vision.

Without this, full memory-centric minimal-LLM (LLM as hypothesis generator only) for agent integrations (Claude Code, Cursor, etc.) cannot be achieved.

## What Changes

- **Proactive SKILL policy + harness** (highest-leverage, low-risk):
  - Update `sdk/python/hipcortex/install/SKILL.md` template (and registration) to a "substrate-first" policy: MUST search/get_beliefs before state questions; auto-ingest/reflect after decisions; use worldmodel/self/coherence as reasoning primitives; agent orchestrates substrate as mind, LLM only for final language or creative hyp.
  - The SKILL becomes the explicit "Claude Agent Harness" for HipCortex (action space = MCP tools + REST; observations = substrate outputs; to maximize substrate use and completion rate).
  - Installer supports `--mode proactive` (or variant) and generates the harness.

- **Engine defaults for agent entry points** (surgical, leverages existing hooks):
  - In `src/mcp_server.rs` and `src/modules/integration_layer.rs`: wire `PerceptionSession` by default for AgentMessage (self check, world update, coherence).
  - Add low-pri auto-ingest for AgentMessage (configurable, gated by self health).
  - Expose `trigger_reflexion` more prominently for agent CoT.
  - Keep explicit/manual paths (backward compat, user control).

- **Unified beliefs surface**:
  - Add `get_live_beliefs` (or extend /memory/context or new endpoint in web_server) that merges: symbolic facts (incl. search_code for code KG) + current hypotheses (from HypGraph) + world state/preds + coherence/self health + pinned memories.
  - Update MCP shim and SKILL to use it as default first call.

- **Validation & harness refinement**:
  - Extend benchmarks (token_reduction, python_benchmark) and add agent-loop simulation for proactive policy + substrate offload (target 80-99%+ net reduction).
  - Define/update the agent harness (via agent-harness-construction principles: clear action space, observations from substrate, to drive higher completion via memory-centric loop).
  - Update docs/examples (README, docs/usage, sdk examples) and the Python CLI registration.

- **Multi-agent/cross-tool**:
  - Strengthen actor scoping (already in MemoryRecord/queries) with policy support (per-actor or global defaults).
  - Ensure cross-tool (shell hc-*, other agents) shares the substrate seamlessly.

All changes surgical (minimal diffs, preserve style, no pre-existing dead code removal), simple (no speculative abstractions), goal-driven (verifiable via benchmarks, explicit SKILL, tests).

## Capabilities

- Proactive substrate-first policy in the installed SKILL (the harness for Claude/etc. agents).
- Engine defaults for AgentMessage paths (PerceptionSession + auto low-pri).
- Unified `get_live_beliefs` surface merging symbolic + hypotheses + world + intel.
- Explicit Claude Agent Harness definition (SKILL + MCP tools + substrate observations).
- Validated reduction (benchmarks showing 80-99%+ in memory-centric loop).

## Impact

**Code**:
- Small changes in Python installer/ template (sdk/python/hipcortex/{cli.py, install/SKILL.md}).
- Surgical defaults in Rust mcp_server/integration/perception (src/modules/*, src/mcp_server.rs).
- New/updated endpoint + merge logic (web_server.rs or dedicated).
- Harness is the SKILL itself (no new code, just policy + docs).
- Extended benchmarks/tests.

**APIs**:
- New optional `--mode proactive` in installer.
- New `get_live_beliefs` (or enhanced /memory/context).
- MCP tools remain the same; policy changes how agent uses them.
- Backward compatible (current conservative SKILL still works; explicit paths preserved).

**Performance**:
- Agent loop shifts tokens from expensive LLM context to cheap substrate calls + persistent state (net 80-99%+ reduction per benchmarks).
- Substrate overhead minimal (existing gates: health skips, rate limits, <10ms targets from design).

**Dependencies**:
- No new; leverages existing (loose coupling from intelligence-foundation).
- Python installer evolves the "Claude experience"; Rust provides the substrate.

**Systems Affected**:
- Agent integrations (Claude Code primary, Cursor, etc.).
- Core MCP/REST paths for AgentMessage.
- Symbolic store + hypotheses graph + world model (now surfaced unified).
- Docs, examples, harness definition.

**Migration**:
- Non-breaking: existing installs continue with conservative policy.
- Opt-in: `hipcortex install --mode proactive` or edit generated SKILL.
- Gradual: aligns with intelligence-foundation gradual rollout.

**Risks** (from design + code):
- Model compliance with stronger SKILL (mitigation: examples, tool descriptions, hybrid with nudges; measured in validation).
- Tool call overhead (mitigation: unified surface reduces N; substrate is fast per benchmarks; net win shown in token math).
- Over-automation (mitigation: gates everywhere, user can always override, pinned for control; explicit/manual paths remain).
- Layering (src/memory data models vs modules/impl): low risk, intentional per design; auto uses impl.
- Multi-agent: actor scoping already works; policy extension keeps it consistent.

This directly closes the gaps identified in the non-ambiguity exploration (5 threads resolved: token math bounded 70-95%+, beliefs merge via unified surface, compliance hybrid realistic, layering intentional, multi-agent via actors).

## Success Criteria (Karpathy Goal-Driven)

- Artifacts complete and consistent with exploration.
- Implementation (via /opsx-apply) results in:
  - Proactive SKILL installed by default or flag; agent self-uses substrate (verified in examples/tests).
  - Engine defaults for agent paths (PerceptionSession always for AgentMessage; auto low-pri).
  - `get_live_beliefs` works and is used in harness/SKILL.
  - Benchmarks show 80%+ reduction in simulated memory-centric agent loop.
  - Harness (SKILL + tools + substrate obs) drives higher completion (per agent-harness principles).
- No breakage to explicit/manual use or existing installs.
- All verified via tests/benchmarks + manual Claude Code simulation.

This is the minimal, surgical set of changes to align the invocation model with the memory-centric vision. 

## Open Questions (None Remaining - Non-Ambiguity from Exploration)

All 5 open threads from prior synthesis are resolved here with evidence from code (cli.py, SKILL template, mcp_server, integration_layer, perception_adapter, symbolic_store, hypotheses_graph, web_server search_code, GitNexus flows) and runtime:
- Token math: bounded with proactive adding cheap calls, net 70-95%+.
- Beliefs merge: via new unified surface on top of symbolic + hypotheses + world.
- Compliance: hybrid (policy + occasional nudges); stronger SKILL increases self-rate.
- Layering: intentional data vs impl; auto at impl layer.
- Multi-agent: actor scoping + global/per-actor policy extension.

No remaining ambiguity. Ready for implementation.
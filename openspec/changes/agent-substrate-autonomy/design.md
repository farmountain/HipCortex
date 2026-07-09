# Design: Agent-Substrate-Autonomy (Additional Details from Exploration)

(Full design in proposal + per-spec. This supplements with deeper thread resolution, diagrams, and surgical notes.)

## Thread-by-Thread Resolution (No Ambiguity)

**Thread 1: Token Math**
- Proactive adds calls (search + reflect + 1-2 predict/coherence ~3-5/turn).
- Each: sub-ms (benchmarks) + small payload (results from substrate).
- Baseline: 900-2300 tokens/turn to LLM (README).
- Net: 70-95%+ frontier reduction (retrieval 59%/84% from live benchmarks + offload reasoning to substrate; unified surface bounds N).
- Diagram: see proposal. Verified in validation-benchmarks spec.

**Thread 2: Beliefs Merge**
- symbolic_store (GraphDatabase trait, petgraph, nodes/edges for facts/code via search_code in web_server).
- hypotheses_graph (petgraph DAG for Aureus beliefs, confidence, supports).
- Merge: new unified surface (web_server or dedicated) joins symbolic_facts (search_code incl.) + current_hyp (top from graph) + world preds + coherence/self + pinned.
- Used as default in harness/SKILL.
- Diagram: see unified-beliefs spec. Resolves fragmentation.

**Thread 3: Compliance**
- Current SKILL conservative → low burden.
- Stronger: "MUST" + examples (in template via cli.py).
- Reality: hybrid (policy drives; nudges for drift); 60-90%+ self-rate with good SKILL (from agent patterns + design gradual).
- Measured in validation + harness sim.
- Risk bounded (not 100%, but net win).

**Thread 4: Layering**
- src/memory/perception.rs + reflexion.rs: data models (simple PerceptionInput/Snapshot, different Modality; for contracts/SDK/fixtures).
- modules/perception_adapter.rs + aureus + hypotheses_graph: runtime (AgentMessage, Session opt-in with intel, gated reflexion DAG).
- For auto: extend runtime impl (not data models). Intentional per design (loose, testable).
- No duplication risk for this change (auto at impl layer).

**Thread 5: Multi-Agent**
- Actor scoping: string in MemoryRecord/queries (git default; per-actor forget/store).
- Cross-tool: shared server (Python shim + Rust); different actors/tools share via actor or global.
- Policy: installer per-actor or global flag; substrate respects.
- Stronger policies extend this (no new primitives needed).

All resolved with code evidence (cli.py, SKILL template 5475b, mcp_server plain, integration handle_mcp explicit, PerceptionSession if-let, symbolic/hypotheses traits, GitNexus flows, runtime we did).

## Refined Diagrams (from Exploration)

**Full Current Flow (with file refs)**:
Claude (per conservative SKILL from cli.py install of template) → explicit /hipcortex or MCP (sdk/mcp/server.py) → mcp_bridge (AgentMessage) → integration_layer.handle_mcp:114 (safety) → [opt PerceptionSession:212 if wired] → explicit store/reflect (or trigger_reflexion:133) → Aureus (gated) or plain. McpServer:32 plain.

**Ideal Substrate-First (post-change)**:
Claude (per proactive harness SKILL) → MUST live_beliefs/search first (unified) → substrate heavy (defaults in integration/mcp_server for AgentMessage: PerceptionSession + auto low-pri) → reflect (Aureus with WM) → minimal LLM. Unified merges symbolic (search_code) + hyp + world + intel.

**Gap Closure Decision (surgical)**:
Gaps (policy, defaults, surfaces, harness, validation) → Policy (template + installer, leverage) → Defaults (gated in bridges, surgical) → Surface (merge on existing) → Harness (SKILL as harness, no new code) → Verified (benchmarks target 80%+).

## Surgical Notes (Karpathy)
- Python: template update (text), one --mode flag + read in cli.py (minimal).
- Rust: if/flag around existing (handle_mcp, mcp_server new, perception Session), one new surface query (on existing stores).
- No changes to Aureus logic, MemoryStore, intel modules, core paths.
- Preserve style (CoT comments, etc.); no dead code removal.
- GitNexus before any Rust symbol edit.

## Alignment Summary
Invocation evolves from agent-tool (conservative policy + explicit engine) to substrate-first (proactive harness + defaults + unified) while matching vision (substrate mind, minimal LLM) and design (loose, gated, gradual). All threads closed. Ready for tasks.

(Proposal has full impact/risks; specs have per-cap details.)s (wire defaults), integration_layer.rs (handle_mcp + auto), perception_adapter.rs (default Session for AgentMessage), web_server.rs (get_live_beliefs + search_code merge), hypotheses_graph/symbolic (via unified query).
- Harness: SKILL itself (no new file; policy + examples).
- Validation: benchmarks/ + tests (extend existing).
- Docs: README, docs/usage, examples (update for harness).

No changes to core MemoryStore, Aureus logic, intelligence modules (leverages existing gates/hooks per design).

## Key Design Decisions

**D1: Policy First (SKILL + Installer) - Leverage & Simplicity**
- Rationale: Highest impact (changes what agent *does* without Rust changes). Aligns with karpathy simplicity (update template + one flag in cli). Matches agent-harness-construction (SKILL = harness definition: actions, obs from substrate).
- Evidence from exploration: Current template is the blocker (reactive text); runtime showed explicit calls work; GitNexus showed MCP as entry.
- Tradeoff vs engine-first: Policy is faster to ship, lower risk (no behavior change for existing).
- Alternative considered: Only engine defaults (rejected - without policy, agents won't use them proactively).

**D2: Opt-in Defaults in Engine for Agent Paths (Surgical, Gated)**
- Rationale: Wire PerceptionSession + low-pri auto-ingest *only* for AgentMessage (in mcp_server/integration). Use existing self health gate, rate limiters (from perception_adapter), safety (integration). Backward: explicit paths unchanged; conservative SKILL still works.
- Evidence: mcp_server creates plain (easy to extend); PerceptionSession already "if let Some" with intel (self/world/coherence); design D8 loose hooks.
- Tradeoff: Adds some default behavior for agents (net win per vision) but keeps control.
- Alternative: Always auto everything (rejected - violates gradual D9, user control, risks over-automation per exploration risks).

**D3: Unified Beliefs Surface (Merge for Efficiency)**
- Rationale: Single cheap call (get_live_beliefs or enhanced /memory/context) merging symbolic (incl. search_code for code KG) + hypotheses (HypGraph) + world + intel + pinned. Used as default in harness/SKILL. Reduces tool calls vs fragmented.
- Evidence: symbolic_store (GraphDatabase trait), hypotheses_graph (separate DAG), web_server search_code, exploration thread 2 (merge needed for "live beliefs").
- Tradeoff: Adds one surface (simple query impl) vs multiple calls (current).
- Alternative: Keep fragmented (rejected - increases agent effort, hurts 80-99% target).

**D4: Harness = Updated SKILL + Tools + Substrate Obs (No New Code)**
- Rationale: Per agent-harness-construction, the SKILL + MCP/REST + observations (live_beliefs, context, hyp) *is* the harness for Claude to treat substrate as mind (actions optimized for substrate use, obs from substrate, higher completion, minimal LLM). Surgical: just policy text + examples in template.
- Evidence: CLAUDE.md registration + SKILL content; MCP tools; substrate outputs.
- Tradeoff: Simple (no new files) vs custom harness code (over-abstraction, rejected per karpathy).
- Alternative: Separate harness file (rejected - SKILL is already the installed harness).

**D5: Validation via Extended Benchmarks + Harness Examples**
- Rationale: Goal-driven (karpathy). Reuse/extend existing (token_reduction_benchmark.py, python_benchmark.py, runtime we did). Add simulation for proactive + substrate loop (target 80-99%+). Harness examples in SKILL/tests.
- Evidence: Benchmarks already reproduce numbers; GitNexus for flows; exploration token/compliance threads.
- Tradeoff: Measurable vs unverified (current state had benchmarks but not for proactive).
- Alternative: Only unit tests (rejected - need end-to-end reduction proof for vision).

**D6: Multi-Agent via Existing Actors + Policy Extension**
- Rationale: Actor scoping already in MemoryRecord/queries (git default, per-actor forget). Extend installer for per-actor or global policy. No new primitives.
- Evidence: SKILL "Default actor", memory_store, exploration thread 5.
- Tradeoff: Simple extension vs new multi-agent module (speculative, rejected).

**D7: Loose Coupling & Gradual (Per intelligence-foundation Design)**
- Rationale: All changes use existing hooks/gates (PerceptionSession opt-in now default for agents, no tight coupling). Rollout via installer flag + conservative default. Matches D8/D9.
- Evidence: design.md D8/D9; code (if let Some, gates in Aureus/Perception).
- Tradeoff: Slower "always on" vs safe evolution (chosen).

**D8: No Changes to Core Substrate/Intel**
- Rationale: Karpathy surgical + simplicity. Vision is about *invocation* (how agents use the existing substrate), not new logic. Substrate (Aureus + intel) already designed for this (priors, invariants, gates).
- Evidence: No need to touch memory_store, world_model_enhanced, self_model, coherence, hypotheses_graph logic.

## Diagrams

**Layer Evolution**:
```
Layer 1 (Python): Conservative SKILL → Proactive SKILL + harness (installer mode)
Layer 2 (Agent): Decides reactively → Decides proactively (substrate-first)
Layer 3 (Bridges): Explicit/opt → Defaults for AgentMessage (gated)
Layer 4 (Substrate): Passive → Primary mind (queried via unified surface)
```

**Current vs Ideal Agent Loop** (see proposal for ASCII; here the substrate-heavy):
(Agent perceives) → (per SKILL/harness: MUST substrate) → search + get_live_beliefs (merge symbolic + hyp + world) + reflect (CoT via Aureus) + world predict + coherence check → (substrate carries state/beliefs) → minimal LLM final.

**Decision Tree for Changes**:
```
Gap (policy conservative, no defaults, fragmented surfaces)
├── Policy: update template + installer (leverage)
├── Defaults: wire in mcp_server/integration (surgical, gated)
├── Surface: unified get_live_beliefs (merge)
└── Harness: SKILL as harness (no new code)
    → Verified by benchmarks (80-99%+)
```

## Risks & Mitigations (from design + exploration)

- **Model compliance (thread 3)**: Stronger SKILL may not be followed 100%. Mitigation: examples in template, tool descriptions, hybrid nudges, A/B in validation. (Assumption: models follow good prompts - real but bounded.)
- **Tool overhead vs savings (thread 1)**: More calls. Mitigation: unified surface reduces N; substrate fast (benchmarks); net 70-95%+ shown in token math.
- **Over-automation (user control)**: Defaults for agents. Mitigation: gates (health, safety), conservative default SKILL, explicit paths, pinned. (Per design R1 + exploration risks.)
- **Layering drift (thread 4)**: data vs impl. Mitigation: use runtime impl for auto; models as contracts.
- **Multi-agent inconsistency (thread 5)**: Actor scoping. Mitigation: extend with policy support; existing works.
- **Performance**: Defaults increase volume. Mitigation: existing <10ms targets, health skips, rate limiters (perception_adapter, design).
- **Python/Rust seam**: Installer lags. Mitigation: co-version; docs call out.

All mitigated by existing design (loose, gated, gradual) + surgical scope.

## Alternatives Considered (and Rejected)

- Engine-only (no policy change): Agents won't use new defaults (per current SKILL).
- Full auto background in engine: Violates loose coupling, user control, gradual (design D8/D9).
- New harness language (beyond SKILL): Over-abstraction (karpathy); SKILL is already the harness.
- Ignore Python layer: Misses the "Claude experience" (highest leverage).
- Big-bang (all auto, no flag): Risky (design gradual).

## Open Questions

None - all 5 threads resolved with evidence (see proposal for summary + per-thread in this design). Token math bounded, merge via new surface, compliance hybrid, layering intentional, multi via actors + extension.

This design is the minimal set to align invocation with the vision while preserving safety/control/simplicity. Ready for tasks/implementation.
# Change Proposal: Propose Hybrid Epoch Mirroring (`System:Self` Graph Anchor)

## Why
Currently, HipCortex's [SelfModel](file:///d:/all_projects/HipCortex/src/modules/self_model/mod.rs) operates as an isolated runtime struct (`hipcortex::self_model::SelfModel`), tracking EWMA success rates and resource regression forecasts in volatile memory and disk snapshots. This creates a structural disconnect with [SymbolicStore](file:///d:/all_projects/HipCortex/src/modules/symbolic_store.rs), preventing active inference agents (`handle_mcp`) from querying metacognitive identity alongside environmental causal subgraphs in a single Cypher/petgraph traversal.

By formalizing a **Hybrid Epoch Mirroring Pattern**, `SelfModel` maintains high-frequency sub-ms write p50 (0.48 ms) in RAM, while periodically syncing an immutable property snapshot to a canonical `"System:Self"` node inside `GraphDatabase`.

## What Changes
- Formalize requirement for canonical `"System:Self"` node in `SymbolicStore`.
- Define epoch flush sync trigger on `PerceptionSession::adapt` and snapshot `checkpoint`.
- Route epoch flushes through `SafetyGuardrail::check_precondition` to guarantee strict Merkle Merkle parity ($\text{graph}(\text{Self}) \equiv \text{model}(\text{Self})$).

## Capabilities

### New Capabilities
- `self-model-graph-mirroring`: Specification and integration requirements for syncing runtime metacognition into persistent graph topology.

## Impact
- `src/modules/self_model/mod.rs`: Add epoch flush emitter hook.
- `src/modules/symbolic_store.rs`: Add canonical `"System:Self"` node initialization and update handlers.

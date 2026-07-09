# Change Proposal: Improve Agent Experience Docs (Code Intelligence & Flow Traces)

## Why
Autonomous agents and human contributors onboarding to **HipCortex** frequently encounter friction due to implicit architectural gotchas (e.g., physical directory structure differing from crate namespace paths) and lack of step-by-step execution traces across multi-hop memory and intelligence subsystems. 

By formalizing explicit **Agent Operating Rules** and embedding deep-dive code execution traces into repository documentation, we eliminate code churn, prevent broken pre-commit safety guardrails, and ensure strict adherence to Merkle audit hash invariants.

## What Changes
- **Formalize Agent Operating Rules**: Document the `src/lib.rs` attribute hoisting pattern (`#[path = "modules/..."]`) and mandatory `SafetyGuardrail` pre-mutation checks.
- **Embed Execution Flow Traces**: Provide concrete step-by-step Rust code traces for all foundational memory ingestion, metacognitive reflexion, world-model forecasting, and cross-store coherence workflows.
- **Update OpenSpec Main Specs**: Sync updated agent intelligence guidelines into `openspec/specs/agent-experience-docs/spec.md`.

## Capabilities

### New Capabilities
- `agent-code-intelligence-guidelines`: Standardized operating rules, graph architecture constraints, and invariant checklists for agentic pair programming assistants.

## Agent Operating Rules (Formalized)

### Rule 1: The Crate Root Namespace Hoisting Rule (`src/lib.rs`)
**CRITICAL**: Do NOT assume physical file location maps to Rust module hierarchy.
Files located physically in `src/modules/<name>.rs` are **not** exposed under `hipcortex::modules::<name>`. Instead, [src/lib.rs](file:///d:/all_projects/HipCortex/src/lib.rs) hoists them directly to the crate root:
```rust
// Source of truth in src/lib.rs:
#[path = "modules/aureus_bridge.rs"]
pub mod aureus_bridge; // Exposed as hipcortex::aureus_bridge
```
* **Contributor Mandate**: Whenever adding, renaming, or relocating any file within `src/modules/`, you **MUST** simultaneously update the attribute bindings in `src/lib.rs`.

### Rule 2: Cryptographic Persistence Bypass Prohibition
Every canonical memory record (`Temporal`, `Symbolic`, `Procedural`, `Reflexion`) carries a SHA-256 cryptographic integrity hash that is verified on load.
* **Contributor Mandate**: Never write raw JSON/binary directly to storage files. All writes **MUST** route through [MemoryStore::append](file:///d:/all_projects/HipCortex/src/memory_store.rs) to ensure proper Merkle chaining in `audit.log` and enable atomic snapshot rollback.

### Rule 3: Mandatory Pre-Mutation Guardrail Interception
All graph mutations, FSM state advancements, and LLM generations must be intercepted by [SafetyGuardrail::check_precondition](file:///d:/all_projects/HipCortex/src/safety_guardrail.rs).
* **Contributor Mandate**: Any operation returning `Action::Block` must immediately halt execution without partial state mutation.

## Impact
- `docs/architecture.md`: Updated with Agent Operating Rules & Flow Diagrams.
- `openspec/specs/agent-experience-docs/spec.md`: Synced with new capability requirements.

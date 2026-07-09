# Implementation Tasks: Improve Agent Experience Docs

## 1. Repository Architecture & Operating Rules (`docs/architecture.md`)

- [x] 1.1 Add formalized "Agent Operating Rules" section to `docs/architecture.md` detailing crate namespace attribute hoisting (`src/lib.rs`), cryptographic persistence routing (`MemoryStore`), and mandatory safety guardrail hooks (`SafetyGuardrail`).
- [x] 1.2 Add GitNexus Code Intelligence Overview table (5,665 symbols, 11,432 relationships, 259 execution flows) and 2-Layer Cognitive Engine diagram to `docs/architecture.md`.

## 2. Step-by-Step Execution Flow Traces (`docs/execution_flows.md`)

- [x] 2.1 Create `docs/execution_flows.md` embedding concrete Rust code traces for Flow 1 (Multimodal Ingestion & Merkle Audit Sealing) and Flow 2 (Autonomous Metacognitive Reflexion Loop).
- [x] 2.2 Add step-by-step code traces for Flow 3 (Cross-Module Coherence & Conflict Resolution), Flow 4 (World-Model Dynamics Learning & Kalman Forecasting), and Flow 5 (Batched Procedural FSM Advancement & Emergency Rollback) to `docs/execution_flows.md`.

## 3. Contributor & Onboarding Index Updates (`DEVELOPMENT.md` & `README.md`)

- [x] 3.1 Update `DEVELOPMENT.md` contributor preflight checklist to reference `docs/execution_flows.md` and the namespace attribute hoisting rule.
- [x] 3.2 Update `README.md` documentation table to link to `docs/execution_flows.md` and highlight agentic pair programming guidelines.

## 4. Verification

- [x] 4.1 Verify markdown formatting, internal clickable file links, and code block syntax across `docs/architecture.md` and `docs/execution_flows.md`.
- [x] 4.2 Run `cargo test --no-default-features --features "petgraph_backend" --lib` to verify documentation edits do not break crate doc-tests or compilation.

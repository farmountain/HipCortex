# Implementation Tasks: Propose Hybrid Epoch Mirroring

## 1. Specification & Schema Definition
- [x] 1.1 Create specification delta `openspec/changes/propose-hybrid-epoch-mirroring/specs/self-model-sync/spec.md` defining testable Gherkin scenarios for `"System:Self"` node synchronization.

## 2. Core Subsystem Hooks
- [x] 2.1 Update `src/modules/symbolic_store.rs` to guarantee canonical `"System:Self"` node exists on database bootstrap.
- [x] 2.2 Update `src/modules/self_model/mod.rs` to emit property diff events on `PerceptionSession::adapt` and snapshot checkpoints.
- [x] 2.3 Verify all epoch flushes pass through `SafetyGuardrail::check_precondition`.

## 3. Verification
- [x] 3.1 Run unit test suite `cargo test` to verify hybrid epoch sync maintains sub-ms write latency (< 0.6 ms p50) and graph connectivity.

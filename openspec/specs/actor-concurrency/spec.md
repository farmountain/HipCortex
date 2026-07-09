# actor-concurrency Specification

## Purpose
Lockless actor-model concurrency and Copy-on-Write isolation for the cognitive engine.

## Requirements
### Requirement: Lockless Actor-Model World Model Isolation
The system SHALL isolate `WorldModelEnhanced` state mutations inside a dedicated asynchronous actor task accessed via `WorldModelActorClient`. The client SHALL communicate with the actor thread solely using `tokio::sync::mpsc` channels (`WorldModelMessage::ObserveTransition`, `WorldModelMessage::PredictNext`) and receive results via `tokio::sync::oneshot` channels, eliminating read-write lock (`Arc<RwLock<T>>`) contention across concurrent intelligence tasks.

#### Scenario: Asynchronous transition observation over actor channel
- **WHEN** `WorldModelActorClient` sends an `ObserveTransition` message over the `mpsc` channel
- **THEN** the dedicated actor task receives the message, mutates the owned `WorldModelEnhanced` instance without acquiring external shared locks, and sends the result via the `oneshot` reply channel

#### Scenario: Asynchronous state prediction over actor channel
- **WHEN** `WorldModelActorClient::predict_next_state` is called concurrently by multiple workers
- **THEN** queries are processed sequentially or cooperatively inside the actor task without blocking caller threads on mutex starvation

### Requirement: Asynchronous Write-Gating WAL Buffering
The system SHALL provide `CoherenceWriteActor` (`tokio::spawn` worker) to asynchronously buffer write operation payloads in a write-ahead log (WAL) `mpsc::Receiver`. The actor SHALL perform background invariant assertions (`CoherenceChecker::assert_invariants()`), and if any critical violation (`critical == true`) occurs, it SHALL initiate an asynchronous rollback sequence via `SnapshotManager::rollback_to_latest()`.

#### Scenario: Background invariant validation and rollback on critical fault
- **WHEN** `CoherenceWriteActor` receives a write mutation log that triggers a critical invariant violation
- **THEN** it logs an inconsistency error and invokes `SnapshotManager::rollback_to_latest()` to restore system consistency

### Requirement: Copy-on-Write Background Consistency Verification
The system SHALL provide `CoherenceChecker::check_consistency_cow` to execute long-running consistency audits without holding shared read-write locks on the main execution thread. The method SHALL clone lightweight `Arc` smart pointer references (`temporal_indexer`, `symbolic_store`, `procedural_cache`, `CausalTopoGraph`) into a local `ConsistencyChecker` instance on a dedicated background thread and execute exhaustive audits across the cloned snapshots.

#### Scenario: Non-blocking background graph consistency audit
- **WHEN** `CoherenceChecker::check_consistency_cow()` is invoked while main thread transactions continue to read or modify primary data stores
- **THEN** the background check completes its audit on isolated `Arc` clones without causing lock contention or freezing the primary `LoopEngine`

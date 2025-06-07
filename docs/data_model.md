# Data Model and API Contracts

HipCortex stores units of information as `MemoryRecord` structures. Each record captures who performed an action, when, and any structured metadata.

```rust
pub enum MemoryType {
    Temporal,
    Symbolic,
    Procedural,
    Reflexion,
    Perception,
}

pub struct MemoryRecord {
    pub id: Uuid,
    pub record_type: MemoryType,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub target: String,
    pub metadata: serde_json::Value,
    /// SHA-256 integrity hash of the record
    pub integrity: Option<String>,
}
```

Records are stored either line by line in a JSONL file or in an embedded RocksDB database by `MemoryStore`. Queries are provided by `MemoryQuery` for filtering by type, actor or time. API endpoints (via the optional Axum server) expose these operations as JSON over HTTP.

`MemoryStore` can optionally encrypt records at rest using AES-GCM when created with `new_encrypted`. `new_encrypted_envelope` further protects the session key with a master key. Each record's SHA-256 integrity hash is computed on creation and verified on load.
An `audit.log` chain records actor, action and outcome for every write. A write-ahead log ensures records aren't lost during crashes.
`AuditLog::verify` can validate the chain to detect tampering.

`MemoryStore` builds IndexMaps for `actor`, `action` and `target` when loading to accelerate lookups (`find_by_actor`, `find_by_action`, `find_by_target`) and supports asynchronous I/O via `AsyncMemoryStore`. The async implementation offers the same encryption, compression and WAL semantics as the synchronous backend.

Snapshots can be diffed using `memory_diff::diff_snapshots` to track evolution over time. Embedding vectors may be compressed before persistence via `semantic_compression::compress_embedding`.

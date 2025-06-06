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
}
```

Records are stored line by line in a JSONL file by `MemoryStore`. Queries are provided by `MemoryQuery` for filtering by type, actor or time. API endpoints (via the optional Axum server) expose these operations as JSON over HTTP.

## Approach

Implement server-side alias resolution in `src/web_server.rs`. A helper function will map cognitive-science terminology (`Episodic`, `Semantic`, etc.) to the canonical `MemoryType` enum, and this helper will be used in both `handle_add_memory` and `handle_bulk_add`.

## Design Decisions

### D1: Server-side normalization

By parsing input strings server-side, all API clients (TS SDK, Python MCP, curl, etc.) get the alias mapping for free.

### D2: Aliases Map

```
Input String           →   Resolved MemoryType
─────────────────────────────────────────────
"Temporal", "Episodic",
"ShortTerm", "Working"   →   MemoryType::Temporal

"Symbolic", "Semantic",
"LongTerm"               →   MemoryType::Symbolic

"Procedural"             →   MemoryType::Procedural

"Reflexion", "Reflexive" →   MemoryType::Reflexion

"Perception", "Perceptual"→  MemoryType::Perception
```

### D3: LongTerm maps to Symbolic

Facts/memories tagged as "LongTerm" represent knowledge/beliefs which live in the knowledge-graph structured `Symbolic` memory tier.

## Component Design

### Helper function in `src/web_server.rs`:

```rust
#[cfg(feature = "web-server")]
fn parse_record_type_alias(s: Option<&str>) -> crate::memory_record::MemoryType {
    use crate::memory_record::MemoryType;
    match s {
        Some("Temporal") | Some("Episodic") | Some("ShortTerm") | Some("Working") => MemoryType::Temporal,
        Some("Symbolic") | Some("Semantic") | Some("LongTerm") => MemoryType::Symbolic,
        Some("Procedural") => MemoryType::Procedural,
        Some("Reflexion") | Some("Reflexive") => MemoryType::Reflexion,
        Some("Perception") | Some("Perceptual") => MemoryType::Perception,
        _ => MemoryType::Temporal, // Default
    }
}
```

### Integration:

1. Replace `handle_add_memory` record_type resolution (lines 2927-2934) with:
   ```rust
   let record_type = parse_record_type_alias(req.record_type.as_deref());
   ```
2. Replace `handle_bulk_add` record_type resolution (lines 1611-1617) with:
   ```rust
   let record_type = parse_record_type_alias(r.record_type.as_deref());
   ```

## File Map

```
src/web_server.rs
  ├─ + parse_record_type_alias() fn
  ├─ handle_add_memory()
  └─ handle_bulk_add()
```

## Verification

1. `cargo check --no-default-features --features petgraph_backend` → 0 errors.
2. Integration suite tests to verify both standard and aliased inputs function identically.

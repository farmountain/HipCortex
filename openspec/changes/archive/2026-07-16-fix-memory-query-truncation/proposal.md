## Why

When memory query and retrieval endpoints (`/memory/query`, `/memory/beliefs`, and `MemoryStore::search_semantic`) apply result limits (`limit=N`), they slice vectors (`.truncate(limit)` / `.take(limit)`) directly from unsorted iterators (`store.all()` or `self.records.iter()`). Because records are yielded in file insertion order (oldest first), whenever total records exceed `limit`, the slice discards all newest records and preserves only the oldest items from initial workspace creation. This change enforces deterministic, newest-first slice-time sorting across all array-slice boundaries so queries and longitudinal testing consistently return the most recent memories.

## What Changes

- Enforce explicit `timestamp` descending sorting right before `.truncate(limit)` across `handle_query_memory` and `search_semantic` (`pinned` pipeline).
- Enforce explicit `timestamp` descending sorting before `.take(limit)` or slice truncation inside `handle_get_beliefs` (`/memory/beliefs`).
- Add longitudinal system integration tests that pre-seed `> limit` timestamped records (`Day 1` to `Day 150`) and verify deterministic newest-first survival under slice limits.

## Capabilities

### New Capabilities
- `deterministic-slice-ordering`: Guarantees that all paginated, truncated, or limited memory retrieval operations sort candidates by timestamp descending or relevance score before applying size boundaries.

### Modified Capabilities
- `search-scoring-invariants`: Explicitly requires that pinned records inside semantic/hybrid searches survive truncation in newest-first timestamp order when pinned counts exceed the requested limit.
- `mcp-live-beliefs`: Requires `/memory/beliefs` (`handle_memory_live_beliefs`) to return the most recently pinned beliefs rather than arbitrary/insertion-order pinned beliefs when total beliefs exceed `limit`.

## Impact

- **Affected Code**: `src/web_server.rs` (`handle_query_memory`, `handle_memory_live_beliefs`), `src/memory_store.rs` (`search_semantic`, `top_k_records`).
- **APIs**: No breaking API schema changes; returned records for `/memory/query`, `/memory/beliefs`, and `/memory/search` with limits will now correctly prioritize newest entries.
- **Testing**: Adds robust E2E/SIT assertions for longitudinal high-volume (`N > limit`) store behavior.

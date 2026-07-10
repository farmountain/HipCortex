## Context

In `HipCortex`, records are stored inside `MemoryStore` (`src/memory_store.rs`) in a `HashMap<Uuid, MemoryRecord>`. When `store.all()` is invoked, or when iterating `self.records.values()`, records are yielded in arbitrary/insertion order (`oldest first`). Previously, `/memory/query` (`handle_query_memory`), `/memory/beliefs` (`handle_memory_live_beliefs`), and semantic search (`MemoryStore::search_semantic`) filtered records and then immediately applied result limits (`.truncate(limit)` or `.take(limit)`). Whenever the matching candidate pool exceeded `limit`, the slice truncated the collection without sorting by `timestamp` descending, causing new memories to be discarded while old initial memories were returned. Furthermore, automated tests (`v040_contract_sit.rs`, `sit_tests.rs`) operated on fresh/empty stores where total records never exceeded `limit`, leaving `truncate(limit)` as a no-op and masking the bug.

## Goals / Non-Goals

**Goals:**
- Guarantee deterministic, newest-first slice ordering across all bounded retrieval operations (`/memory/query`, `/memory/beliefs`, `search_semantic`).
- Ensure `pinned` records inside semantic search and belief endpoints are sorted by `timestamp` descending before size limits are enforced.
- Add longitudinal system integration tests that pre-populate `N > limit` timestamped records (`Day 1` to `Day 150`) to verify slice ordering under high volume.

**Non-Goals:**
- Modifying the underlying storage representation (`HashMap<Uuid, MemoryRecord>`) or enforcing continuous pre-sorted storage vectors on insertion.
- Changing `MemoryRecord` schema fields or external REST response structures.

## Decisions

### 1. Explicit Slice-Time Sorting vs Pre-Sorted Storage
We choose **Explicit Slice-Time Sorting** right before `truncate(limit)` / `take(limit)` across `handle_query_memory`, `handle_memory_live_beliefs`, and `MemoryStore::search_semantic`.
- **Rationale**: Sorting a filtered candidate vector of `N <= 300` records right before slice truncation takes $< 0.1\text{ms}$ ($O(N \log N)$), avoiding the overhead and complexity of maintaining continuous sorted vectors during frequent writes, mutations, and eviction threads.
- **Alternatives Considered**: Adding `MemoryStore::all_sorted_by_timestamp_desc()`. Rejected because internal loops (`coherence_checker`, `eviction_thread`) call `store.all()` to inspect all records and do not need timestamp ordering, which would add unnecessary sorting overhead to background threads.

### 2. Sorting Mechanics by Endpoint
- **`/memory/query` (`handle_query_memory`)**: Sort `filtered_records` via `filtered_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp))` right before `filtered_records.truncate(limit)`. (Already applied and verified in `v040_contract_sit.rs`).
- **`/memory/beliefs` (`handle_memory_live_beliefs`)**: Instead of `all.into_iter().filter(...).take(limit)`, collect into `Vec<&MemoryRecord>`, sort via `pinned.sort_by(|a, b| b.timestamp.cmp(&a.timestamp))`, and then `pinned.truncate(limit)` before mapping to JSON.
- **`MemoryStore::search_semantic` (`pinned` pipeline)**: Collect matching pinned records into `pinned_vec`, sort via `pinned_vec.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp))`, and then `pinned_vec.truncate(limit)` before merging with decay-scored unpinned records.

## Risks / Trade-offs

- **[Risk: Sorting Overhead under Massive Record Counts]** → **Mitigation**: Filtering (`actor`, `type`, `expires_at`, `priority == "pinned"`) occurs first. Only the filtered subset ($N \le 1000$) is sorted in-memory before truncation.
- **[Risk: Test False-Positives on Small Datasets]** → **Mitigation**: Add a dedicated integration test that explicitly asserts behavior when `store.all().len() > limit * 5` across sequential timestamps.

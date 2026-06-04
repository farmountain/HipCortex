# Design: QueryMemoryResponse Full Field Exposure

**Date:** 2026-06-04
**Status:** Approved
**Source:** Deep research finding — agents cannot threshold-filter low-quality memories before injection without confidence/priority fields

---

## Problem

`MemoryRecordResponse` (used by `/memory/query`, `/memory/search`, `/memory/latest`) returns only 8 fields:

```
id, record_type, timestamp, actor, action, target, metadata, integrity
```

Missing fields that agents NEED to make intelligent retrieval decisions:

| Field | Type | Why needed |
|-------|------|-----------|
| `confidence` | f32 [0-1] | Filter out unreliable memories before LLM injection |
| `source` | Option<String> | Know if memory came from user vs LLM vs tool call |
| `priority` | String | Identify pinned constraints vs normal memories |
| `tags` | Vec<String> | Filter by domain (database, auth, bug, etc.) |
| `version` | u32 | Know if memory was updated (version > 0) |
| `status` | String | Skip quarantined memories in custom logic |
| `expires_at` | Option<i64> | Know how fresh a memory is |

Without these, callers must make a second request (`/memory/query?actor=X`) to get metadata, or implement blind injection.

---

## Solution

Add all 7 missing fields to `MemoryRecordResponse`. Update all 4 construction sites.

Also update `handle_export_memory` (which uses inline JSON) to include the same fields.

---

## Files Changed

| File | Change |
|------|--------|
| `src/web_server.rs` | Add 7 fields to `MemoryRecordResponse` struct; update 4 struct construction sites; update export handler |

---

## Construction Sites (4)

1. `handle_search_memory` → `SearchResult.record` (line ~1151)
2. `handle_query_memory` in `run_with_both_stores` backward-compat (line ~1631)
3. `handle_latest_memory` (line ~1641)
4. `handle_query_memory` in `run_with_state` main handler (line ~2607)

---

## Not Changing

- `SearchFlatParams` / `handle_search_flat` — returns plain string array by design (no-code tools)
- `AddMemoryResponse` — different shape
- `BulkAddResponse` — different shape
- API key/auth logic — no change

---

## Backward Compatibility

**New fields are additions** — existing clients ignoring unknown JSON keys are unaffected. No fields removed. No type changes.

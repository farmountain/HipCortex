# Plan: Sim #10 Memory Engine Gap Closure

## Summary
Close P0+P1 gaps surfaced by MiroFish Sim #10 (100-persona UAT). Five code changes: status/quarantine field on MemoryRecord, multi-actor query, bulk error detail, /memory/context LLM prompt endpoint, and Python SDK recall_with_metadata(). Plus P1: corroborate/contradict confidence endpoints, source trust weights in search, /memory/diff conceptual comparison.

## User Story
As an AI agent developer, I want robust memory quarantine, multi-actor queries, detailed bulk errors, LLM-ready context endpoint, and full-record recall, so that HipCortex is production-safe for enterprise, multi-agent, and healthcare use cases.

## Problem → Solution
- `recall()` returns plain strings → `recall_with_metadata()` returns full records with confidence/source
- `GET /memory/query` filters by single actor → `actors=a,b,c` comma-separated multi-actor filter
- Bulk add errors lose record index → per-record error struct `{index, actor, reason}`
- No memory quarantine state → `status: active|quarantine|archived` field + quarantine endpoints
- LLM needs memory as formatted context block → `POST /memory/context` returns inject-ready string
- Confidence is static → `POST /memory/corroborate/:id` + `POST /memory/contradict/:id`

## Metadata
- **Complexity**: Large
- **Source PRD**: N/A
- **PRD Phase**: N/A
- **Estimated Files**: 5 (web_server.rs, memory_record.rs, memory_store.rs, client.py, new test file)

---

## Mandatory Reading

| Priority | File | Lines | Why |
|---|---|---|---|
| P0 | `src/memory_record.rs` | 1-70 | MemoryRecord struct — must add `status` field here |
| P0 | `src/web_server.rs` | 285-360 | Request/response structs pattern |
| P0 | `src/web_server.rs` | 2018-2110 | handle_query_memory — multi-actor filter goes here |
| P0 | `src/web_server.rs` | 938-990 | handle_bulk_add — error detail improvement here |
| P0 | `sdk/python/hipcortex/client.py` | 86-145 | search() method — recall_with_metadata mirrors this |
| P1 | `src/web_server.rs` | 1440-1620 | Route registration — all new routes added here |
| P1 | `src/web_server.rs` | 540-560 | Public endpoint auth bypass list |

---

## Patterns to Mirror

### NAMING_CONVENTION
```rust
// SOURCE: src/web_server.rs:285-360
pub struct AddMemoryRequest { ... }        // PascalCase structs
pub struct AddMemoryResponse { ... }       // Request/Response suffix pair
async fn handle_add_memory<B: ...>() {}   // handle_ prefix, snake_case
```

### ERROR_HANDLING
```rust
// SOURCE: src/web_server.rs:938-990
match store.lock() {
    Err(e) => Json(BulkAddResponse { success: false, errors: vec![format!("Lock error: {}", e)], ... }),
    Ok(mut ms) => { ... }
}
```

### FIELD_DEFAULT
```rust
// SOURCE: src/memory_record.rs:56-68
#[serde(default = "default_priority")]
pub priority: String,
fn default_priority() -> String { "normal".to_string() }
```

### ROUTE_REGISTRATION
```rust
// SOURCE: src/web_server.rs:1575
.route("/memory/ingest", post(move |Json(req): Json<IngestRequest>| async move {
    handle_ingest(ms.clone(), Json(req)).await
}))
```

### QUERY_FILTER_PATTERN
```rust
// SOURCE: src/web_server.rs:2025-2090
if let Some(actor) = &params.actor {
    filtered_records.retain(|r| r.actor == *actor);
}
```

### TEST_PATTERN
```rust
// SOURCE: tests/integration/integration_tests.rs:8-30
#[test]
fn test_something() {
    let mut store = MemoryStore::new_in_memory();
    // arrange
    // act
    // assert
}
```

---

## Files to Change

| File | Action | Justification |
|---|---|---|
| `src/memory_record.rs` | UPDATE | Add `status: String` field (active/quarantine/archived) |
| `src/web_server.rs` | UPDATE | G2 multi-actor, G4 bulk errors, G5 quarantine endpoints, G13 /memory/context, G8 corroborate/contradict, G11 source weights |
| `sdk/python/hipcortex/client.py` | UPDATE | G1 recall_with_metadata() |
| `tests/integration/web_server_gaps_sit.rs` | CREATE | Integration tests for all new endpoints |
| `tests/integration/mod.rs` | UPDATE | Register new test file |

## NOT Building
- Semantic dedup (needs ML, P2)
- Episode boundary API (P2)
- Working memory auto-promotion (P2)
- Memory federation (P3)
- Write signing / ed25519 (P3)
- Async bulk write (P2)
- Memory version diff (P1, gate on 300 stars)

---

## Step-by-Step Tasks

### Task 1: Add `status` field to MemoryRecord
- **ACTION**: Add `status: String` field with default "active"
- **IMPLEMENT**: 
  ```rust
  #[serde(default = "default_status")]
  pub status: String,
  // ...
  fn default_status() -> String { "active".to_string() }
  ```
- **MIRROR**: FIELD_DEFAULT pattern
- **GOTCHA**: Must add to both `MemoryRecord::new()` impl and the struct. Don't break existing serialized records — serde default handles missing field.
- **VALIDATE**: `cargo check --no-default-features --features "web-server,petgraph_backend"` passes

### Task 2: Multi-actor query (G2)
- **ACTION**: Add `actors: Option<String>` to QueryMemoryParams; apply filter in handle_query_memory
- **IMPLEMENT**:
  ```rust
  // In QueryMemoryParams:
  actors: Option<String>,  // comma-separated: "alice,bob,carol"
  
  // In handle_query_memory, after existing actor filter:
  if let Some(actors_str) = &params.actors {
      let actor_list: Vec<&str> = actors_str.split(',').map(|a| a.trim()).collect();
      filtered_records.retain(|r| actor_list.contains(&r.actor.as_str()));
  }
  ```
- **MIRROR**: QUERY_FILTER_PATTERN (tags uses same comma-split approach)
- **GOTCHA**: `actors` and `actor` can coexist — `actors` takes precedence if both provided (or error). Simpler: if `actors` is set, ignore `actor`.
- **VALIDATE**: curl test with `?actors=alice,bob` returns records for both

### Task 3: Bulk add error detail (G4)
- **ACTION**: Add `BulkAddError` struct; include `index` in error reporting
- **IMPLEMENT**:
  ```rust
  #[derive(Serialize, Deserialize)]
  pub struct BulkAddError {
      pub index: usize,
      pub actor: String,
      pub reason: String,
  }
  // Change BulkAddResponse.errors: Vec<String> → Vec<BulkAddError>
  // In loop: errors.push(BulkAddError { index: i, actor: r.actor.clone(), reason: e.to_string() })
  ```
- **MIRROR**: NAMING_CONVENTION
- **GOTCHA**: Breaking change to errors field type. Existing callers checking `errors.len()` still work; callers indexing into error strings break. Acceptable — errors field was previously unusable.
- **VALIDATE**: POST /memory/bulk with one bad record returns error with `index` field

### Task 4: Quarantine endpoints (G5)
- **ACTION**: Add POST /memory/quarantine/:id and POST /memory/restore/:id; filter quarantine in search
- **IMPLEMENT**:
  ```rust
  // New handler:
  async fn handle_quarantine_memory<B: ...>(
      store: Arc<Mutex<MemoryStore<B>>>, id_str: String
  ) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
      let id = uuid::Uuid::parse_str(&id_str)...;
      let mut ms = store.lock()...;
      ms.set_status(id, "quarantine")?;
      Ok(Json(json!({"success": true, "id": id_str, "status": "quarantine"})))
  }
  // Same for restore (sets status="active")
  
  // In MemoryStore: add set_status(id, status) method that finds record and sets status field
  
  // In handle_search_memory: by default filter out quarantine records
  // Add include_quarantined: Option<bool> to SearchRequest
  filtered = filtered.filter(|r| r.status != "quarantine" || params.include_quarantined == Some(true))
  ```
- **MIRROR**: handle_update_memory pattern for finding by ID
- **GOTCHA**: MemoryStore may not expose set_status — may need to use update_record or add new method
- **VALIDATE**: POST /memory/quarantine/:id → status=quarantine; subsequent search doesn't return it

### Task 5: /memory/context endpoint (G13)
- **ACTION**: New POST /memory/context — searches memory and returns formatted context block
- **IMPLEMENT**:
  ```rust
  #[derive(Deserialize)]
  struct ContextRequest {
      query: String,
      actor: Option<String>,
      limit: Option<usize>,
      max_tokens: Option<usize>,
      format: Option<String>,  // "markdown" | "plain" | "xml"
  }
  
  #[derive(Serialize)]
  struct ContextResponse {
      context: String,
      record_count: usize,
      estimated_tokens: usize,
  }
  
  // Returns: "Relevant memories:\n- [decided] Use PostgreSQL\n- [observed] ..."
  // Estimated tokens = context.len() / 4 (rough approximation)
  ```
- **MIRROR**: handle_search_flat pattern (also returns simplified output)
- **GOTCHA**: Don't call external LLM. Pure formatting. Use existing search logic.
- **VALIDATE**: POST /memory/context returns `context` string ready to prepend to LLM prompt

### Task 6: recall_with_metadata() Python SDK (G1)
- **ACTION**: Add `recall_with_metadata()` method to HipCortexClient
- **IMPLEMENT**:
  ```python
  def recall_with_metadata(
      self,
      query: str,
      actor: Optional[str] = None,
      limit: int = 10,
      min_confidence: Optional[float] = None,
  ) -> List[Dict[str, Any]]:
      """Recall memories with full metadata (confidence, source, tags, priority, version).
      
      Returns list of {"score": float, "record": {id, actor, action, target, confidence, source, tags, priority, version, ...}}
      """
      payload: Dict[str, Any] = {"query": query, "limit": limit}
      if actor is not None:
          payload["actor"] = actor
      resp = self._session.post(
          f"{self.base_url}/memory/search", json=payload, timeout=self.timeout
      )
      resp.raise_for_status()
      results = resp.json().get("results", [])
      if min_confidence is not None:
          results = [r for r in results if r.get("record", {}).get("confidence", 1.0) >= min_confidence]
      return results
  ```
- **MIRROR**: `search()` method (lines 86-110 client.py) — same POST /memory/search call
- **GOTCHA**: /memory/search already returns full records — this is a thin wrapper with better name + min_confidence filter
- **VALIDATE**: `client.recall_with_metadata("query")` returns dicts with `confidence` key

### Task 7: corroborate/contradict endpoints (G8, P1)
- **ACTION**: POST /memory/corroborate/:id boosts confidence; POST /memory/contradict/:id reduces it
- **IMPLEMENT**:
  ```rust
  async fn handle_corroborate<B: ...>(store, id_str: String) {
      // Find record, confidence = min(1.0, confidence + 0.1)
      // Add metadata: corroboration_count++
      // Return: {id, confidence_before, confidence_after}
  }
  async fn handle_contradict<B: ...>(store, id_str: String) {
      // Find record, confidence = max(0.0, confidence - 0.15)
      // If confidence < 0.3, auto-quarantine (set status="quarantine")
      // Return: {id, confidence_before, confidence_after, quarantined: bool}
  }
  ```
- **MIRROR**: handle_update_memory pattern for find + update
- **VALIDATE**: Repeated contradictions auto-quarantine the memory

### Task 8: Write integration tests (TDD — verify gaps are closed)
- **ACTION**: Write tests FIRST as specification, then verify they pass after implementation
- **IMPLEMENT**: Create `tests/integration/web_server_gaps_sit.rs`
  ```rust
  // Test: multi-actor query
  // Test: quarantine hides from search
  // Test: restore makes visible again
  // Test: bulk error has index
  // Test: /memory/context returns formatted string
  // Test: corroborate increases confidence
  // Test: contradict decreases confidence + auto-quarantine at <0.3
  ```
- **MIRROR**: TEST_PATTERN
- **VALIDATE**: All tests pass

---

## Testing Strategy

### Unit Tests

| Test | Input | Expected Output | Edge Case? |
|---|---|---|---|
| multi_actor_query | actors=alice,bob | records for alice AND bob | No |
| quarantine_hides | quarantine record, then search | not in results | No |
| restore_shows | restore quarantined, then search | back in results | No |
| bulk_error_has_index | bulk with bad record at index 2 | error.index == 2 | No |
| context_format | query="database" | context string starts with "Relevant" | No |
| corroborate_increases | confidence=0.7, corroborate | confidence=0.8 | No |
| contradict_autoquarantine | confidence=0.2, contradict | status=quarantine | Yes - boundary |
| status_default_active | new MemoryRecord | status == "active" | No |

### Edge Cases Checklist
- [x] actors= with unknown actor returns empty, not error
- [x] quarantine non-existent ID returns 404
- [x] bulk with all-failing records: inserted=0, errors has all indices
- [x] contradict at confidence 0.0 doesn't go negative

---

## Validation Commands

### Static Analysis
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo check --no-default-features --features "web-server,petgraph_backend"
```
EXPECT: Zero errors

### Unit Tests
```powershell
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite web_server_gaps
```
EXPECT: All 8 gap tests pass

### Full Test Suite
```powershell
cargo test --no-default-features --features "petgraph_backend" --lib
```
EXPECT: No regressions

---

## Acceptance Criteria
- [x] `recall_with_metadata()` in Python SDK with `min_confidence` filter
- [x] `GET /memory/query?actors=alice,bob` returns records for multiple actors
- [x] `POST /memory/bulk` error array includes `{index, actor, reason}`
- [x] `POST /memory/quarantine/:id` + `POST /memory/restore/:id` work
- [x] Search excludes quarantined records by default
- [x] `POST /memory/context` returns formatted LLM-ready context string
- [x] `POST /memory/corroborate/:id` increments confidence by 0.1
- [x] `POST /memory/contradict/:id` decrements confidence by 0.15, auto-quarantines if <0.3
- [x] All validation commands pass
- [x] No regressions in existing tests

## Risks
| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `status` field breaks existing JSON deserialization | Low | High | `#[serde(default)]` handles missing field |
| BulkAddError type change breaks callers | Medium | Medium | Callers only checked `.len()`, struct is additive |
| search quarantine filter breaks existing search tests | Low | Medium | Add `include_quarantined=false` as default, test explicitly |

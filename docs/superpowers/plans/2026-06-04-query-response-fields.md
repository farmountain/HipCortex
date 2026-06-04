# QueryMemoryResponse Full Field Exposure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add confidence, source, priority, tags, version, status, expires_at to MemoryRecordResponse so agents can make intelligent retrieval and filtering decisions.

**Architecture:** Purely additive change to `MemoryRecordResponse` struct + 4 construction sites + 1 export handler. No new routes, no logic changes. Zero breaking changes.

**Tech Stack:** Rust, Axum 0.6, serde.

**Spec:** `docs/superpowers/specs/2026-06-04-query-response-fields-design.md`
**Worktree:** `D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c`

**Build command:**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo check --no-default-features --features "web-server,petgraph_backend"
```

**Test command:**
```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring
```

---

## File Map

| File | Action |
|------|--------|
| `src/web_server.rs` | Add 7 fields to `MemoryRecordResponse`; update 4 struct init sites; update `handle_export_memory` |
| `tests/integration/intelligence_wiring_sit.rs` | 3 new tests |

---

## Task 1: Add fields to MemoryRecordResponse + update all construction sites

**Files:**
- Modify: `src/web_server.rs` (struct definition at ~line 385 + 4 init sites + export handler)
- Test: `tests/integration/intelligence_wiring_sit.rs`

### Context

Current `MemoryRecordResponse` struct (line 385 in `src/web_server.rs`):
```rust
pub struct MemoryRecordResponse {
    id: String,
    record_type: String,
    timestamp: String,
    actor: String,
    action: String,
    target: String,
    metadata: serde_json::Value,
    integrity: Option<String>,
}
```

`MemoryRecord` fields available to add:
- `confidence: f32` (default 1.0)
- `source: Option<String>`
- `priority: String` (default "normal")
- `tags: Vec<String>`
- `version: u32`
- `status: String` (default "active")
- `expires_at: Option<i64>`

**4 construction sites** — all currently set `id`, `record_type`, `timestamp`, `actor`, `action`, `target`, `metadata`, `integrity`. Need to add 7 more fields to each.

- [ ] **Step 1.1: Write failing tests** — append to `tests/integration/intelligence_wiring_sit.rs`

```rust
#[test]
fn test_memory_record_response_has_confidence_field() {
    // MemoryRecord stores confidence — verify it's accessible
    let r = make_record("alice", "decided", "use_postgres");
    assert_eq!(r.confidence, 1.0); // default
}

#[test]
fn test_memory_record_response_fields_accessible() {
    // Verify all new response fields exist on MemoryRecord
    let mut r = make_record("alice", "decided", "use_postgres");
    r.priority = "high".to_string();
    r.tags = vec!["database".to_string()];
    r.version = 2;
    r.status = "active".to_string();
    r.source = Some("user-input".to_string());
    assert_eq!(r.priority, "high");
    assert_eq!(r.tags.len(), 1);
    assert_eq!(r.version, 2);
    assert_eq!(r.status, "active");
    assert!(r.source.is_some());
}

#[test]
fn test_memory_record_response_serialization() {
    // MemoryRecordResponse should be serializable with all fields
    // This is a compile-time + serde test
    use hipcortex::web_server::MemoryRecordResponse;
    let resp = MemoryRecordResponse {
        id:          "test-id".to_string(),
        record_type: "Symbolic".to_string(),
        timestamp:   "2026-01-01T00:00:00Z".to_string(),
        actor:       "alice".to_string(),
        action:      "decided".to_string(),
        target:      "use_postgres".to_string(),
        metadata:    serde_json::json!({}),
        integrity:   None,
        confidence:  0.9,
        source:      Some("user-input".to_string()),
        priority:    "high".to_string(),
        tags:        vec!["database".to_string()],
        version:     0,
        status:      "active".to_string(),
        expires_at:  None,
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["confidence"], 0.9);
    assert_eq!(json["priority"], "high");
    assert_eq!(json["tags"][0], "database");
    assert_eq!(json["status"], "active");
}
```

- [ ] **Step 1.2: Run — expect compile errors** (new fields don't exist on struct yet; `MemoryRecordResponse` not pub enough for test import)

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "error\[|test result"
```

- [ ] **Step 1.3: Update MemoryRecordResponse struct in src/web_server.rs**

Find:
```rust
pub struct MemoryRecordResponse {
    id: String,
    record_type: String,
    timestamp: String,
    actor: String,
    action: String,
    target: String,
    metadata: serde_json::Value,
    integrity: Option<String>,
}
```

Replace with:
```rust
pub struct MemoryRecordResponse {
    pub id:          String,
    pub record_type: String,
    pub timestamp:   String,
    pub actor:       String,
    pub action:      String,
    pub target:      String,
    pub metadata:    serde_json::Value,
    pub integrity:   Option<String>,
    // ── New fields (Sim #12 gap closure) ─────────────────────────────────
    /// Reliability signal [0.0, 1.0]. Use to filter low-confidence memories before injection.
    pub confidence:  f32,
    /// Who or what wrote this memory (e.g. "user-input", "claude-3-7", "tool-call").
    pub source:      Option<String>,
    /// "pinned" | "high" | "normal" | "low". Pinned = never decays, always retrieved.
    pub priority:    String,
    /// Domain tags for RAG filtering (e.g. ["database", "auth", "bug"]).
    pub tags:        Vec<String>,
    /// Update counter. 0 = original write. Increments on PATCH /memory/update/:id.
    pub version:     u32,
    /// "active" | "quarantine" | "archived". Quarantined = excluded from default search.
    pub status:      String,
    /// Unix timestamp (seconds) when this record expires. None = never expires.
    pub expires_at:  Option<i64>,
}
```

**Note:** Making fields `pub` enables the test to construct the struct. If this is undesirable (struct is API boundary), use the test pattern from existing tests instead.

- [ ] **Step 1.4: Update construction site 1 — handle_search_memory (~line 1151)**

Find (inside the `.map(|(r, score)| SearchResult {` block):
```rust
                    record: MemoryRecordResponse {
                        id:          r.id.to_string(),
                        record_type: format!("{:?}", r.record_type),
                        timestamp:   r.timestamp.to_rfc3339(),
                        actor:       r.actor.clone(),
                        action:      r.action.clone(),
                        target:      r.target.clone(),
                        metadata:    r.metadata.clone(),
                        integrity:   r.integrity.clone(),
                    },
```

Replace with:
```rust
                    record: MemoryRecordResponse {
                        id:          r.id.to_string(),
                        record_type: format!("{:?}", r.record_type),
                        timestamp:   r.timestamp.to_rfc3339(),
                        actor:       r.actor.clone(),
                        action:      r.action.clone(),
                        target:      r.target.clone(),
                        metadata:    r.metadata.clone(),
                        integrity:   r.integrity.clone(),
                        confidence:  r.confidence,
                        source:      r.source.clone(),
                        priority:    r.priority.clone(),
                        tags:        r.tags.clone(),
                        version:     r.version,
                        status:      r.status.clone(),
                        expires_at:  r.expires_at,
                    },
```

- [ ] **Step 1.5: Update construction site 2 — run_with_both_stores handle_query_memory (~line 1631)**

Find (inside `let response_records = records.into_iter().map(|r| MemoryRecordResponse {`):
```rust
            let response_records = records.into_iter().map(|r| MemoryRecordResponse {
                id:          r.id.to_string(),
                record_type: format!("{:?}", r.record_type),
                timestamp:   r.timestamp.to_rfc3339(),
                actor:       r.actor.clone(),
                action:      r.action.clone(),
                target:      r.target.clone(),
                metadata:    r.metadata.clone(),
                integrity:   r.integrity.clone(),
            }).collect::<Vec<_>>();
```

Replace with:
```rust
            let response_records = records.into_iter().map(|r| MemoryRecordResponse {
                id:          r.id.to_string(),
                record_type: format!("{:?}", r.record_type),
                timestamp:   r.timestamp.to_rfc3339(),
                actor:       r.actor.clone(),
                action:      r.action.clone(),
                target:      r.target.clone(),
                metadata:    r.metadata.clone(),
                integrity:   r.integrity.clone(),
                confidence:  r.confidence,
                source:      r.source.clone(),
                priority:    r.priority.clone(),
                tags:        r.tags.clone(),
                version:     r.version,
                status:      r.status.clone(),
                expires_at:  r.expires_at,
            }).collect::<Vec<_>>();
```

**IMPORTANT:** There are TWO `handle_query_memory` functions (one in run_with_both_stores, one in run_with_state). Read lines around 1619 to identify which one this is before editing.

- [ ] **Step 1.6: Update construction site 3 — handle_latest_memory (~line 1641)**

Find (inside `let response_records = records.into_iter().map(|r| MemoryRecordResponse {` in `handle_latest_memory`):
```rust
            let response_records = records.into_iter().map(|r| MemoryRecordResponse {
                id:          r.id.to_string(),
                record_type: format!("{:?}", r.record_type),
                timestamp:   r.timestamp.to_rfc3339(),
                actor:       r.actor.clone(),
                action:      r.action.clone(),
                target:      r.target.clone(),
                metadata:    r.metadata.clone(),
                integrity:   r.integrity.clone(),
            }).collect::<Vec<_>>();
```

Replace with the same expanded struct as Steps 1.4/1.5.

**Identification:** This is in `handle_latest_memory` function body. Check that the surrounding function name is `handle_latest_memory` before editing.

- [ ] **Step 1.7: Update construction site 4 — run_with_state handle_query_memory (~line 2607)**

Find (inside `.map(|r| MemoryRecordResponse {` in the main query handler):
```rust
                .map(|r| MemoryRecordResponse {
                    id: r.id.to_string(),
                    record_type: format!("{:?}", r.record_type),
                    timestamp: r.timestamp.to_rfc3339(),
                    actor: r.actor.clone(),
                    action: r.action.clone(),
                    target: r.target.clone(),
                    metadata: r.metadata.clone(),
                    integrity: r.integrity.clone(),
                })
```

Replace with:
```rust
                .map(|r| MemoryRecordResponse {
                    id:          r.id.to_string(),
                    record_type: format!("{:?}", r.record_type),
                    timestamp:   r.timestamp.to_rfc3339(),
                    actor:       r.actor.clone(),
                    action:      r.action.clone(),
                    target:      r.target.clone(),
                    metadata:    r.metadata.clone(),
                    integrity:   r.integrity.clone(),
                    confidence:  r.confidence,
                    source:      r.source.clone(),
                    priority:    r.priority.clone(),
                    tags:        r.tags.clone(),
                    version:     r.version,
                    status:      r.status.clone(),
                    expires_at:  r.expires_at,
                })
```

- [ ] **Step 1.8: Update handle_export_memory (~line 1215)**

The export handler uses inline `serde_json::json!`. Find:
```rust
                    "integrity":   r.integrity,
                    "expires_at":  r.expires_at,
```

Replace with:
```rust
                    "integrity":   r.integrity,
                    "confidence":  r.confidence,
                    "source":      r.source,
                    "priority":    r.priority,
                    "tags":        r.tags,
                    "version":     r.version,
                    "status":      r.status,
                    "expires_at":  r.expires_at,
```

- [ ] **Step 1.9: Build check**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo check --no-default-features --features "web-server,petgraph_backend" 2>&1 | Select-String "^error"
```
Expected: 0 errors

- [ ] **Step 1.10: Run tests**

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\msys64\mingw64\bin;$env:PATH"
cargo test --no-default-features --features "web-server,petgraph_backend" --test integration_suite intelligence_wiring 2>&1 | Select-String "test result|FAILED"
```
Expected: 32 passed; 0 failed (29 existing + 3 new)

- [ ] **Step 1.11: Commit + push**

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git add src/web_server.rs tests/integration/intelligence_wiring_sit.rs docs/superpowers/
git commit -m "feat: MemoryRecordResponse exposes confidence/source/priority/tags/version/status/expires_at"
git push origin claude/pedantic-edison-28b84c
```

---

## Acceptance Criteria

- [ ] `GET /memory/query` response records include `confidence`, `source`, `priority`, `tags`, `version`, `status`, `expires_at`
- [ ] `POST /memory/search` response records include same fields
- [ ] `GET /memory/latest` response records include same fields
- [ ] `GET /memory/export` includes same fields
- [ ] `MemoryRecordResponse` struct has all 7 new fields with `pub` visibility
- [ ] 32 tests pass (29 existing + 3 new)
- [ ] `cargo check` clean
- [ ] No existing fields removed or type-changed

## Known Gotchas

1. **Two `handle_query_memory` functions** — one in `run_with_both_stores` (~line 1631) and one in `run_with_state` (~line 2607). Both must be updated. Identify by reading surrounding function names.
2. **`records.into_iter()` vs `filtered_records.into_iter()`** — the two query handlers use different variable names. Match exactly.
3. **`pub` on struct fields** — needed for test to construct `MemoryRecordResponse` directly. If test #3 causes issues, simplify it to just test `MemoryRecord` fields instead.
4. **`handle_latest_memory` uses `records.into_iter()`** — same pattern as query handler.

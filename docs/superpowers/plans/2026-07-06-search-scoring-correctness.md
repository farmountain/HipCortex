# Search Scoring Correctness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix three bugs in `MemoryStore::search_semantic` — expired records leak into search results, and `high`/`low` priority and per-record decay have no effect on scoring despite being stored on every record.

**Architecture:** All changes are in `src/memory_store.rs`. Task 1 adds an `expires_at` predicate to the existing filter chain (two locations). Task 2 inserts a `priority_mult` factor into the weighted-score line. Task 3 adds a `compute_decay()` helper and folds it into the same score line. The three tasks compose onto the same scoring formula in order; each is independently testable. No new files, no new dependencies.

**Tech Stack:** Rust, chrono (already in Cargo.toml), existing `MemoryRecord` fields (`expires_at`, `priority`, `confidence`, metadata `decay_factor` / `decay_half_life_secs`).

## Global Constraints

- Test command: `cargo test --no-default-features --features "petgraph_backend" --lib`
- Integration test command: `cargo test --no-default-features --features "petgraph_backend" --test integration_suite`
- All new tests go in `tests/integration/web_server_gaps_sit.rs` (already registered in `tests/integration/mod.rs`, no feature gate needed)
- `search_semantic` signature must NOT change — all callers pass the same 4 args
- `compute_decay` is a free function (`pub(crate)` or private), not a method on MemoryStore
- Pinned records already bypassed with score `2.0` — do NOT apply `priority_mult` or decay to them (they stay at `2.0` regardless)
- `compute_decay` returns `rec.confidence as f64` (not 1.0) when λ=0 or elapsed<1s — confidence is still applied as base multiplier

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `src/memory_store.rs:398-401` | Add `expires_at` predicate to main filter |
| Modify | `src/memory_store.rs:429-431` | Add `expires_at` predicate to pinned-records filter |
| Modify | `src/memory_store.rs:425` | Add `priority_mult` to weighted-score line |
| Add | `src/memory_store.rs` (before `search_semantic`) | `compute_decay(rec: &MemoryRecord) -> f64` helper |
| Modify | `src/memory_store.rs:425` | Multiply weighted score by `compute_decay(rec)` |
| Modify | `tests/integration/web_server_gaps_sit.rs` | Add 7 new tests across the three tasks |

---

### Task 1: Filter expired records from `search_semantic`

**Files:**
- Modify: `src/memory_store.rs:398-401` (main filter in `search_semantic`)
- Modify: `src/memory_store.rs:429-431` (pinned-records filter in `search_semantic`)
- Test: `tests/integration/web_server_gaps_sit.rs`

**Interfaces:**
- Consumes: `MemoryRecord.expires_at: Option<i64>` (Unix timestamp seconds; `None` = never expires)
- Produces: `search_semantic` now silently drops expired records — same return type, same signature

- [ ] **Step 1: Write three failing tests**

Open `tests/integration/web_server_gaps_sit.rs`. Add imports at the top if not present:
```rust
use chrono::{Duration, Utc};
```

Then append these three tests:

```rust
#[test]
fn test_search_excludes_expired_records() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.expires_at = Some(Utc::now().timestamp() - 1); // 1 second in the past
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        results.is_empty(),
        "expired record must not appear in search_semantic results"
    );
}

#[test]
fn test_search_includes_non_expired_record() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.expires_at = Some(Utc::now().timestamp() + 3600); // 1 hour from now
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        !results.is_empty(),
        "non-expired record must appear in search_semantic results"
    );
}

#[test]
fn test_search_excludes_expired_pinned_record() {
    let mut store = make_store();
    let mut r = make_record("alice", "decided", "use postgres");
    r.priority = "pinned".to_string();
    r.expires_at = Some(Utc::now().timestamp() - 1); // expired
    store.add(r).unwrap();
    let results = store.search_semantic(None, "postgres", 10, false);
    assert!(
        results.is_empty(),
        "expired pinned record must not appear in search_semantic (pinned bypass is score, not expiry)"
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_search_excludes_expired -- --nocapture
```

Expected: FAIL — expired record appears (bug confirmed).

- [ ] **Step 3: Fix the main filter in `search_semantic`**

Open `src/memory_store.rs`. Find the `search_semantic` function. The current filter is:

```rust
        let mut scored: Vec<(&MemoryRecord, f64)> = self
            .records
            .iter()
            .filter(|r| include_quarantined || r.status != "quarantine")
```

Replace with:

```rust
        let now_ts = chrono::Utc::now().timestamp();
        let mut scored: Vec<(&MemoryRecord, f64)> = self
            .records
            .iter()
            .filter(|r| {
                (include_quarantined || r.status != "quarantine")
                    && r.expires_at.map_or(true, |exp| exp > now_ts)
            })
```

- [ ] **Step 4: Fix the pinned-records filter in the same function**

Further down in the same function, find:

```rust
        // Pinned active records always appear first, regardless of score
        let mut pinned: Vec<(&MemoryRecord, f64)> = self.records.iter()
            .filter(|r| r.priority == "pinned" && (include_quarantined || r.status != "quarantine"))
```

Replace with (reuse `now_ts` from Step 3 — it's in scope):

```rust
        // Pinned active records always appear first, regardless of score
        let mut pinned: Vec<(&MemoryRecord, f64)> = self.records.iter()
            .filter(|r| {
                r.priority == "pinned"
                    && (include_quarantined || r.status != "quarantine")
                    && r.expires_at.map_or(true, |exp| exp > now_ts)
            })
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_search_excludes_expired -- --nocapture
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_search_includes_non_expired -- --nocapture
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_search_excludes_expired_pinned -- --nocapture
```

Expected: 3 PASS.

- [ ] **Step 6: Run full lib tests to confirm no regressions**

```
cargo test --no-default-features --features "petgraph_backend" --lib
```

Expected: all pass.

- [ ] **Step 7: Commit**

```
git add src/memory_store.rs tests/integration/web_server_gaps_sit.rs
git commit -m "fix(search): exclude expired records from search_semantic (main + pinned filters)"
```

---

### Task 2: Apply `high`/`low` priority as score multipliers

**Files:**
- Modify: `src/memory_store.rs` (inside `search_semantic`, the `weighted` score line ~line 425)
- Test: `tests/integration/web_server_gaps_sit.rs`

**Interfaces:**
- Consumes: `MemoryRecord.priority: String` — values `"pinned" | "high" | "normal" | "low"` (pinned records never reach this code path — they are handled separately at score 2.0)
- Produces: `high` records score ×1.5, `low` records score ×0.5, `normal` unchanged

- [ ] **Step 1: Write two failing tests**

Append to `tests/integration/web_server_gaps_sit.rs`:

```rust
#[test]
fn test_high_priority_ranks_above_normal_for_same_content() {
    let mut store = make_store();

    let mut high_r = make_record("alice", "decided", "use postgres as database");
    high_r.priority = "high".to_string();
    store.add(high_r.clone()).unwrap();

    let normal_r = make_record("bob", "decided", "use postgres as database");
    store.add(normal_r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres as database", 10, false);
    let high_pos = results.iter().position(|(r, _)| r.id == high_r.id).expect("high record missing");
    let normal_pos = results.iter().position(|(r, _)| r.id == normal_r.id).expect("normal record missing");
    assert!(
        high_pos < normal_pos,
        "high priority record (pos {}) must rank above normal priority (pos {})",
        high_pos, normal_pos
    );
}

#[test]
fn test_low_priority_ranks_below_normal_for_same_content() {
    let mut store = make_store();

    let mut low_r = make_record("alice", "decided", "use redis for cache");
    low_r.priority = "low".to_string();
    store.add(low_r.clone()).unwrap();

    let normal_r = make_record("bob", "decided", "use redis for cache");
    store.add(normal_r.clone()).unwrap();

    let results = store.search_semantic(None, "use redis for cache", 10, false);
    let low_pos = results.iter().position(|(r, _)| r.id == low_r.id).expect("low record missing");
    let normal_pos = results.iter().position(|(r, _)| r.id == normal_r.id).expect("normal record missing");
    assert!(
        low_pos > normal_pos,
        "low priority record (pos {}) must rank below normal priority (pos {})",
        low_pos, normal_pos
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_high_priority_ranks -- --nocapture
```

Expected: FAIL — positions are equal (priority has no effect today).

- [ ] **Step 3: Add priority_mult to the weighted-score computation**

In `src/memory_store.rs`, inside `search_semantic`, find the `map(|rec| {...})` closure. The current weighted-score line reads:

```rust
                let weighted = base_score * (0.5 + 0.5 * trust); // range: 0.5x-1.0x of base
```

Replace with:

```rust
                // Priority multiplier: high=1.5×, low=0.5×, normal/pinned=1.0×
                // Note: pinned records take a separate code path (score 2.0 override).
                let priority_mult: f64 = match rec.priority.as_str() {
                    "high" => 1.5,
                    "low"  => 0.5,
                    _      => 1.0,
                };
                let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult;
```

- [ ] **Step 4: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_high_priority_ranks -- --nocapture
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_low_priority_ranks -- --nocapture
```

Expected: 2 PASS.

- [ ] **Step 5: Run full lib tests**

```
cargo test --no-default-features --features "petgraph_backend" --lib
```

Expected: all pass.

- [ ] **Step 6: Commit**

```
git add src/memory_store.rs tests/integration/web_server_gaps_sit.rs
git commit -m "feat(search): apply high/low priority multipliers in search_semantic scoring"
```

---

### Task 3: Apply time-based decay to search scoring

**Files:**
- Modify: `src/memory_store.rs` — add `compute_decay` helper function before `search_semantic`, then multiply into weighted score
- Test: `tests/integration/web_server_gaps_sit.rs`

**Interfaces:**
- Consumes:
  - `MemoryRecord.confidence: f32` — base decay multiplier [0.0, 1.0]
  - `MemoryRecord.timestamp: DateTime<Utc>` — elapsed time computed from here
  - `MemoryRecord.metadata["decay_factor"]: f64` — λ (default 1.0; 0 = no decay)
  - `MemoryRecord.metadata["decay_half_life_secs"]: f64` — t½ in seconds (default 2_592_000 = 30 days)
- Produces:
  - `compute_decay(rec: &MemoryRecord) -> f64` — returns `[0.0, confidence]`
  - Scoring formula updated: `base × trust_factor × priority_mult × compute_decay(rec)`

**Decay formula:** `confidence × exp(−λ × elapsed / t½)`

| λ | t½ | elapsed=0 | elapsed=t½ | elapsed=2t½ |
|---|---|---|---|---|
| 1.0 | 30 days | conf×1.0 | conf×0.5 | conf×0.25 |
| 0.0 | any | conf (no decay) | conf | conf |
| 2.0 | 1 day | conf×1.0 | conf×0.25 | conf×0.0625 |

- [ ] **Step 1: Write two failing tests**

Append to `tests/integration/web_server_gaps_sit.rs`:

```rust
#[test]
fn test_old_record_with_short_half_life_scores_lower_than_fresh() {
    let mut store = make_store();

    // "Old" record: 200 seconds old, half-life 100s → 2 half-lives → score ≈ 0.25×
    let mut old_r = make_record("alice", "decided", "use postgres now");
    old_r.timestamp = Utc::now() - Duration::seconds(200);
    old_r.metadata = serde_json::json!({
        "decay_factor": 1.0,
        "decay_half_life_secs": 100
    });
    store.add(old_r.clone()).unwrap();

    // "Fresh" record: same content, just created (timestamp = Utc::now())
    let fresh_r = make_record("bob", "decided", "use postgres now");
    store.add(fresh_r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres now", 10, false);
    let old_score = results.iter().find(|(r, _)| r.id == old_r.id).map(|(_, s)| *s).unwrap_or(0.0);
    let fresh_score = results.iter().find(|(r, _)| r.id == fresh_r.id).map(|(_, s)| *s).unwrap_or(0.0);
    assert!(
        old_score < fresh_score,
        "decayed old record (score {:.4}) must score lower than fresh record (score {:.4})",
        old_score, fresh_score
    );
}

#[test]
fn test_zero_decay_factor_record_is_not_decayed() {
    let mut store = make_store();

    // Very old record but λ=0 → no decay regardless of age
    let mut r = make_record("alice", "decided", "use postgres");
    r.timestamp = Utc::now() - Duration::seconds(999_999_999); // ancient
    r.metadata = serde_json::json!({"decay_factor": 0.0});
    r.confidence = 1.0;
    store.add(r.clone()).unwrap();

    let results = store.search_semantic(None, "use postgres", 10, false);
    let score = results.iter().find(|(rec, _)| rec.id == r.id).map(|(_, s)| *s).unwrap_or(0.0);
    // λ=0 → decay=1.0, conf=1.0, trust default=0.5, keyword=1.0, priority=1.0
    // weighted = 1.0 * (0.5 + 0.5*0.5) * 1.0 * 1.0 = 0.75
    assert!(score > 0.5, "λ=0 record must not decay (score {:.4} must be > 0.5)", score);
}
```

- [ ] **Step 2: Run tests to confirm they fail**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_old_record_with_short_half -- --nocapture
```

Expected: FAIL — old and fresh records score the same (decay not yet implemented).

- [ ] **Step 3: Add `compute_decay` helper function**

In `src/memory_store.rs`, immediately **before** the `pub fn search_semantic` function, insert:

```rust
/// Compute time-based confidence decay for a memory record.
///
/// Formula: `confidence × exp(−λ × elapsed_seconds / t½)`
///
/// Parameters read from `rec.metadata`:
///   - `"decay_factor"` (λ): rate multiplier, default 1.0; 0.0 = no decay
///   - `"decay_half_life_secs"` (t½): seconds for confidence to halve, default 2,592,000 (30 days)
///
/// Returns `rec.confidence as f64` unchanged when λ=0 or elapsed < 1s.
fn compute_decay(rec: &MemoryRecord) -> f64 {
    let elapsed_secs = (chrono::Utc::now().timestamp() - rec.timestamp.timestamp()).max(0) as f64;
    let conf = rec.confidence as f64;
    if elapsed_secs < 1.0 {
        return conf;
    }
    let lambda = rec
        .metadata
        .get("decay_factor")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0)
        .clamp(0.0, 10.0);
    if lambda < f64::EPSILON {
        return conf; // λ=0 → no decay
    }
    let half_life = rec
        .metadata
        .get("decay_half_life_secs")
        .and_then(|v| v.as_f64())
        .unwrap_or(2_592_000.0) // 30 days
        .max(1.0);
    conf * (-lambda * elapsed_secs / half_life).exp()
}
```

- [ ] **Step 4: Fold `compute_decay` into the scoring formula**

In `search_semantic`, the current weighted-score line (after Task 2) reads:

```rust
                let priority_mult: f64 = match rec.priority.as_str() {
                    "high" => 1.5,
                    "low"  => 0.5,
                    _      => 1.0,
                };
                let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult;
```

Replace with:

```rust
                // Priority multiplier: high=1.5×, low=0.5×, normal/pinned=1.0×
                // Note: pinned records take a separate code path (score 2.0 override).
                let priority_mult: f64 = match rec.priority.as_str() {
                    "high" => 1.5,
                    "low"  => 0.5,
                    _      => 1.0,
                };
                let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult * compute_decay(rec);
```

- [ ] **Step 5: Run tests to confirm they pass**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_old_record_with_short_half -- --nocapture
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_zero_decay_factor -- --nocapture
```

Expected: 2 PASS.

- [ ] **Step 6: Run full test suite**

```
cargo test --no-default-features --features "petgraph_backend" --lib
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
```

Expected: all pass. If any existing tests fail, check that `make_record` produces records with `timestamp = Utc::now()` — they have default `decay_factor` absent from metadata → `compute_decay` returns `confidence(=1.0) × exp(-1.0 × near_zero / 30_days) ≈ 1.0`, so impact on fast tests is negligible.

- [ ] **Step 7: Commit**

```
git add src/memory_store.rs tests/integration/web_server_gaps_sit.rs
git commit -m "feat(search): add time-based decay scoring via confidence×exp(-λt/t½)"
```

---

## Self-Review

### Spec coverage

| Gap | Task covering it |
|-----|-----------------|
| Expired records in `search_semantic` (main + pinned) | Task 1 |
| `high`/`low` priority scoring | Task 2 |
| `decay_factor`/`half_life` applied in scoring | Task 3 |

### Type consistency
- `compute_decay(rec: &MemoryRecord) -> f64` — used in Task 3 scoring line — consistent.
- `now_ts` defined at top of `search_semantic` in Task 1 — reused in pinned filter within same function — consistent.
- `priority_mult` introduced in Task 2 then referenced in Task 3's replacement block — consistent.

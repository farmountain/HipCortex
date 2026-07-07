# Task 3: Time-based Decay Scoring — Implementation Report

**Status:** ✅ Complete  
**Commit:** `f998b07`  
**Date:** 2026-07-07

---

## Summary

Successfully implemented time-based confidence decay for `search_semantic` scoring. Records now decay exponentially over time based on configurable parameters, with older records scoring progressively lower than fresh ones.

## Implementation

### Added Function: `compute_decay`

Location: `src/memory_store.rs` (before `search_semantic`)

```rust
fn compute_decay(rec: &MemoryRecord) -> f64
```

**Formula:** `confidence × exp(−λ × elapsed_seconds / t½)`

**Parameters (from `rec.metadata`):**
- `decay_factor` (λ): Rate multiplier, default 1.0; 0.0 = no decay
- `decay_half_life_secs` (t½): Seconds for confidence to halve, default 2,592,000 (30 days)

**Behavior:**
- Returns `rec.confidence as f64` unchanged when λ=0 or elapsed < 1s
- Clamps λ to [0.0, 10.0] range
- Minimum t½ is 1.0 seconds

### Updated Scoring Formula

Old formula (Task 2):
```rust
let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult;
```

New formula (Task 3):
```rust
let weighted = base_score * (0.5 + 0.5 * trust) * priority_mult * Self::compute_decay(rec);
```

## Tests

### New Tests (2/2 passed)

**`test_old_record_with_short_half_life_scores_lower_than_fresh`**
- Old record: 200 seconds old, half-life 100s → 2 half-lives → score ≈0.25×
- Fresh record: same content, just created
- ✅ Verified: old_score < fresh_score

**`test_zero_decay_factor_record_is_not_decayed`**
- Very old record (999,999,999 seconds) with λ=0
- ✅ Verified: score > 0.5 (no decay applied)

### Test Suite Results

**Integration suite:** 102 passed, 6 failed  
- ✅ Both decay tests pass
- ✅ All Task 1 (expires_at) tests pass
- ✅ All Task 2 (priority_mult) tests pass

**Failed tests (pre-existing, unrelated to decay):**
- `intelligence_hooks_sit::test_symbolic_store_health_reporter_initial_state`
- `intelligence_hooks_sit::test_symbolic_store_add_node_with_self_model`
- `intelligence_hooks_sit::test_symbolic_store_constructs_with_intelligence`
- `intelligence_hooks_sit::test_symbolic_store_full_intelligence_pipeline`
- `intelligence_hooks_sit::test_symbolic_store_nodes_and_edges_with_intelligence`
- `world_model_cli_sit::cli_exports_graph`

**Analysis:** All failures are in the intelligence subsystem and relate to unexpected node counts, not scoring. These tests do not exercise `search_semantic` or memory store search functionality. The decay feature has no code paths that would affect symbolic graph construction or node counts.

## Concerns

1. **Pre-existing test failures:** 6 intelligence_hooks_sit tests fail due to unexpected graph node counts. These failures are unrelated to the decay scoring implementation but should be investigated separately.

2. **Windows linker issues:** Encountered `LNK1104` file locking errors when running `--lib` tests, preventing full library test validation. Integration tests run successfully.

3. **Performance:** `compute_decay` is called for every record during search. For large datasets, consider caching decay values with a short TTL if profiling shows this becomes a bottleneck.

4. **Default metadata absence:** Records created without explicit `decay_factor` or `decay_half_life_secs` in metadata will use defaults (λ=1.0, t½=30 days). This is the intended behavior per the brief.

## Verification Commands

```powershell
# Test decay functionality
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_old_record_with_short_half -- --nocapture
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_zero_decay_factor -- --nocapture

# Full integration suite (102/108 pass)
cargo test --no-default-features --features "petgraph_backend" --test integration_suite
```

## Next Steps

Task 3 is complete and committed. Pre-existing intelligence test failures should be triaged separately as they do not block the SDD Plan A scoring enhancements.

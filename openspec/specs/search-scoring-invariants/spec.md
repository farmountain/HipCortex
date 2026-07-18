## Purpose
This specification defines the invariants for search scoring, Personalized PageRank, and the SymbolicStore identity anchor.
## Requirements
### Requirement: Decay scoring is always suppressive
The `compute_decay` function SHALL return a value in the range `[0.0, rec.confidence]`.
It MUST NOT return a value exceeding `rec.confidence`, ensuring decay can only reduce
or preserve relevance scores, never amplify them. The record `confidence` field at ingestion
time acts as a permanent ceiling on the maximum achievable weighted score for that record,
regardless of query relevance or time elapsed.

#### Scenario: Zero elapsed time
- **WHEN** a record was just ingested (elapsed < 1 second)
- **THEN** `compute_decay` returns exactly `rec.confidence` (no decay applied)

#### Scenario: Non-zero elapsed time with default parameters
- **WHEN** 30 days have elapsed, `decay_factor=1.0`, `decay_half_life_secs=2592000`
- **THEN** `compute_decay` returns approximately `0.5 * rec.confidence` (one half-life)

#### Scenario: Zero decay factor
- **WHEN** `decay_factor=0.0` in record metadata
- **THEN** `compute_decay` returns `rec.confidence` unchanged (no-decay mode)

#### Scenario: Low confidence ceiling
- **WHEN** a record has `confidence=0.1` and full semantic relevance (`base_score=1.0`)
- **THEN** the final weighted score SHALL be at most `0.1 * trust_factor * priority_mult`
  regardless of query match quality

### Requirement: PPR search and semantic search use independent scoring models
The `GET /memory/search/related` endpoint (PPR/graph search) SHALL score results by
graph centrality from the seed node via Personalized PageRank. It MUST NOT apply the
pinned-record score override (2.0) used by `GET /memory/query` and `search_semantic`.
Pinned priority is a search-ranking concept; PPR scores reflect topological proximity.

#### Scenario: Pinned record in PPR results
- **WHEN** a pinned record is reachable from the seed node via the CausalTopoGraph
- **THEN** it appears in results with its PPR-derived score (not overridden to 2.0)

#### Scenario: Unlinked pinned record
- **WHEN** a pinned record has no edges in the CausalTopoGraph connecting it to the seed
- **THEN** it SHALL NOT appear in `search_related` results
  (PPR only surfaces graph-reachable nodes)

#### Scenario: High-degree pinned record dominates PPR naturally
- **WHEN** a pinned record has many incoming edges from other linked records
- **THEN** its PPR score SHALL be proportionally higher due to graph in-degree
  (graph topology, not priority label, determines PPR prominence)

### Requirement: SymbolicStore always contains a System:Self identity anchor
`SymbolicStore::new()` SHALL always initialize with a `System:Self` node bearing
`role="canonical_identity_anchor"`. This node SHALL be present before any user-defined
nodes are added. Tests asserting node counts SHALL account for this pre-seeded node.

#### Scenario: Fresh store node count
- **WHEN** a new `SymbolicStore` is created via `SymbolicStore::new()`
- **THEN** `export_graph().0.len()` SHALL equal 1 (the System:Self anchor)

#### Scenario: Test node count after adds
- **WHEN** a test adds N nodes to a fresh store
- **THEN** `export_graph().0.len()` SHALL equal `1 + N` (anchor plus user nodes)

### Requirement: Pinned search candidate ordering under truncation limits
When the `search_semantic` or hybrid search pipeline collects records with `priority == "pinned"`, it SHALL sort those candidates descending by `timestamp` before merging or truncating to `limit`. This guarantees that high-volume pinned memory collections do not suffer from insertion-order starvation where old initial pinned records crowd out newly created pinned records.

#### Scenario: High volume pinned candidate ordering
- **WHEN** multiple pinned candidates match semantic filters and total pinned count `P` exceeds `limit` `L`
- **THEN** the top `L` candidates retained by the search pipeline SHALL be the `L` most recently timestamped pinned records


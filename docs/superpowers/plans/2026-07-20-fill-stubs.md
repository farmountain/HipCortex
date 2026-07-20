# Plan: Fill real stubs (not UI placeholders)

## Inventory (actionable)
| Stub | Location | Action |
|------|----------|--------|
| search.rs empty | topological_memory | PPR/path/Markov walk APIs |
| contradiction.rs empty | topological_memory | contradiction report API |
| deconstructor.rs empty | topological_memory | rule-based hyp parser |
| detect_coverage_gap | loop_engine | PPR/strata coverage |
| propagate_uncertainty | loop_engine | edge conf decay |
| simulate_rollouts | loop_engine | use WM + topo nodes |
| check_graph_consistency | coherence | WM vs symbolic edges |
| check_entity_permanence | coherence | topo ∩ symbolic |
| check_memory_consistency | invariants | use entity_cache |
| get_candidate_values | resolver | from report metadata |
| migrate/import_trace | scripts | minimal real CLI |

Skip: UI placeholders, optional-dep stubs (crewai/autogen import fallbacks), intentional test noops.

## Tasks
1. topo search + contradiction + deconstructor
2. loop_engine real gap/uncertainty/sim
3. coherence checker/invariants/resolver
4. scripts placeholders
5. tests + commit

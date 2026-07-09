# procedural-attention Specification

## Purpose
Topological recency vector decay context eviction and dynamic procedural FSM skill compilation.

## Requirements
### Requirement: Topological and Recency Vector Decay Context Eviction
The system SHALL provide a `SessionContext::evict_with_topological_decay` method that evicts paged context items based on a weighted combination of structural topological relevance and exponential time recency. Specifically, the method SHALL compute personalized PageRank over `CausalTopoGraph` using active task node identifiers as seed vectors, combine each item's PageRank score with exponential age decay ($0.7 \times \text{PR} + 0.3 \times e^{-0.001 \times \text{age}}$), sort items ascending by combined score, and evict the lowest-scoring items until the target token budget is satisfied.

#### Scenario: Topological context eviction favoring causal seeds
- **WHEN** `SessionContext::evict_with_topological_decay` is invoked when `estimated_tokens` exceeds the budget while active task seeds are provided
- **THEN** it evicts items with the lowest combined PageRank and recency scores first, retaining items that are topologically central to the active task seeds even if older in age

### Requirement: Episodic Skill Sequence Extraction and Parameterization
The system SHALL provide a `SkillCompiler` struct capable of analyzing historical action sequences (`parameterize_trace`). When an action sequence meets or exceeds `success_threshold`, `SkillCompiler` SHALL abstract concrete target entity parameters (such as file paths or symbol names) into positional variable placeholders (`$arg0`, `$arg1`), producing reusable `SkillTemplate` objects.

#### Scenario: Parameterization of concrete action traces into templates
- **WHEN** `SkillCompiler::parameterize_trace` is called with an action sequence and concrete target entity strings
- **THEN** it maps unique target entities to `$arg` placeholders and returns a parameterized sequence of template actions

### Requirement: Dynamic Procedural FSM Transition Compilation
The system SHALL provide `SkillCompiler::compile_and_register_skill` to compile `SkillTemplate` structures directly into finite state machine rules (`FSMTransition`) and register them inside the `ProceduralCache` without requiring a system restart.

#### Scenario: Registration of compiled skill into procedural cache
- **WHEN** `SkillCompiler::compile_and_register_skill` is invoked with a valid `SkillTemplate` and a mutable reference to `ProceduralCache`
- **THEN** it generates `FSMState` and `FSMTransition` objects for each step in the template and appends them to the cache transitions

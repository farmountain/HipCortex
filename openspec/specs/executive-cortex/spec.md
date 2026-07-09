# executive-cortex Specification

## Purpose
Hierarchical task graph planning solver, goal stack scheduling, and continuation checkpointing.

## Requirements
### Requirement: Hierarchical Task Graph State-Space Planning Solver
The system SHALL provide a `TaskGraph` structure powered by `petgraph::DiGraph<TaskNode, ()>` where each `TaskNode` encapsulates unique UUIDs, precondition maps, effect maps, cost metrics, and task states. The graph SHALL expose a `solve_planning_problem` method that uses an A* search algorithm over belief state transitions to produce a directed acyclic task plan reaching a target goal state from an initial belief state.

#### Scenario: Successful state-space task planning
- **WHEN** `TaskGraph::solve_planning_problem` is invoked with a valid initial belief map, target goal map, and a set of available candidate `TaskNode` actions
- **THEN** it returns a `TaskGraph` whose directed edges and node ordering represent a valid step-by-step plan satisfying all goal conditions with minimum cumulative path cost and unmet goal penalty

#### Scenario: Unsolvable planning problem handling
- **WHEN** `TaskGraph::solve_planning_problem` is invoked with target goals that cannot be reached using any sequence of available `TaskNode` actions
- **THEN** it returns an `Err` describing that no planning path was found

### Requirement: Active Goal Stack Scheduler with Pre-emptive Diagnostics
The system SHALL provide an `ExecutiveScheduler` structure maintaining an active `goal_stack` (`Vec<StackFrame>`) and an active `TaskGraph`. During each execution tick (`ExecutiveScheduler::tick()`), the scheduler SHALL assert that the `SelfModel` is healthy (`is_healthy()`) and that the `CoherenceChecker` allows write operations (`gate_write()`). If either invariant fails, the scheduler SHALL pre-empt regular step execution and push a pre-emptive Diagnostic Goal Frame onto the top of the stack without dropping existing stack frames.

#### Scenario: Execution pre-emption on system health fault
- **WHEN** `ExecutiveScheduler::tick()` is called while `SelfModel::is_healthy()` returns false
- **THEN** the scheduler pushes a new Diagnostic `StackFrame` onto `goal_stack` and returns an `Err` indicating a pre-emptive diagnostic interrupt

#### Scenario: Topologically ordered step progression
- **WHEN** `ExecutiveScheduler::tick()` executes on a healthy system with an active task step that completes
- **THEN** the scheduler updates `current_step` inside the active `StackFrame` to the next topological neighbor node in the `TaskGraph`, or pops the stack frame if all steps are complete

### Requirement: Continuation Checkpoint Persistence
The system SHALL allow `ContinuationCheckpoint` instances to serialize active `goal_stack` frames into `ContinuationCheckpointData` JSON structures and persist them into the storage backend (`MemoryStore<B>`) as `MemoryType::Reflexion` records.

#### Scenario: Checkpoint persistence to storage backend
- **WHEN** `ContinuationCheckpoint::persist_to_store()` is called with an active slice of `StackFrame` items
- **THEN** it serializes task IDs, belief maps, and current step indices into JSON, adds a `MemoryType::Reflexion` record to `MemoryStore<B>`, and returns the record ID

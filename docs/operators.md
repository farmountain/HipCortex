# HipCortex Canonical Operators (v1.0.0)

## Base Cognitive Operators (6)

| Operator | CognitiveDelta variant | Mathematical | REST | MCP tool |
|----------|----------------------|-------------|------|----------|
| AddMemory | `AddMemory(MemoryRecord)` | ΔM | `POST /v1/cognitive/transact` | `add_memory` |
| UpdateBelief | `UpdateBelief {id, payload}` | ΔB | transact | `update_belief` |
| AdvanceGoal | `AdvanceGoal {id, status}` | ΔG | transact | `advance_goal` |
| RetractBelief | `RetractBelief {id, reason}` | ΔB⁻ | transact | `retract_belief` |
| AutoConsolidate | `AutoConsolidate {min_frequency}` | ΔP | transact | `consolidate_memory` |
| WorkspaceOpen | `WorkspaceOpen {id, mode}` | ΔW | transact | `workspace_open` |

## SCM Operators (4, v1.0.0+)

| Operator | Pearl Tier | REST alias | MCP tool |
|----------|-----------|------------|----------|
| `Intervene {var, value}` | Tier 2 do(X=x) | `POST /v1/causal/intervene` | `causal_intervene` |
| `Counterfactual {actual_state, intervention_var, intervention_value}` | Tier 3 | `POST /v1/causal/counterfactual` | `causal_counterfactual` |
| `CreditAssign(FailureSignal)` | Tier 3 attribution | `POST /v1/causal/credit-assign` | `causal_credit_assign` |
| `RewriteStructuralEquation {node_id, new_weights}` | Tier 2 | `POST /v1/causal/rewrite-equation` | `causal_rewrite_equation` |

## MGV Metacognitive Operator

| REST | MCP tool | Returns |
|------|----------|---------|
| `POST /v1/mgv/check` | `mgv_check` | `{fok, jol, divergence, should_quarantine}` |

## Invariants

- All mutations flow through `POST /v1/cognitive/transact` (CognitiveDelta).
- REST aliases are thin wrappers that call transact internally.
- `SafetyGuardrail::check_precondition` is called before every mutation.
- `credit_assign_gated` requires `calibration_score >= 0.70` and `MMBPhenotype::SelectiveSensitivity`.

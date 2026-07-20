# Plan: MCTS + deep causal wiring

## Gap
- `/worldmodel/rollout` only calls `predict_multi_step` (needs 100+ obs trained predictors) → always error for normal use.
- Spec wants MCTS (`MctsSimulator`) over Dirichlet `TransitionModel`.
- HTTP causal uses heuristic `compute_intervention` / `compute_counterfactual`; empirical + SCM already exist but unused.

## Tasks
1. `WorldModelEnhanced::rollout_dirichlet` — multi-step MAP via TransitionModel
2. `WorldModelEnhanced::mcts_best_action` — wrap MctsSimulator
3. Wire `handle_wm_rollout` — dirichlet first; optional mode=mcts; keep ensemble fallback
4. Wire causal_intervention → empirical first, heuristic fallback; counterfactual → SCM
5. Unit tests + SIT observe→rollout success

## Constraints
- Do not remove/break `predict_multi_step` signature
- Surgical changes in world_model_enhanced + web_server handlers

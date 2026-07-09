## Why

`WorldModelEnhanced.predict_multi_step()` already implements multi-step ensemble rollout (sequence of actions → final predicted state + confidence), but there is no HTTP route exposing it. Agents and external tools can only call single-step prediction via `GET /worldmodel/predict`. Multi-step rollout is the natural way to answer "what happens if I do A, then B, then C?" and is essential for planning tasks.

## What Changes

- **Add `POST /worldmodel/rollout`**: New HTTP route accepting `{initial_state: string, actions: string[]}` and returning `{initial_state, actions, predicted_state, distribution, confidence, steps}` by delegating to the existing `predict_multi_step()`.
- **Add `WmRolloutRequest` struct**: Deserializable request body with `initial_state` (required) and `actions: Vec<String>` (required, non-empty validated).
- **Auth whitelist**: Add `/worldmodel/rollout` to the unauthenticated path list alongside `/worldmodel/causal`.
- **Optional LM tool**: Register `hipcortex_rollout` in the extension to make multi-step rollout available to Copilot agents.

## Capabilities

### New Capabilities

- `worldmodel-rollout`: Multi-step state prediction via `POST /worldmodel/rollout`. Takes an initial state and a sequence of N actions, returns ensemble-averaged final state prediction with confidence decay over steps.

### Modified Capabilities

_(none — additive only)_

## Impact

- **Files**: `src/web_server.rs` (handler + route + auth), `vscode-extension/src/extension.ts` (LM tool).
- **No breaking changes**: existing `/worldmodel/predict` unchanged.
- **Build**: `cargo test --no-default-features --features petgraph_backend` + `npm run compile`.
- **Dependency**: `predict_multi_step()` requires at least one trained predictor; returns `"No trained predictors available"` error otherwise — document in spec.

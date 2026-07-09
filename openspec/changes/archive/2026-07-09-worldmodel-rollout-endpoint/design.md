## Approach

Add a `POST /worldmodel/rollout` route to the Rust binary that delegates to the already-implemented `WorldModelEnhanced::predict_multi_step()`. The existing method handles ensemble averaging and confidence decay across steps. No new business logic needed — this is purely an HTTP exposure layer. Then wire an optional LM tool in the extension.

## Design Decisions

### D1: POST not GET for rollout

The request body contains a `Vec<String>` of actions which cannot be cleanly expressed as query params. POST with JSON body is the correct choice. Consistent with other WorldModel mutation-adjacent endpoints (`/worldmodel/observe`, `/worldmodel/entity`).

### D2: Non-empty `actions` validated in handler, not type system

`Vec<String>` can be empty. The handler returns a 200 with `{"error": "actions must be non-empty"}` for empty arrays (consistent with the error style used by other wm handlers) rather than a 400, keeping the error surface uniform.

### D3: Auth whitelist follows existing causal pattern

`/worldmodel/rollout` added to the unauthenticated path list alongside `/worldmodel/predict` and `/worldmodel/causal` — world model routes are not gated.

### D4: Response includes `initial_state` + `actions` echo

Echo the inputs back in the response to make the output self-documenting, especially useful for LLM consumption. Consistent with how `/worldmodel/predict` echoes `from_state` and `action`.

### D5: LM tool `hipcortex_rollout` is a separate tool from `hipcortex_predict`

`hipcortex_predict` is single-step and synchronous-feeling. `hipcortex_rollout` is multi-step and planning-oriented. Different model descriptions, different use cases. Keep them separate.

## Component Design

### `WmRolloutRequest` struct (add to web_server.rs)

```rust
#[cfg(feature = "web-server")]
#[derive(Deserialize)]
struct WmRolloutRequest {
    initial_state: String,
    actions: Vec<String>,
}
```

### `handle_wm_rollout()` handler (add to web_server.rs, ~20 lines)

```
Input:  Json<WmRolloutRequest>
Guard:  if req.actions.is_empty() → return {"error": "actions must be non-empty"}
Call:   world_model.read()?.predict_multi_step(req.initial_state, req.actions)
Output: {
  "initial_state":   echo,
  "actions":         echo,
  "predicted_state": pred.predicted_state,
  "distribution":    pred.distribution,      ← HashMap<String, f64>
  "confidence":      pred.confidence,        ← decreases with steps
  "steps":           pred.steps,             ← == actions.len()
}
Error paths: lock poisoned → {"error": "lock: ..."}, no predictors → {"error": "No trained predictors available"}
```

### Route binding (in build_router, near line 855)

```rust
.route("/worldmodel/rollout", post({
    let wm = world_model.clone();
    move |Json(req): Json<WmRolloutRequest>| async move {
        handle_wm_rollout(wm, Json(req)).await
    }
}))
```

### Auth whitelist (near line 1184)

```rust
|| path == "/worldmodel/rollout"
```

### `hipcortex_rollout` LM tool (extension.ts, ~30 lines)

```
modelDescription: "Predict state after a sequence of N actions using WorldModelEnhanced
  multi-step ensemble rollout. Provide initial_state (current state string) and
  actions (array of action strings). Returns predicted_state, confidence (decreases
  with steps), and probability distribution. Requires trained predictors — call
  /worldmodel/observe first if no predictions yet."

input: { initial_state: string, actions: string[] }
HTTP:  POST /worldmodel/rollout
output: JSON text of { predicted_state, confidence, steps, distribution }
```

## File Map

```
src/web_server.rs
  ├─ + WmRolloutRequest struct            ← near line 3719 (after WmPredictParams)
  ├─ + handle_wm_rollout() fn             ← after handle_wm_predict (~line 3742)
  ├─ route binding in build_router()      ← near line 855
  └─ auth whitelist entry                 ← near line 1184

vscode-extension/src/extension.ts
  └─ + hipcortex_rollout LM tool          ← after hipcortex_predict (~line 861)
```

## Verification

1. `cargo test --no-default-features --features petgraph_backend --test integration_suite` → 0 failed
2. `cargo check --no-default-features --features petgraph_backend` → 0 errors
3. `curl -X POST http://localhost:3000/worldmodel/rollout -H 'Content-Type: application/json' -d '{"initial_state":"idle","actions":["start","process"]}'`
   - With no trained predictors: `{"error":"No trained predictors available"}`
   - With predictors: `{"initial_state":"idle","actions":[...],"predicted_state":"...","confidence":0.xx,"steps":2}`
4. Empty actions: `{"error":"actions must be non-empty"}`
5. `cd vscode-extension && npm run compile` → 0 errors

# Plan: WorldModel / Self route surface completion

## Finding
`run_with_state` already registers worldmodel/self/live_beliefs/link handlers.
v0.5.0 binary serves them (probed). Mac “missing routes” claim was largely **OpenAPI under-documenting** + empty-body 4xx, not absent handlers.

## Goal
Make intelligence surface **discoverable, contract-aligned, and tested** for extension + SDKs.

## Global constraints
- Surgical: openapi, observe aliases, Python can_execute, SIT smoke — no MCTS rewrite
- Feature-gate: web-server tests only
- Keep existing response field names clients already use
- Branch: `feat/worldmodel-self-routes-openapi` (stacked on health dual-contract)

## Tasks
1. **OpenAPI**: document `/self/health`, `/self/capabilities`, `/self/can-execute` (already partial), `/worldmodel/status|observe|entities|entity|causal*`, `/memory/live_beliefs`, `/memory/link`, `/memory/reflect`, `/memory/hypotheses` — match run_with_state
2. **Observe aliases**: accept `from|state`, `to|next_state` (and keep `action`)
3. **Status map**: fix predict URL in `handle_worldmodel_status` to POST
4. **Python SDK**: `can_execute` → GET `/self/can-execute?operation=`
5. **SIT smoke**: observe → predict → rollout + self/health + can-execute in one test file under tests/integration/

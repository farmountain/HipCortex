## Why

The VS Code extension routes both `search` and `query` chat commands to `handleQueryMemory`, which calls the filter-only `GET /memory/query` endpoint. The semantic `POST /memory/search` (scored, embedding-aware, pinned boost) is never reachable from the chat participant. Additionally, the fully-implemented causal/counterfactual REST API (`/worldmodel/causal/counterfactual`, `/worldmodel/causal/intervention`) has no LM tool registered, making it invisible to Copilot agents.

## What Changes

- **Split `search` vs `query` dispatch**: `search` commands route to a new `handleSearchMemory()` that calls `POST /memory/search` (scored results). `query` commands continue routing to `handleQueryMemory()` (`GET /memory/query`, filter-only).
- **Add `searchMemory()` API method**: New `HipCortexAPI.searchMemory(query, limit)` → `POST /memory/search` returning `{results: [{score, record}], total}`.
- **Register `hipcortex_causal` LM tool**: Exposes `GET /worldmodel/causal` (graph inspection), `POST /worldmodel/causal/counterfactual`, and `POST /worldmodel/causal/intervention` to Copilot via a `mode` parameter (`graph | counterfactual | intervention`).

## Capabilities

### New Capabilities

- `extension-semantic-search`: `@HipCortex search <term>` invokes scored semantic search via `POST /memory/search`, returning results ranked by cosine similarity + confidence + priority.
- `extension-causal-lm-tool`: `hipcortex_causal` LM tool exposes counterfactual and intervention queries over the WorldModel causal graph to Copilot agents.

### Modified Capabilities

- `extension-chat-dispatch`: The dispatch table in `HipCortexChatParticipant` gains a dedicated `search` branch, separating it from the existing `query` branch.

## Impact

- **File**: `vscode-extension/src/extension.ts` only — no Rust changes, no server changes.
- **API surface**: `hipcortex_causal` added to LM tool registry (4 tools → 5 tools).
- **No breaking changes**: `query` behavior unchanged. `search` now reaches the correct endpoint instead of silently falling back to filter-only query.
- **Build**: `npm run compile` in `vscode-extension/`.

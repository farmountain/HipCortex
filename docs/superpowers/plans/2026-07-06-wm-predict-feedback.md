# WM Predict Feedback Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After every auto-captured file save, call `predictState` against the world model and show the top predicted next action and uncertainty (entropy) in the VS Code status bar — closing the observe→learn→predict→confirm feedback loop.

**Architecture:** Single file change in `vscode-extension/src/extension.ts`, inside the `onSave` handler. After the existing `addMemory` + `reflect` calls, call `api.predictState("vscode-user", "edited")`, extract the top-probability next target and Shannon entropy from the response, and update the status bar text. On fresh installs with no WM history, `predictState` returns an error or empty probabilities — both are handled gracefully (no status bar update, no thrown error). Depends on Plan B (`extension-tier-surface`) having run first, but is not blocked by it — the `predictState` API already exists.

**Tech Stack:** TypeScript (VS Code extension), existing `HipCortexAPI.predictState()`, existing status bar item, `npm run compile`, `npm run package`.

## Global Constraints

- `npm run compile` must produce 0 errors (run from `vscode-extension/`)
- The predict call is **best-effort** — wrapped in `.catch(() => null)` to not break auto-capture if WM is cold/locked
- Status bar format: `$(database) HipCortex: WM ${trans} | loops ${loops} | → ${nextFile}? | ${saved} tok`  
  (when prediction available; falls back to existing format when no prediction)
- `predictState` response shape: `{ from_state, action, probabilities: {[target: string]: number}, entropy: number, observation_count: number }`
- Only show prediction when `observation_count >= 3` (too few observations = noisy / not useful)
- Entropy display: show as `H:${entropy.toFixed(1)}` in status bar when observation_count >= 3
- The `updateStatusBar` helper already exists and sets `statusBarItem.text` — the new code updates the same status bar item after predict resolves

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `vscode-extension/src/extension.ts` | `onSave` handler — add `predictState` call + status bar update after `reflect()` |

---

### Task 1: Add WM predict call to onSave handler + status bar feedback

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `onSave` handler (around line 866)

**Interfaces:**
- Consumes:
  - `HipCortexAPI.predictState(state: string, action: string): Promise<any>` — already implemented at line 369; returns `{ probabilities: {[k:string]: number}, entropy: number, observation_count: number }` or throws
  - `statusBarItem.text` — the VS Code status bar item already in scope in `onSave` closure
  - `tokenTracker.getSnapshot().savedTokens` — already in scope
- Produces: Status bar updated with top predicted next file and entropy after each save (when WM has ≥3 observations)

- [ ] **Step 1: Locate the exact insertion point**

Open `vscode-extension/src/extension.ts`. Find the `onSave` handler. The current body after `addMemory` is:

```typescript
            const r = await api.reflect(`edited ${fileName}`).catch(() => ({} as any));
            const trans = 0;
            const loops = r && r.loops_run ? r.loops_run : 0;
            const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();
            const summaryLive = `edited ${fileName}`;
            statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops} | ${saved} tok`;
            serverChannel.appendLine(`Live: ${summaryLive}`);
            updateStatusBar(trans, loops);
```

The predict call goes **after** `reflect()` settles, replacing/augmenting the final status bar update.

- [ ] **Step 2: Add the predict call**

Replace the block from `const r = await api.reflect(...)` through `updateStatusBar(trans, loops);` with:

```typescript
            const r = await api.reflect(`edited ${fileName}`).catch(() => ({} as any));
            const trans = 0;
            const loops = r && r.loops_run ? r.loops_run : 0;
            const saved = tokenTracker.getSnapshot().savedTokens.toLocaleString();

            // WM predict: what file will likely be edited next?
            const pred = await api.predictState('vscode-user', 'edited').catch(() => null);
            let predSuffix = '';
            if (
                pred &&
                pred.probabilities &&
                typeof pred.observation_count === 'number' &&
                pred.observation_count >= 3
            ) {
                // Find highest-probability next target
                const entries = Object.entries(pred.probabilities as Record<string, number>);
                if (entries.length > 0) {
                    entries.sort(([, a], [, b]) => b - a);
                    const [topTarget] = entries[0];
                    // Show just the filename portion (strip path prefix if present)
                    const topFile = topTarget.split(/[\\/]/).pop()?.split(' ')[0] || topTarget;
                    const entropy = typeof pred.entropy === 'number' ? pred.entropy.toFixed(1) : '?';
                    predSuffix = ` | →${topFile}? H:${entropy}`;
                }
            }

            statusBarItem.text = `$(database) HipCortex: WM ${trans} | loops ${loops}${predSuffix} | ${saved} tok`;
            serverChannel.appendLine(`Live: edited ${fileName}${predSuffix}`);
            updateStatusBar(trans, loops);
```

- [ ] **Step 3: Compile**

```
cd vscode-extension && npm run compile
```

Expected: 0 errors.

- [ ] **Step 4: Verify behavior manually**

To verify the status bar updates correctly:

1. Start the HipCortex server: `cargo run --bin webserver --no-default-features --features "web-server,petgraph_backend"`
2. Install the extension: `code --install-extension vscode-extension/hipcortex-memory-0.4.1.vsix`
3. Open a Rust project in VS Code
4. Save 3+ different `.rs` files
5. Save `src/memory_store.rs` again
6. Status bar should update to something like: `$(database) HipCortex: WM 0 | loops 2 | →web_server.rs? H:1.8 | 0 tok`
7. If WM has fewer than 3 observations, status bar shows normal format without `→file? H:X.X`

- [ ] **Step 5: Repackage VSIX**

```
cd vscode-extension && npm run package && npx vsce package --no-dependencies
```

Expected: `hipcortex-memory-0.4.1.vsix` rebuilt.

- [ ] **Step 6: Commit**

```
git add vscode-extension/src/extension.ts vscode-extension/hipcortex-memory-0.4.1.vsix
git commit -m "feat(ext): show WM next-file prediction + entropy in status bar after save"
```

---

## Self-Review

### Spec coverage

| Gap | Task |
|-----|------|
| No automatic WM prediction trigger — always explicit | Task 1 |
| Prediction result never shown in status bar | Task 1 |

### Type consistency
- `api.predictState(state: string, action: string)` — already defined in `HipCortexAPI` at line 369, returns `Promise<any>` — used correctly here.
- `pred.probabilities` accessed as `Record<string, number>` — matches server response `{"probabilities": {"target": 0.6, ...}}` — consistent.
- `pred.observation_count` — matches server response field `"observation_count": usize` — consistent.
- `predSuffix` — string, default `''` — safely interpolated into status bar text — consistent.

### Dependency note
This plan is independent of Plan B (`extension-tier-surface`). Both modify `extension.ts` in different locations. If implemented sequentially, the second implementer reads the updated file first and applies their change. No conflicts.

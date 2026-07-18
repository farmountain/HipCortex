# QuickPick Tags Display (Option A) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the Option A ship gate: tags already written on save/palette (commits `ca1d30a`–`375fb95`) must also **appear** in `hipcortex.queryMemory` QuickPick labels and detail dialog, then repack extension, verify, and push **only** semantic-tags commits — no lifecycle dirt.

**Architecture:** Pure client display layer on top of existing write path. Extract two pure helpers (`formatMemoryQuickPickItem`, `formatMemoryDetailMessage`) from `MemoryRecord` so Jest can assert label/detail without VS Code UI. Wire helpers inside `queryMemoryCommand` only. Explicitly **exclude** unstaged `killProcessOnPort` / version-gate lifecycle work (separate archived change). Bump **0.5.2 → 0.5.3** so the Marketplace/VSIX identity matches “write + display complete.”

**Tech Stack:** TypeScript, VS Code Extension API (`showQuickPick`, `showInformationMessage`), Jest (`vscode-extension/src/test/extension.test.ts`), webpack/`@vscode/vsce`.

## Global Constraints

- **Option A scope only:** Bucket B = QuickPick + detail tags UI. Do **not** commit Bucket C (`killProcessOnPort`, health version mismatch kill) or non-extension dirt.
- Working tree is dirty: use **surgical staging** (checkout/patch/commit only queryMemory + helpers + tests). Never `git add -A`.
- Target files: `vscode-extension/src/extension.ts`, `vscode-extension/src/test/extension.test.ts`, `vscode-extension/package.json`, new VSIX.
- `MemoryRecord.tags?: string[]` already on HEAD (extension.ts ~L89). No backend changes.
- Tests: `cd vscode-extension` then `npx jest --forceExit --testPathPattern=extension.test` — all prior tests must stay green (expect ≥43 before this plan; +N new).
- Label format (exact): `` `${actor} → ${action}${tagsStr}` `` where `tagsStr` is ` [tag1, tag2]` or `""`.
- Detail must include line: `• Keywords/Tags: ${tags joined or 'None'}`.
- Version: **0.5.3** (0.5.2 VSIX already built without display; do not overwrite meaning of 0.5.2).
- Push: `git push origin main` only after write+UX smoke evidence. Do not push unstaged files.
- GitNexus index may be stale; re-run `npx gitnexus analyze` **after** push (optional follow-up, not blocking).

## Preconditions (already true — do not re-implement)

| Item | Evidence |
|------|----------|
| `extractSemanticTags` + helpers | commit `ca1d30a` + fix `800f87e` |
| onSave `tags: autoTags` | commit `b6efc27` |
| palette merge `mergedTags` | commit `30432fa` |
| package 0.5.2 + VSIX with write path | commit `375fb95` (VSIX has `tags:o`, lacks QuickPick strings) |
| Server stores `tags: Vec<String>` | `src/web_server.rs` AddMemoryRequest / query responses |

## File Map

| File | Action | What changes |
|------|--------|--------------|
| `vscode-extension/src/extension.ts` | Modify | Add pure format helpers (module-level, exported); rewrite `queryMemoryCommand` body (~L1314–1345 on HEAD) |
| `vscode-extension/src/test/extension.test.ts` | Modify | Import helpers; add `describe('memory display formatters')` |
| `vscode-extension/package.json` | Modify | `"version": "0.5.3"` |
| `vscode-extension/hipcortex-memory-0.5.3.vsix` | Create | Packaged bundle with display + write |
| `docs/superpowers/plans/2026-07-17-semantic-tag-auto-capture.md` | Optional note | Not required; this plan supersedes Task 4 UX smoke for Option A |

**Out of scope (leave unstaged):** `killProcessOnPort`, version-mismatch kill in `doAutoStartServer`, unrelated openspec/sdk/rust dirt, old VSIX binaries, agent tooling dirs.

---

## Task 1: Pure display formatters + unit tests

**Files:**
- Modify: `vscode-extension/src/extension.ts` (insert **before** `export function activate`, near other exports such as `extractSemanticTags`)
- Test: `vscode-extension/src/test/extension.test.ts`

**Interfaces:**
- Produces:
  - `formatMemoryQuickPickItem(record: MemoryRecord): { label: string; description: string; detail: string; record: MemoryRecord }`
  - `formatMemoryDetailMessage(record: MemoryRecord): string`
- Consumes: `MemoryRecord` (`tags?: string[]`, `priority?: string`, `source?: string`, `metadata`, etc.)

---

- [ ] **Step 1: Add imports in test file**

In `vscode-extension/src/test/extension.test.ts`, extend the existing import from `../extension` (same block that already imports `extractSemanticTags`):

```typescript
import {
    HipCortexAPI,
    buildPublishedBinaryName,
    HARNESS_TOOL_DESCRIPTIONS,
    isValidServerBinary,
    activate,
    AddMemoryRequest,
    extractSemanticTags,
    formatMemoryQuickPickItem,
    formatMemoryDetailMessage,
} from '../extension';
```

- [ ] **Step 2: Write failing tests**

Append before the final closing of the file (sibling of `describe('extractSemanticTags')`):

```typescript
describe('memory display formatters', () => {
    const baseRecord = {
        id: 'rec-1',
        record_type: 'Symbolic',
        timestamp: '2026-07-17T00:00:00.000Z',
        actor: 'vscode-user',
        action: 'edited',
        target: 'extension.ts (typescript, 100 lines)',
        metadata: { workspace: 'HipCortex', source: 'vscode-auto-capture' },
        priority: 'normal',
        source: 'vscode-auto-capture',
        tags: ['typescript', 'ts', 'auto-capture', 'core'],
    } as any;

    test('QuickPick label includes bracketed tags', () => {
        const item = formatMemoryQuickPickItem(baseRecord);
        expect(item.label).toBe(
            'vscode-user → edited [typescript, ts, auto-capture, core]'
        );
        expect(item.description).toBe(baseRecord.target);
        expect(item.record).toBe(baseRecord);
    });

    test('QuickPick label omits brackets when tags empty', () => {
        const item = formatMemoryQuickPickItem({ ...baseRecord, tags: [] });
        expect(item.label).toBe('vscode-user → edited');
        expect(item.label).not.toContain('[');
    });

    test('QuickPick label omits brackets when tags undefined', () => {
        const { tags, ...rest } = baseRecord;
        const item = formatMemoryQuickPickItem(rest as any);
        expect(item.label).toBe('vscode-user → edited');
    });

    test('QuickPick detail includes timestamp and workspace snippet', () => {
        const item = formatMemoryQuickPickItem(baseRecord);
        expect(item.detail).toContain('workspace: HipCortex');
        // timestamp locale-dependent — just require non-empty detail
        expect(item.detail.length).toBeGreaterThan(0);
    });

    test('detail message lists Keywords/Tags', () => {
        const msg = formatMemoryDetailMessage(baseRecord);
        expect(msg).toContain('Keywords/Tags: typescript, ts, auto-capture, core');
        expect(msg).toContain('ID: rec-1');
        expect(msg).toContain('Actor: vscode-user');
    });

    test('detail message shows None when no tags', () => {
        const msg = formatMemoryDetailMessage({ ...baseRecord, tags: undefined });
        expect(msg).toContain('Keywords/Tags: None');
    });
});
```

- [ ] **Step 3: Run tests — expect fail**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPattern=extension.test
```

Expected: FAIL import/export — `formatMemoryQuickPickItem` / `formatMemoryDetailMessage` not exported.

- [ ] **Step 4: Implement helpers in `extension.ts`**

Insert at **module level** immediately **after** `extractSemanticTags` (and its helpers), **before** `export function activate`:

```typescript
/**
 * Builds a QuickPick list item for hipcortex.queryMemory.
 * Tags appear in the label so users can scan without opening the detail dialog.
 */
export function formatMemoryQuickPickItem(record: MemoryRecord): {
    label: string;
    description: string;
    detail: string;
    record: MemoryRecord;
} {
    const tagsStr =
        record.tags && record.tags.length > 0
            ? ` [${record.tags.join(', ')}]`
            : '';

    const detailParts: string[] = [];
    if (record.source) {
        detailParts.push(`source: ${record.source}`);
    }
    if (record.priority && record.priority !== 'normal') {
        detailParts.push(`priority: ${record.priority}`);
    }
    if (record.metadata) {
        if (typeof record.metadata.user_message === 'string') {
            const um = record.metadata.user_message as string;
            const snippet = um.length > 60 ? um.substring(0, 57) + '...' : um;
            detailParts.push(`"${snippet}"`);
        } else if (record.metadata.workspace) {
            detailParts.push(`workspace: ${record.metadata.workspace}`);
        } else if (record.metadata.tokens !== undefined) {
            detailParts.push(`tokens: ${record.metadata.tokens}`);
        }
    }

    const timeStr = new Date(record.timestamp).toLocaleString();
    const detailLine =
        detailParts.length > 0
            ? `${timeStr} • ${detailParts.join(' • ')}`
            : timeStr;

    return {
        label: `${record.actor} → ${record.action}${tagsStr}`,
        description: record.target,
        detail: detailLine,
        record,
    };
}

/** Human-readable multi-line detail body for the selected QuickPick record. */
export function formatMemoryDetailMessage(record: MemoryRecord): string {
    const tagsStr =
        record.tags && record.tags.length > 0
            ? record.tags.join(', ')
            : 'None';
    return (
        `Memory Record Details:\n` +
        `• ID: ${record.id}\n` +
        `• Actor: ${record.actor}\n` +
        `• Action: ${record.action}\n` +
        `• Target: ${record.target}\n` +
        `• Keywords/Tags: ${tagsStr}\n` +
        `• Priority: ${record.priority || 'normal'}\n` +
        `• Source: ${record.source || 'unknown'}\n` +
        `• Timestamp: ${new Date(record.timestamp).toLocaleString()}\n` +
        `• Metadata: ${JSON.stringify(record.metadata, null, 2)}`
    );
}
```

If `MemoryRecord` is not exported and tests need only structural typing, either export the interface or keep `as any` fixtures (tests above already use `as any`). Prefer **exporting** `MemoryRecord` only if compile fails without it — do not change unrelated interfaces.

- [ ] **Step 5: Run tests — expect pass**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPattern=extension.test
```

Expected: all prior tests + 6 new formatter tests PASS.

- [ ] **Step 6: Commit**

```powershell
git add vscode-extension/src/extension.ts vscode-extension/src/test/extension.test.ts
git commit -m "feat(extension): pure formatters for QuickPick tags display"
```

Verify staged diff has **no** `killProcessOnPort`. If it does, unstage and re-apply only helper hunks.

---

## Task 2: Wire formatters into `queryMemoryCommand`

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `queryMemoryCommand` (HEAD ~L1314–1345; line numbers drift after Task 1)

**Interfaces:**
- Consumes: `formatMemoryQuickPickItem`, `formatMemoryDetailMessage` from Task 1

---

- [ ] **Step 1: Replace `queryMemoryCommand` body**

Find:

```typescript
    const queryMemoryCommand = vscode.commands.registerCommand('hipcortex.queryMemory', async () => {
        try {
            const api = new HipCortexAPI();
            const response = await api.queryMemory({ limit: 50 });
            
            const items = response.records.map(record => ({
                label: `${record.actor} → ${record.action}`,
                description: record.target,
                detail: new Date(record.timestamp).toLocaleString(),
                record
            }));
            
            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select a memory record to view details'
            });
            
            if (selected) {
                const record = selected.record;
                const message = `**Memory Record**\\n\\n` +
                    `ID: ${record.id}\\n` +
                    `Actor: ${record.actor}\\n` +
                    `Action: ${record.action}\\n` +
                    `Target: ${record.target}\\n` +
                    `Timestamp: ${new Date(record.timestamp).toLocaleString()}\\n` +
                    `Metadata: ${JSON.stringify(record.metadata, null, 2)}`;
                
                vscode.window.showInformationMessage(message);
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
        }
    });
```

Replace with:

```typescript
    const queryMemoryCommand = vscode.commands.registerCommand('hipcortex.queryMemory', async () => {
        try {
            const api = new HipCortexAPI();
            const response = await api.queryMemory({ limit: 50 });

            const items = response.records.map(record => formatMemoryQuickPickItem(record));

            const selected = await vscode.window.showQuickPick(items, {
                placeHolder: 'Select a memory record to view details'
            });

            if (selected) {
                vscode.window.showInformationMessage(
                    formatMemoryDetailMessage(selected.record)
                );
            }
        } catch (error) {
            vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
        }
    });
```

**Do not** paste lifecycle hunks from the working tree while editing this block.

- [ ] **Step 2: Run unit tests**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPattern=extension.test
```

Expected: all PASS (formatters + existing 43).

- [ ] **Step 3: Commit**

```powershell
git add vscode-extension/src/extension.ts
git commit -m "feat(extension): show semantic tags in queryMemory QuickPick"
```

---

## Task 3: Bump 0.5.3, package VSIX, bundle proof

**Files:**
- Modify: `vscode-extension/package.json` line `"version"`
- Create: `vscode-extension/hipcortex-memory-0.5.3.vsix`

---

- [ ] **Step 1: Bump version**

In `vscode-extension/package.json`:

```json
    "version": "0.5.3",
```

- [ ] **Step 2: Package**

```powershell
cd vscode-extension
npx jest --forceExit --testPathPattern=extension.test
npx @vscode/vsce package --no-dependencies --out hipcortex-memory-0.5.3.vsix
```

If `@vscode/vsce` missing: `npx vsce package --no-dependencies --out hipcortex-memory-0.5.3.vsix`.

Expected: `DONE  Packaged: hipcortex-memory-0.5.3.vsix`

- [ ] **Step 3: Bundle string proof (write + display)**

```powershell
cd vscode-extension
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead("$pwd\hipcortex-memory-0.5.3.vsix")
$entry = $zip.GetEntry("extension/dist/extension.js")
$reader = [System.IO.StreamReader]::new($entry.Open())
$js = $reader.ReadToEnd(); $reader.Close(); $zip.Dispose()
@('auto-capture','Keywords/Tags','tags:') | ForEach-Object {
  if ($js.Contains($_)) { "PASS: $_" } else { "FAIL: $_" }
}
```

Expected:

```
PASS: auto-capture
PASS: Keywords/Tags
PASS: tags:
```

All three must PASS before commit.

- [ ] **Step 4: Commit**

```powershell
git add vscode-extension/package.json vscode-extension/hipcortex-memory-0.5.3.vsix
git commit -m "chore: bump extension to v0.5.3 — QuickPick semantic tags display"
```

---

## Task 4: UX + write smoke, then push

**Files:** none (ops). Artifact: optional note in `.superpowers/sdd/progress.md` (local only).

---

- [ ] **Step 1: Install VSIX**

1. VS Code: `Extensions: Install from VSIX…` → `vscode-extension/hipcortex-memory-0.5.3.vsix`
2. `Developer: Reload Window`
3. Confirm extension version **0.5.3** in Extensions view

- [ ] **Step 2: Write smoke (server must be up)**

1. Ensure HipCortex server reachable (auto-start or manual on port from settings, default 3030)
2. Open any workspace file under `src/` or `vscode-extension/src/` (e.g. `extension.ts`)
3. Make a trivial edit, **Save**
4. Output channel **HipCortex Server** should log auto-capture / Live edited line (non-fatal if only API success)

- [ ] **Step 3: UX smoke (Option A gate)**

1. Command Palette → **HipCortex: Query Memory Records** (`hipcortex.queryMemory`)
2. Find a recent `vscode-user → edited` item
3. **PASS criteria (all required):**
   - Label contains `[` and at least one of: `auto-capture`, language id (e.g. `typescript`), or path hint (`core`, `sdk`, …)
   - Select item → detail dialog contains `Keywords/Tags:` with non-`None` value for that auto-captured record

If label has no brackets: fail — do not push; check server returns `tags` on query (curl `GET /memory/query?limit=5` or equivalent) and that extension 0.5.3 is the active one.

- [ ] **Step 4: Pre-push hygiene**

```powershell
git status -sb
git log origin/main..HEAD --oneline
```

Expected: branch ahead by prior 5 + new Option A commits (typically +3).  
Working tree may still show dirty lifecycle/other files — **leave them**.

Confirm last commits touch only:

- `vscode-extension/src/extension.ts`
- `vscode-extension/src/test/extension.test.ts`
- `vscode-extension/package.json`
- `vscode-extension/hipcortex-memory-0.5.3.vsix`

- [ ] **Step 5: Push**

```powershell
git push origin main
```

Expected: success; remote has 0.5.3 + QuickPick tags.

- [ ] **Step 6: Optional post-push**

```powershell
npx gitnexus analyze
```

Not a ship blocker.

---

## Self-Review

**Option A coverage:**
- ✅ QuickPick label shows tags (Task 1–2)
- ✅ Detail dialog Keywords/Tags (Task 1–2)
- ✅ Unit tests for empty/undefined/populated tags (Task 1)
- ✅ VSIX 0.5.3 proves `auto-capture` + `Keywords/Tags` + `tags:` (Task 3)
- ✅ Write path already shipped (precondition); UX smoke validates end-to-end (Task 4)
- ✅ Push only after smoke; no lifecycle / non-ext dirt (Global Constraints + Task 4)

**Explicit non-goals:**
- ❌ `killProcessOnPort` / version-gate auto-start
- ❌ Marketplace publish (optional human step after push)
- ❌ Backend / SDK changes

**Type consistency:**
- `MemoryRecord.tags?: string[]` — HEAD already
- Helpers return plain objects + string; `queryMemoryCommand` maps `response.records` through `formatMemoryQuickPickItem`

**No placeholders:** full code and commands above.

**Risk notes for implementer:**
- Dirty WT has a full mixed diff — **copy helper/command from this plan**, do not `git checkout` working tree wholesale into the commit.
- Unicode arrow `→` must match tests exactly.
- If `MemoryRecord` is file-private, export it or use structural typing in helpers without exporting.

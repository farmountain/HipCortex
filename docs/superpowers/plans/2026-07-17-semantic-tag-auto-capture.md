# Semantic Tag Auto-Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the VS Code extension so every automatically captured memory record (file save + command palette add) includes meaningful semantic tags — making the `hipcortex.queryMemory` QuickPick display rich, filterable labels instead of blank suffixes.

**Architecture:** A pure client-side fix in `vscode-extension/src/extension.ts`. A new exported function `extractSemanticTags(doc, timeout?)` extracts tags from 5 layers (structural, path, git, symbols, diagnostics) with async graceful degradation. Called inside `onDidSaveTextDocument` and the `hipcortex.addMemory` command, adding a `tags` field to every `addMemory()` call. No backend changes — the server already stores `tags: Vec<String>` and the QuickPick display already renders them (confirmed in v0.5.1 bundle).

**Tech Stack:** TypeScript, VS Code Extension API (`vscode.extensions`, `vscode.languages.getDiagnostics`, `vscode.commands.executeCommand`), Jest (existing harness at `vscode-extension/src/test/extension.test.ts`), webpack 5 (existing), `path` (already imported).

## Global Constraints

- Target file: `vscode-extension/src/extension.ts` (1339 lines, 2026-07-17). Do NOT restructure — follow existing patterns.
- Test runner: `npm run test` (Jest). Run from `vscode-extension/` directory.
- Build: `npm run package` (webpack). Must produce zero errors.
- Tag format: lowercase, no spaces (use hyphens), max 8 tags per record.
- Async timeout: 200 ms budget for git + symbol extraction. Gracefully skip on timeout/failure.
- `extractSemanticTags` must be exported for direct test imports.
- All existing 33 tests must continue to pass.

---

## File Map

| File | Action | What changes |
|------|--------|--------------|
| `vscode-extension/src/extension.ts` | Modify | Add `extractSemanticTags()`, `getGitBranch()`, `getDocumentSymbolTags()`, `withTimeout()` helper; patch `onDidSaveTextDocument` (L1140); patch `hipcortex.addMemory` command (L1221) |
| `vscode-extension/src/test/extension.test.ts` | Modify | Add `extractSemanticTags` test suite (10 new tests) |
| `vscode-extension/package.json` | Modify | Version bump 0.5.1 → 0.5.2 |
| `vscode-extension/hipcortex-memory-0.5.2.vsix` | Create | Rebuilt VSIX package |

No new files beyond the VSIX. No backend changes.

---

## Task 1: Add `extractSemanticTags` function to extension.ts

**Files:**
- Modify: `vscode-extension/src/extension.ts` — insert before line 1129
- Test: `vscode-extension/src/test/extension.test.ts`

**Interfaces:**
- Produces: `extractSemanticTags(doc: vscode.TextDocument, timeoutMs?: number): Promise<string[]>`
  - `doc` — TextDocument being saved
  - `timeoutMs` — default 200ms. Pass 0 to skip async layers.
  - Returns: deduplicated lowercase string[], max 8 elements

---

- [ ] **Step 1: Add `extractSemanticTags` to imports in test file**

In `vscode-extension/src/test/extension.test.ts`, update line 6 import:

```typescript
import {
    HipCortexAPI,
    buildPublishedBinaryName,
    HARNESS_TOOL_DESCRIPTIONS,
    isValidServerBinary,
    activate,
    AddMemoryRequest,
    extractSemanticTags,
} from '../extension';
```

- [ ] **Step 2: Write the failing tests**

Add this new `describe` block at the bottom of `describe('HipCortex Extension Unit Tests', ...)`, before its closing `});`:

```typescript
describe('extractSemanticTags', () => {
    function makeDoc(overrides: Partial<{
        languageId: string;
        fileName: string;
        lineCount: number;
    }> = {}): vscode.TextDocument {
        return {
            languageId: overrides.languageId ?? 'typescript',
            fileName: overrides.fileName ?? '/home/user/project/src/extension.ts',
            lineCount: overrides.lineCount ?? 100,
            uri: { scheme: 'file', fsPath: overrides.fileName ?? '/home/user/project/src/extension.ts' } as any,
            getText: () => '',
        } as any;
    }

    test('always includes languageId and auto-capture tag', async () => {
        const doc = makeDoc({ languageId: 'typescript', fileName: '/project/src/foo.ts' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('typescript');
        expect(tags).toContain('auto-capture');
    });

    test('extracts file extension as tag', async () => {
        const doc = makeDoc({ fileName: '/project/src/server.rs', languageId: 'rust' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('rs');
    });

    test('detects test file from path', async () => {
        const doc = makeDoc({ fileName: '/project/tests/unit/mod.rs', languageId: 'rust' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('testing');
    });

    test('detects api path segment', async () => {
        const doc = makeDoc({ fileName: '/project/src/api/routes.ts', languageId: 'typescript' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('api');
    });

    test('detects sdk path segment', async () => {
        const doc = makeDoc({ fileName: '/project/sdk/python/client.py', languageId: 'python' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('sdk');
    });

    test('caps tags at 8', async () => {
        const doc = makeDoc({
            fileName: '/project/sdk/api/tests/spec/frontend/backend/database/auth.ts',
            languageId: 'typescript'
        });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags.length).toBeLessThanOrEqual(8);
    });

    test('returns no duplicates', async () => {
        const doc = makeDoc({ fileName: '/project/test/test_api.ts', languageId: 'typescript' });
        const tags = await extractSemanticTags(doc, 0);
        const unique = [...new Set(tags)];
        expect(tags.length).toBe(unique.length);
    });

    test('all tags are lowercase', async () => {
        const doc = makeDoc({ fileName: '/project/SDK/API/Handler.ts', languageId: 'typescript' });
        const tags = await extractSemanticTags(doc, 0);
        tags.forEach(t => expect(t).toBe(t.toLowerCase()));
    });

    test('docs path segment maps to documentation tag', async () => {
        const doc = makeDoc({ fileName: '/project/docs/architecture.md', languageId: 'markdown' });
        const tags = await extractSemanticTags(doc, 0);
        expect(tags).toContain('documentation');
    });

    test('tags from command palette parse correctly', () => {
        const rawInput = 'typescript, auth, refactor';
        const tags = rawInput.split(',').map(t => t.trim().toLowerCase()).filter(t => t.length > 0);
        expect(tags).toEqual(['typescript', 'auth', 'refactor']);
        const request: AddMemoryRequest = { actor: 'User', action: 'reviewed', target: 'auth.ts', tags };
        expect(request.tags).toHaveLength(3);
    });
});
```

- [ ] **Step 3: Run tests to confirm they fail**

```bash
cd vscode-extension
npm run test
```

Expected: Import error — `extractSemanticTags is not exported from '../extension'`. Existing 33 tests must still PASS.

- [ ] **Step 4: Implement the functions in `extension.ts`**

Insert immediately **before** line 1129 (`const onSave = vscode.workspace.onDidSaveTextDocument`) — i.e. inside `activate()` scope, or just above it if it's at module level. Check context: if `onSave` is declared inside `export function activate(context)`, insert before the `const onSave` line within that function body.

```typescript
/** Races a promise against a timeout. Returns null if timed out or errored. */
async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T | null> {
    if (ms <= 0) { return null; }
    return Promise.race([
        promise.catch(() => null as T | null),
        new Promise<null>(resolve => setTimeout(() => resolve(null), ms))
    ]);
}

/**
 * Extracts semantic tags from a TextDocument for memory auto-capture.
 *
 * Layer 1 (sync): languageId, file extension, 'auto-capture'
 * Layer 2 (sync): path-segment semantic hints (testing, api, sdk, docs, ...)
 * Layer 3 (async, capped): git branch name slug
 * Layer 4 (async, capped): document symbol categorization
 * Layer 5 (sync): error diagnostics → 'has-errors'
 *
 * @param doc       The TextDocument being processed.
 * @param timeoutMs Budget in ms for async layers (default 200). Pass 0 to skip async.
 * @returns         Deduplicated lowercase string[], max 8 elements.
 */
export async function extractSemanticTags(
    doc: vscode.TextDocument,
    timeoutMs: number = 200
): Promise<string[]> {
    const tags: string[] = [];

    // Layer 1: Structural (always present, sync)
    const ext = path.extname(doc.fileName).replace(/^\./, '').toLowerCase();
    tags.push(doc.languageId.toLowerCase());
    if (ext && ext !== doc.languageId.toLowerCase()) {
        tags.push(ext);
    }
    tags.push('auto-capture');

    // Layer 2: Path-segment semantic hints (sync)
    const lowerPath = doc.fileName.replace(/\\/g, '/').toLowerCase();
    const segs = lowerPath.split('/');
    const pathHints: [string[], string][] = [
        [['test', 'tests', 'spec', '__test__', '__tests__', '.test.', '.spec.'], 'testing'],
        [['api', 'routes', 'handlers', 'endpoints'],                            'api'],
        [['sdk'],                                                                'sdk'],
        [['docs', 'documentation'],                                             'documentation'],
        [['migrations', 'migration'],                                           'database'],
        [['components', 'views', 'pages', 'frontend'],                         'frontend'],
        [['modules', 'module'],                                                 'module'],
        [['src', 'lib', 'core'],                                                'core'],
        [['scripts', 'tools'],                                                  'tooling'],
    ];
    for (const [patterns, tag] of pathHints) {
        if (patterns.some(p => segs.some(s => s.includes(p)) || lowerPath.includes(p))) {
            tags.push(tag);
        }
    }

    // Layers 3+4: Async (git branch + document symbols) within budget
    const [branchResult, symbolsResult] = await Promise.all([
        withTimeout(getGitBranch(doc), timeoutMs),
        withTimeout(getDocumentSymbolTags(doc), timeoutMs),
    ]);
    if (branchResult) { tags.push(branchResult); }
    if (symbolsResult && symbolsResult.length > 0) { tags.push(...symbolsResult); }

    // Layer 5: Error diagnostics (sync)
    try {
        const diags = vscode.languages.getDiagnostics(doc.uri);
        if (diags.some(d => d.severity === vscode.DiagnosticSeverity.Error)) {
            tags.push('has-errors');
        }
    } catch { /* skip in test environments */ }

    // Deduplicate, lowercase, cap at 8
    const seen = new Set<string>();
    return tags
        .map(t => t.toLowerCase().replace(/\s+/g, '-'))
        .filter(t => t.length > 0 && !seen.has(t) && seen.add(t))
        .slice(0, 8);
}

/** Gets the current git branch slug for a document's repository. Returns null on any failure. */
async function getGitBranch(doc: vscode.TextDocument): Promise<string | null> {
    try {
        const gitExt = vscode.extensions.getExtension('vscode.git');
        if (!gitExt) { return null; }
        const git = gitExt.isActive ? gitExt.exports : await gitExt.activate();
        const api = git.getAPI(1);
        if (!api?.repositories?.length) { return null; }
        const repo = api.repositories.find((r: any) =>
            doc.uri.fsPath.startsWith(r.rootUri.fsPath)
        ) ?? api.repositories[0];
        const branch: string | undefined = repo?.state?.HEAD?.name;
        if (!branch) { return null; }
        const stripped = branch.replace(/^(feature|fix|chore|bugfix|hotfix)\//, '');
        return stripped.replace(/[^a-z0-9-]/gi, '-').toLowerCase().slice(0, 30) || null;
    } catch {
        return null;
    }
}

/** Extracts semantic category tags from document symbols. Returns at most 2 tags. */
async function getDocumentSymbolTags(doc: vscode.TextDocument): Promise<string[]> {
    try {
        const symbols: vscode.DocumentSymbol[] | undefined =
            await vscode.commands.executeCommand('vscode.executeDocumentSymbolProvider', doc.uri);
        if (!symbols?.length) { return []; }
        const names = symbols.map(s => s.name.toLowerCase());
        const result: string[] = [];
        if (names.some(n => n.startsWith('test_') || n.startsWith('it_') || n === 'test')) {
            result.push('testing');
        }
        if (names.some(n => n.includes('handle_') || n.includes('handler') || n.includes('route'))) {
            result.push('api');
        }
        return [...new Set(result)].slice(0, 2);
    } catch {
        return [];
    }
}
```

- [ ] **Step 5: Run tests to verify all pass**

```bash
cd vscode-extension
npm run test
```

Expected: 10 new tests PASS. All 33 existing tests PASS. Total: 43 passing.

- [ ] **Step 6: Commit**

```bash
git add vscode-extension/src/extension.ts vscode-extension/src/test/extension.test.ts
git commit -m "feat(extension): add extractSemanticTags() with 5-layer context extraction"
```

---

## Task 2: Wire `extractSemanticTags` into `onDidSaveTextDocument`

**Files:**
- Modify: `vscode-extension/src/extension.ts:1140-1153`

**Interfaces:**
- Consumes: `extractSemanticTags(doc, timeoutMs)` → `Promise<string[]>` from Task 1

---

- [ ] **Step 1: Patch `onDidSaveTextDocument` handler**

Replace lines 1140-1153 in `extension.ts`:

**BEFORE:**
```typescript
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Symbolic',
                priority: 'normal',
                source: 'vscode-auto-capture',
                metadata: { 
                    source: 'vscode-auto-capture', 
                    language_id: doc.languageId, 
                    line_count: doc.lineCount,
                    workspace: vscode.workspace.name || 'unknown'
                }
            });
```

**AFTER:**
```typescript
            const autoTags = await extractSemanticTags(doc, 200);
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Symbolic',
                priority: 'normal',
                source: 'vscode-auto-capture',
                tags: autoTags,
                metadata: { 
                    source: 'vscode-auto-capture', 
                    language_id: doc.languageId, 
                    line_count: doc.lineCount,
                    workspace: vscode.workspace.name || 'unknown'
                }
            });
```

- [ ] **Step 2: Run all tests**

```bash
cd vscode-extension
npm run test
```

Expected: 43 tests PASS. Zero failures.

- [ ] **Step 3: Commit**

```bash
git add vscode-extension/src/extension.ts
git commit -m "feat(extension): wire semantic tags into onDidSave auto-capture"
```

---

## Task 3: Patch `hipcortex.addMemory` command to include tags

**Files:**
- Modify: `vscode-extension/src/extension.ts:1221-1239`

**Interfaces:**
- Consumes: `extractSemanticTags(doc, timeoutMs)` from Task 1

---

- [ ] **Step 1: Replace `addMemoryCommand` handler (lines 1221-1239)**

**BEFORE:**
```typescript
    const addMemoryCommand = vscode.commands.registerCommand('hipcortex.addMemory', async () => {
        const actor = await vscode.window.showInputBox({ prompt: 'Enter actor name' });
        const action = await vscode.window.showInputBox({ prompt: 'Enter action' });
        const target = await vscode.window.showInputBox({ prompt: 'Enter target' });
        
        if (actor && action && target) {
            try {
                const api = new HipCortexAPI();
                const response = await api.addMemory({ actor, action, target });
                if (response.success) {
                    vscode.window.showInformationMessage(`Memory added! ID: ${response.record_id}`);
                } else {
                    vscode.window.showErrorMessage(`Failed to add memory: ${response.error}`);
                }
            } catch (error) {
                vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
            }
        }
    });
```

**AFTER:**
```typescript
    const addMemoryCommand = vscode.commands.registerCommand('hipcortex.addMemory', async () => {
        const actor = await vscode.window.showInputBox({ prompt: 'Enter actor name' });
        const action = await vscode.window.showInputBox({ prompt: 'Enter action' });
        const target = await vscode.window.showInputBox({ prompt: 'Enter target' });
        const tagsRaw = await vscode.window.showInputBox({
            prompt: 'Enter tags (comma-separated, optional)',
            placeHolder: 'e.g. auth, refactor, typescript'
        });
        
        if (actor && action && target) {
            try {
                const api = new HipCortexAPI();
                const activeDoc = vscode.window.activeTextEditor?.document;
                const autoTags = activeDoc ? await extractSemanticTags(activeDoc, 200) : [];
                const userTags = tagsRaw
                    ? tagsRaw.split(',').map(t => t.trim().toLowerCase()).filter(t => t.length > 0)
                    : [];
                const mergedTags = [...new Set([...autoTags, ...userTags])].slice(0, 8);

                const response = await api.addMemory({ actor, action, target, tags: mergedTags });
                if (response.success) {
                    vscode.window.showInformationMessage(`Memory added! ID: ${response.record_id}`);
                } else {
                    vscode.window.showErrorMessage(`Failed to add memory: ${response.error}`);
                }
            } catch (error) {
                vscode.window.showErrorMessage(`Error: ${error instanceof Error ? error.message : String(error)}`);
            }
        }
    });
```

- [ ] **Step 2: Run all tests**

```bash
cd vscode-extension
npm run test
```

Expected: 43 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add vscode-extension/src/extension.ts
git commit -m "feat(extension): tags input in addMemory command palette + auto-semantic merge"
```

---

## Task 4: Rebuild VSIX v0.5.2 + smoke-test

**Files:**
- Modify: `vscode-extension/package.json` line 5
- Create: `vscode-extension/hipcortex-memory-0.5.2.vsix`

---

- [ ] **Step 1: Bump version**

Edit `vscode-extension/package.json` line 5:
```json
    "version": "0.5.2",
```

- [ ] **Step 2: Package**

```bash
cd vscode-extension
npx vsce package --no-dependencies --out hipcortex-memory-0.5.2.vsix
```

Expected: `DONE  Packaged: hipcortex-memory-0.5.2.vsix (N files, X.X MB)`

- [ ] **Step 3: Verify `auto-capture` literal is in bundle**

```powershell
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead("$pwd\hipcortex-memory-0.5.2.vsix")
$entry = $zip.GetEntry("extension/dist/extension.js")
$reader = [System.IO.StreamReader]::new($entry.Open())
$js = $reader.ReadToEnd(); $reader.Close(); $zip.Dispose()
if ($js -match "auto-capture") { "PASS: auto-capture string found" } else { "FAIL" }
```

Expected: `PASS: auto-capture string found`

- [ ] **Step 4: Install and smoke-test**

1. `Ctrl+Shift+P` → `Extensions: Install from VSIX` → select `hipcortex-memory-0.5.2.vsix`
2. `Developer: Reload Window`
3. Save any file in the workspace (e.g. touch `extension.ts` with a space and save)
4. `Ctrl+Shift+P` → `HipCortex: Query Memory Records`

Expected QuickPick item: `vscode-user → edited [typescript, ts, auto-capture, core, sdk]`
If tags appear after `→ edited`, the fix is working end-to-end.

- [ ] **Step 5: Commit and push**

```bash
git add vscode-extension/package.json vscode-extension/hipcortex-memory-0.5.2.vsix
git commit -m "chore: bump extension to v0.5.2 — semantic tag auto-capture"
git push origin main
```

---

## Self-Review

**Spec coverage:**
- ✅ Layer 1 — languageId, file extension, auto-capture tag
- ✅ Layer 2 — path segment hints (9 categories)
- ✅ Layer 3 — git branch slug
- ✅ Layer 4 — document symbol categorization
- ✅ Layer 5 — error diagnostics → has-errors
- ✅ 200ms async timeout with graceful degradation
- ✅ Max 8 tags, deduplicated, lowercase, hyphens-only
- ✅ `onDidSaveTextDocument` wired with `tags: autoTags`
- ✅ `hipcortex.addMemory` command wired with merged auto+user tags
- ✅ VSIX v0.5.2 rebuilt and verified
- ✅ 10 new tests; all 33 existing tests preserved

**Type consistency:**
- `extractSemanticTags` → `Promise<string[]>` throughout Tasks 1–3
- `AddMemoryRequest.tags?: string[]` already defined (extension.ts L148) — no interface change needed
- `withTimeout<T>` → `Promise<T | null>` used consistently for both async helpers

**No placeholders:** All code is complete and copy-pasteable.

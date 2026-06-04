# VS Code Extension Enhancement Plan — LM Tool + Auto-Capture + Token Savings

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the HipCortex VS Code extension (v0.1.5) with three features: (1) register as an LM Tool so Copilot can call `hipcortex_search` automatically, (2) auto-capture file saves to HipCortex memory, (3) display token savings in chat responses and status bar.

**Architecture:** Additive changes to `vscode-extension/src/extension.ts`. One new file: `token-tracker.ts` for session savings state. Bump VS Code engine version to 1.90.0 for `registerTool` API support.

**Tech Stack:** TypeScript, VS Code Extension API (1.90+), `vscode.lm.registerTool`, `vscode.workspace.onDidSaveTextDocument`, `vscode.window.createStatusBarItem`, `axios` (already installed).

**Spec:** `docs/superpowers/specs/2026-06-05-vscode-extension-enhancement-design.md`
**Worktree:** `D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension`

---

## File Map

| File | Action |
|------|--------|
| `vscode-extension/package.json` | MODIFY — bump engine to `^1.90.0`, update `@types/vscode`, add `languageModelTools` contribution |
| `vscode-extension/src/token-tracker.ts` | CREATE — session token savings tracker |
| `vscode-extension/src/extension.ts` | MODIFY — add LM tool, auto-capture, token savings display, status bar |

---

## Task 1: Bump VS Code version + add languageModelTools contribution

**Files:**
- Modify: `vscode-extension/package.json`

### Context

`vscode.lm.registerTool` requires VS Code `1.90.0+`. Current engine is `^1.85.0`. Must bump before adding tool registration or the TypeScript types won't include `registerTool`.

- [ ] **Step 1.1: Update package.json engine + types + contribution**

In `vscode-extension/package.json`, make these changes:

**engines.vscode**: change `"^1.85.0"` → `"^1.90.0"`

**devDependencies @types/vscode**: change `"^1.85.0"` → `"^1.90.0"`

**contributes section**: add `languageModelTools` array after `chatParticipants`:
```json
"languageModelTools": [
    {
        "name": "hipcortex_search",
        "displayName": "HipCortex Memory Search",
        "description": "Search HipCortex persistent memory for relevant context. Returns the most relevant memory records matching the query.",
        "modelDescription": "Use this tool when you need to recall previous decisions, architectural choices, debugging notes, or any information that may have been stored in the user's persistent memory. Call this instead of asking the user to repeat information they've already provided.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to search for in memory (e.g. 'database decision', 'authentication approach', 'recent bug fix')"
                }
            },
            "required": ["query"]
        }
    }
]
```

- [ ] **Step 1.2: Update npm packages**

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npm install @types/vscode@^1.90.0 --save-dev
```

- [ ] **Step 1.3: Verify TypeScript compile**

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npx tsc --noEmit
```
Expected: 0 errors (or same errors as before — don't introduce new ones).

- [ ] **Step 1.4: Commit**

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git add vscode-extension/package.json vscode-extension/package-lock.json
git commit -m "feat(vscode): bump engine to 1.90.0 for registerTool API, add languageModelTools manifest"
```

---

## Task 2: Create token-tracker.ts

**Files:**
- Create: `vscode-extension/src/token-tracker.ts`

### Context
Session-scoped token savings tracker. Tracks:
- Tokens used by HipCortex context bundles (treatment)
- Estimated tokens that full history would have used (baseline)

Token counting uses `len // 4` — consistent with HipCortex `/memory/context` endpoint.

- [ ] **Step 2.1: Create the file**

Create `vscode-extension/src/token-tracker.ts`:

```typescript
/**
 * Session token savings tracker for HipCortex.
 * 
 * Tracks how many tokens were used by HipCortex context vs estimated
 * full-history baseline. Resets when VS Code is restarted.
 * 
 * Token estimation: len(text) / 4 (consistent with /memory/context endpoint).
 * Copilot credit rate: $0.01 per 1000 tokens (approximate for Business plan).
 */

export interface TokenSavingsSnapshot {
    hipCortexTokens: number;       // tokens actually used in context bundles
    estimatedBaselineTokens: number; // estimated tokens if full history was used
    savedTokens: number;           // baseline - hipCortex
    savingsPct: number;            // savedTokens / baseline * 100
    estimatedCreditsSaved: number; // savedTokens / 1000 * 0.01
    sessionCallCount: number;      // number of times HipCortex was used
}

export class TokenTracker {
    private hipCortexTokens = 0;
    private estimatedBaselineTokens = 0;
    private sessionCallCount = 0;

    /** Rough token estimate: len(text) / 4 */
    private static estimateTokens(text: string): number {
        return Math.max(1, Math.floor(text.length / 4));
    }

    /**
     * Record a HipCortex memory retrieval event.
     * @param contextBundle  The memory records returned by HipCortex (treatment)
     * @param historyEstimate The conversation history that would have been injected (baseline)
     */
    record(contextBundle: string, historyEstimate: string): void {
        const usedTokens     = TokenTracker.estimateTokens(contextBundle);
        const baselineTokens = TokenTracker.estimateTokens(historyEstimate);
        this.hipCortexTokens            += usedTokens;
        this.estimatedBaselineTokens    += Math.max(usedTokens, baselineTokens);
        this.sessionCallCount           += 1;
    }

    getSnapshot(): TokenSavingsSnapshot {
        const saved   = Math.max(0, this.estimatedBaselineTokens - this.hipCortexTokens);
        const pct     = this.estimatedBaselineTokens > 0
            ? (saved / this.estimatedBaselineTokens) * 100
            : 0;
        const credits = saved / 1000 * 0.01;
        return {
            hipCortexTokens:          this.hipCortexTokens,
            estimatedBaselineTokens:  this.estimatedBaselineTokens,
            savedTokens:              saved,
            savingsPct:               pct,
            estimatedCreditsSaved:    credits,
            sessionCallCount:         this.sessionCallCount,
        };
    }

    /** Formatted footer for @hipcortex chat responses */
    formatSavingsFooter(latestContextTokens: number, latestBaselineTokens: number): string {
        const latestSaved = Math.max(0, latestBaselineTokens - latestContextTokens);
        const latestPct   = latestBaselineTokens > 0
            ? Math.round(latestSaved / latestBaselineTokens * 100)
            : 0;
        const snap = this.getSnapshot();
        return [
            `---`,
            `*HipCortex used ~${latestContextTokens} tokens (vs ~${latestBaselineTokens} in full history = **${latestPct}% savings**)*`,
            `*Session total: ~${snap.savedTokens.toLocaleString()} tokens saved (~$${snap.estimatedCreditsSaved.toFixed(3)} Copilot credits)*`,
        ].join('\n');
    }

    /** Short label for status bar */
    formatStatusBarLabel(): string {
        const snap = this.getSnapshot();
        if (snap.sessionCallCount === 0) {
            return '$(database) HipCortex';
        }
        return `$(database) HipCortex: ~${snap.savedTokens.toLocaleString()} tok saved`;
    }

    reset(): void {
        this.hipCortexTokens            = 0;
        this.estimatedBaselineTokens    = 0;
        this.sessionCallCount           = 0;
    }
}
```

- [ ] **Step 2.2: TypeScript compile check**

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npx tsc --noEmit
```
Expected: 0 errors.

- [ ] **Step 2.3: Commit**

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git add vscode-extension/src/token-tracker.ts
git commit -m "feat(vscode): add TokenTracker for session token savings tracking"
```

---

## Task 3: Wire LM Tool + auto-capture + token savings into extension.ts

**Files:**
- Modify: `vscode-extension/src/extension.ts`

### Context

Three additions to the `activate()` function in `extension.ts`:

1. **LM Tool**: `vscode.lm.registerTool('hipcortex_search', { ... })` — Copilot calls this automatically
2. **Auto-capture**: `vscode.workspace.onDidSaveTextDocument(...)` — stores file save as temporal memory  
3. **Status bar**: `vscode.window.createStatusBarItem(...)` — shows token savings

Also update `handleQueryMemory` / the main chat response to include a token savings footer using `TokenTracker`.

### Step 3.1: Add imports at top of extension.ts

Find the line `import * as vscode from 'vscode';` at the top. Add AFTER it:

```typescript
import { TokenTracker } from './token-tracker';
```

### Step 3.2: Add TokenTracker instance + status bar in activate()

Find the `activate(context: vscode.ExtensionContext)` function. After the opening brace (before existing code), add:

```typescript
    // ── Token savings tracker (session-scoped) ──────────────────────────────
    const tokenTracker = new TokenTracker();

    // ── Status bar item ─────────────────────────────────────────────────────
    const statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Right,
        100
    );
    statusBarItem.tooltip = 'HipCortex Memory — click to query memories';
    statusBarItem.command = 'hipcortex.queryMemory';
    statusBarItem.text = '$(database) HipCortex';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    const updateStatusBar = () => {
        statusBarItem.text = tokenTracker.formatStatusBarLabel();
    };
```

### Step 3.3: Add LM Tool registration in activate()

After the `updateStatusBar` function above, add:

```typescript
    // ── LM Tool: hipcortex_search ────────────────────────────────────────────
    // Copilot can call this tool automatically when it needs memory context.
    // Requires VS Code 1.90+. Gracefully no-ops on older versions.
    if (typeof vscode.lm?.registerTool === 'function') {
        const hipCortexTool = vscode.lm.registerTool(
            'hipcortex_search',
            {
                invoke: async (
                    options: { input: { query?: string } },
                    _token: vscode.CancellationToken
                ) => {
                    const query = options.input?.query?.trim() || 'recent decisions';
                    try {
                        const api = new HipCortexAPI();
                        const response = await api.queryMemory({ limit: 5 });
                        
                        // Build context bundle from top-5 results
                        const records = response.records
                            .filter(r => {
                                const text = `${r.actor} ${r.action} ${r.target}`.toLowerCase();
                                return query.toLowerCase().split(' ').some(w => w.length > 3 && text.includes(w));
                            })
                            .slice(0, 5);

                        if (records.length === 0) {
                            return new vscode.LanguageModelToolResult([
                                new vscode.LanguageModelTextPart('No relevant memories found for: ' + query)
                            ]);
                        }

                        const contextBundle = records
                            .map(r => `[${r.action}] ${r.target} (confidence: ${(r as any).confidence ?? 1.0})`)
                            .join('\n');

                        // Estimate baseline (full conversation history = 2000 tokens avg)
                        const ESTIMATED_FULL_HISTORY_TOKENS = 2000;
                        const bundleTokens = Math.floor(contextBundle.length / 4);
                        tokenTracker.record(contextBundle, ' '.repeat(ESTIMATED_FULL_HISTORY_TOKENS * 4));
                        updateStatusBar();

                        return new vscode.LanguageModelToolResult([
                            new vscode.LanguageModelTextPart(
                                `Relevant memories (${records.length} found):\n${contextBundle}`
                            )
                        ]);
                    } catch (error) {
                        return new vscode.LanguageModelToolResult([
                            new vscode.LanguageModelTextPart(
                                `HipCortex search failed: ${error instanceof Error ? error.message : String(error)}`
                            )
                        ]);
                    }
                }
            }
        );
        context.subscriptions.push(hipCortexTool);
        console.log('✅ HipCortex LM Tool registered: hipcortex_search');
    } else {
        console.log('ℹ️ VS Code < 1.90 — skipping LM Tool registration (no registerTool API)');
    }
```

### Step 3.4: Add auto-capture on file save in activate()

After the LM Tool block, add:

```typescript
    // ── Auto-capture: store file saves as temporal memories ──────────────────
    const onSaveDisposable = vscode.workspace.onDidSaveTextDocument(async (document) => {
        // Skip auto-generated, git, and non-workspace files
        if (document.uri.scheme !== 'file') { return; }
        if (document.fileName.includes('.git')) { return; }
        if (document.fileName.includes('node_modules')) { return; }
        
        try {
            const api = new HipCortexAPI();
            const isConnected = await api.healthCheck();
            if (!isConnected) { return; } // silent no-op if server not running
            
            const fileName = document.fileName.split(/[\\/]/).pop() || document.fileName;
            const languageId = document.languageId;
            const lineCount = document.lineCount;
            
            await api.addMemory({
                actor:       fileName,
                action:      'edited',
                target:      `${fileName} (${languageId}, ${lineCount} lines)`,
                record_type: 'Temporal',
                metadata: {
                    source:      'vscode-auto-capture',
                    language_id: languageId,
                    line_count:  lineCount,
                    full_path:   document.fileName,
                }
            });
        } catch {
            // Silent — never interrupt the user's save workflow
        }
    });
    context.subscriptions.push(onSaveDisposable);
    console.log('✅ HipCortex auto-capture on file save registered');
```

### Step 3.5: Add token savings footer to provideResponse in HipCortexChatParticipant

Find the `provideResponse` method in `HipCortexChatParticipant`. It needs access to `tokenTracker`. The cleanest way: pass `tokenTracker` to the participant constructor.

First, update `HipCortexChatParticipant` class definition (find `class HipCortexChatParticipant`):

Change:
```typescript
class HipCortexChatParticipant {
    private api: HipCortexAPI;

    constructor() {
        this.api = new HipCortexAPI();
    }
```

To:
```typescript
class HipCortexChatParticipant {
    private api: HipCortexAPI;
    private tokenTracker: TokenTracker;

    constructor(tokenTracker: TokenTracker) {
        this.api = new HipCortexAPI();
        this.tokenTracker = tokenTracker;
    }
```

Then find where `HipCortexChatParticipant` is instantiated (in `activate()`):
```typescript
    const chatParticipant = new HipCortexChatParticipant();
```
Change to:
```typescript
    const chatParticipant = new HipCortexChatParticipant(tokenTracker);
```

Then find `handleQueryMemory` inside `HipCortexChatParticipant`. After the query result is streamed, add the savings footer. Find the response records being displayed and add at the end:

```typescript
            // Token savings footer
            const contextBundle = records.map((r: any) => `${r.action} ${r.target}`).join('\n');
            const ESTIMATED_HISTORY = ' '.repeat(2000 * 4); // ~2000 token estimate
            this.tokenTracker.record(contextBundle, ESTIMATED_HISTORY);
            const footer = this.tokenTracker.formatSavingsFooter(
                Math.floor(contextBundle.length / 4),
                2000
            );
            stream.markdown('\n\n' + footer);
```

**IMPORTANT:** Place the footer AFTER the last `stream.markdown()` call in `handleQueryMemory`, before the closing brace of the try block.

### Step 3.6: TypeScript compile check

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npx tsc --noEmit
```
Expected: 0 errors (except possibly warnings about `vscode.LanguageModelToolResult` and `vscode.LanguageModelTextPart` — these are new in 1.90 and may not have full type definitions in @types/vscode yet).

If `vscode.LanguageModelToolResult` or `vscode.LanguageModelTextPart` cause type errors, use type assertion:
```typescript
return { content: [{ type: 'text', value: contextBundle }] } as any;
```

### Step 3.7: Build extension

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npm run compile
```
Expected: build succeeds, `dist/extension.js` updated.

### Step 3.8: Package extension

```bash
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c\vscode-extension"
npx vsce package
```
Expected: `hipcortex-memory-0.1.6.vsix` created.

Update version in `package.json` to `0.1.6` before packaging.

### Step 3.9: Commit + push

```powershell
cd "D:\all_projects\HipCortex\.claude\worktrees\pedantic-edison-28b84c"
git add vscode-extension/
git commit -m "feat(vscode): v0.1.6 — LM Tool for Copilot auto-call, file save auto-capture, token savings display"
git push origin claude/pedantic-edison-28b84c
```

---

## Acceptance Criteria

- [ ] `package.json` engine is `^1.90.0`
- [ ] `languageModelTools` contribution declares `hipcortex_search` tool
- [ ] `vscode.lm.registerTool('hipcortex_search', ...)` registered in `activate()`
- [ ] LM Tool gracefully no-ops on VS Code < 1.90 (guard with `typeof` check)
- [ ] `onDidSaveTextDocument` fires → silent HipCortex add (no error if server down)
- [ ] Status bar shows "HipCortex: ~N tok saved" after first search
- [ ] `@hipcortex` query response includes token savings footer
- [ ] `npx tsc --noEmit` passes
- [ ] `npm run compile` succeeds
- [ ] `hipcortex-memory-0.1.6.vsix` produced

## Known Gotchas

1. **`vscode.LanguageModelToolResult` types** — may not be in `@types/vscode@1.90.0`. If so, use `as any` cast or inline type definition.
2. **`vscode.lm.registerTool` signature** — the exact API shape may vary. Check VS Code 1.90 release notes if TypeScript errors arise. Use `(vscode.lm as any).registerTool(...)` as fallback.
3. **Auto-capture silent failure** — MUST be silent (try/catch with no error display). Never interrupt file save.
4. **Token tracker passed to constructor** — `chatParticipant` is instantiated before `updateStatusBar` in current code. Reorder if needed so `tokenTracker` is created first.
5. **`handleQueryMemory` location** — find the exact spot where records are displayed and add footer AFTER, not in the middle of response streaming.

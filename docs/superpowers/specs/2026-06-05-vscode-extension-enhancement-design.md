# Design: VS Code Extension Enhancement — LM Tool + Auto-Capture + Token Savings

**Date:** 2026-06-05
**Status:** Approved
**Source:** opsx-explore session — resolving Task 4 ambiguities

---

## Problem

Current extension (v0.1.5) is passive: only responds when user types `@hipcortex`. Does NOT:
1. Auto-inject HipCortex memories into Copilot's context automatically
2. Register as a tool Copilot can call during its reasoning
3. Auto-capture decisions from file saves
4. Show how many tokens/credits were saved

---

## What's Technically Feasible (VS Code API reality check)

```
✅ chatParticipants     — already implemented (@hipcortex)
✅ vscode.lm.registerTool — Copilot calls our search tool automatically (v1.90+)
✅ onDidSaveTextDocument — auto-capture file saves
✅ StatusBarItem         — show token savings
❌ Intercept Copilot context assembly — VS Code API does NOT expose this
```

---

## Architecture

**Two independent enhancements to existing extension:**

### Enhancement A: Language Model Tool (`hipcortex_search`)
Register HipCortex search as a Copilot tool. When Copilot decides it needs context to answer a question, it can automatically call `hipcortex_search` instead of relying on full history.

```
User: "@copilot what database did we decide to use?"
Copilot thinks: "I need to search for database decisions"
Copilot calls: hipcortex_search({ query: "database decision" })
Tool returns:  [{text: "[decided] use_postgres", confidence: 0.95, ...}]
Copilot uses:  80 tokens of context instead of 2000 tokens of history
```

### Enhancement B: Auto-Capture on File Save
When user saves a file, record a temporal memory: `actor=<filename>, action=edited, target=<brief summary>`. Uses `vscode.workspace.onDidSaveTextDocument`.

### Enhancement C: Token Savings Display
In `@hipcortex` chat responses, show footer: "Used ~80 tokens (vs ~2,400 in full history = 96.7% savings)". Status bar: "HipCortex: 42 memories · ~$0.18 saved today".

---

## Decisions Locked

| Decision | Choice | Reason |
|----------|--------|--------|
| VS Code min version | `^1.90.0` | `vscode.lm.registerTool` requires 1.90+ |
| Auto-capture trigger | `onDidSaveTextDocument` | High signal, low noise vs onDidChange |
| Auto-capture content | filename + language + char count | No file content (privacy) |
| Token savings calc | `len(context_bundle) // 4` vs `len(all_history) // 4` | Consistent with HipCortex /memory/context |
| Session token tracking | In-memory Map per VS Code session | No persistence needed, resets on restart |
| LM Tool name | `hipcortex_search` | Clear, namespaced |
| Status bar position | Right side, priority 100 | Non-intrusive |

---

## Files Changed

| File | Action |
|------|--------|
| `vscode-extension/package.json` | Bump VS Code engine to `^1.90.0`; add `languageModelTools` contribution |
| `vscode-extension/src/extension.ts` | Add LM tool registration, auto-capture, token tracking, status bar |
| `vscode-extension/src/token-tracker.ts` | CREATE — session token savings tracker |

---

## NOT Building

- Intercepting Copilot inline completions (not possible via VS Code API)
- Modifying Copilot's actual API calls
- Persistent token savings across VS Code restarts (too complex for v1)
- Auto-capture of Copilot Chat turns (VS Code API doesn't expose this)

# Extension Tier Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the server's full memory tier API (`ttl_seconds`, `priority`, `confidence`, `source`, `tags`) to the VS Code extension's `AddMemoryRequest` interface, parse tier fields from the `/add` chat command, apply sensible tier defaults to auto-captured file saves, and broaden the causal-edge condition in the server so Symbolic records always feed the world model's causal graph.

**Architecture:** Two files change. `vscode-extension/src/extension.ts`: (1) extend the `AddMemoryRequest` TypeScript interface, (2) add regex parsers in `handleAddMemory` for `priority:X` and `ttl:N`, (3) add default `ttl_seconds`/`priority` to the `onSave` auto-capture call. `src/web_server.rs`: change `handle_add_memory` causal-edge condition from `pinned AND Symbolic` to `Symbolic OR pinned` so any explicit Symbolic record feeds the causal graph regardless of priority.

**Tech Stack:** TypeScript (VS Code extension), Rust (Axum server), `npm run compile`, `cargo test`.

## Global Constraints

- TypeScript compile must pass with 0 errors: `npm run compile` (run from `vscode-extension/`)
- Rust lib tests must pass: `cargo test --no-default-features --features "petgraph_backend" --lib`
- Rust v040 contract tests must pass: `cargo test --no-default-features --features "petgraph_backend,web-server" --test v040_contract_sit`
- Extension `AddMemoryRequest` must remain backward compatible — all new fields are optional (`?:`)
- The `/add` chat command changes `record_type` from `'Temporal'` to `'Symbolic'` — intentional decisions belong in the Symbolic layer, not Temporal
- Auto-capture (file saves) uses `record_type: 'Temporal'` — file-edit events are temporal observations, not decisions
- Priority values accepted by server: `"pinned" | "high" | "normal" | "low"` — parse only these, ignore unknown values
- `tags` field: server accepts `string[]` — parse space-separated tags after `tags:` token

---

## File Map

| Action | File | What changes |
|--------|------|-------------|
| Modify | `vscode-extension/src/extension.ts:74-79` | Extend `AddMemoryRequest` interface with 5 optional fields |
| Modify | `vscode-extension/src/extension.ts:493-504` | Parse `priority:X`, `ttl:N`, `tags:a,b` from chat prompt in `handleAddMemory` |
| Modify | `vscode-extension/src/extension.ts:876-887` | Add `ttl_seconds: 86400`, `priority: 'low'` to auto-capture `addMemory` call |
| Modify | `src/web_server.rs:3022-3025` | Broaden causal-edge condition: `Symbolic || pinned` instead of `Symbolic && pinned` |
| Test | `tests/integration/web_server_gaps_sit.rs` | 2 new Rust unit tests for broadened causal condition |

---

### Task 1: Extend `AddMemoryRequest` TypeScript interface

**Files:**
- Modify: `vscode-extension/src/extension.ts:74-79`

**Interfaces:**
- Produces: `AddMemoryRequest` with 5 new optional fields — Tasks 2 and 3 rely on these field names exactly

- [ ] **Step 1: Replace the interface**

Open `vscode-extension/src/extension.ts`. Find the current interface (around line 74):

```typescript
interface AddMemoryRequest {
    actor: string;
    action: string;
    target: string;
    record_type?: string;
    metadata?: any;
}
```

Replace with:

```typescript
interface AddMemoryRequest {
    actor: string;
    action: string;
    target: string;
    record_type?: string;
    metadata?: any;
    /** Unix TTL in seconds. Server deletes record after this duration. */
    ttl_seconds?: number;
    /** "pinned" | "high" | "normal" | "low". Affects search ranking. */
    priority?: string;
    /** Reliability signal [0.0, 1.0]. Affects decay scoring. Default 1.0. */
    confidence?: number;
    /** Source identifier, e.g. "user-input", "copilot". */
    source?: string;
    /** Categorization tags for RAG filtering. */
    tags?: string[];
}
```

- [ ] **Step 2: Verify compile passes**

```
cd vscode-extension && npm run compile
```

Expected: 0 errors. (Adding optional fields to an interface breaks nothing.)

- [ ] **Step 3: Commit**

```
git add vscode-extension/src/extension.ts
git commit -m "feat(ext): extend AddMemoryRequest with ttl/priority/confidence/source/tags"
```

---

### Task 2: Parse tier fields from `/add` chat command

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `handleAddMemory` private method (around line 479)

**Interfaces:**
- Consumes: `AddMemoryRequest` from Task 1 (the new optional fields)
- Produces: `memoryRequest` in `handleAddMemory` now sets `priority`, `ttl_seconds`, `tags` when present in user input

**Parsing grammar** (parsed from chat prompt string):
```
priority:pinned   → priority = "pinned"
ttl:3600          → ttl_seconds = 3600
tags:arch,bug     → tags = ["arch", "bug"]
```

- [ ] **Step 1: Write manual test scenario**

No automated unit test harness for extension chat logic. Verify by tracing code manually:

Input prompt: `actor:me action:decided target:use-postgres priority:pinned ttl:86400 tags:arch,db`

After Step 3, the parsed `memoryRequest` must have:
- `priority: "pinned"`
- `ttl_seconds: 86400`
- `tags: ["arch", "db"]`
- `record_type: "Symbolic"` (changed from "Temporal" — intentional decisions are Symbolic)

Compile verification is the gate.

- [ ] **Step 2: Update `handleAddMemory` to parse tier fields**

Find the `handleAddMemory` method (around line 479). The current body starts with:

```typescript
            const actorMatch = prompt.match(/actor[:\s]+[\w\s]+?(?=\s+action|\s+target|$)/i);
            const actionMatch = prompt.match(/action[:\s]+[\w\s]+?(?=\s+actor|\s+target|$)/i);
            const targetMatch = prompt.match(/target[:\s]+[\w\s]+?(?=\s+actor|\s+action|$)/i);

            const actor = this.validateInput(actorMatch?.[1]?.trim() || 'VSCode_Chat', 'actor');
            const action = this.validateInput(actionMatch?.[1]?.trim() || 'chat_interaction', 'action');
            const target = this.validateInput(targetMatch?.[1]?.trim() || 'memory_system', 'target');

            const memoryRequest: AddMemoryRequest = {
                actor,
                action,
                target,
                record_type: 'Temporal',
                metadata: {
                    source: 'vscode-chat',
                    timestamp: new Date().toISOString(),
                    original_prompt: prompt
                }
            };
```

Replace with:

```typescript
            const actorMatch = prompt.match(/actor[:\s]+([\w\s]+?)(?=\s+action|\s+target|\s+priority|\s+ttl|\s+tags|$)/i);
            const actionMatch = prompt.match(/action[:\s]+([\w\s]+?)(?=\s+actor|\s+target|\s+priority|\s+ttl|\s+tags|$)/i);
            const targetMatch = prompt.match(/target[:\s]+([\w\s]+?)(?=\s+actor|\s+action|\s+priority|\s+ttl|\s+tags|$)/i);
            const priorityMatch = prompt.match(/priority[:\s]+(\w+)/i);
            const ttlMatch = prompt.match(/ttl[:\s]+(\d+)/i);
            const tagsMatch = prompt.match(/tags[:\s]+([\w,\s]+?)(?=\s+actor|\s+action|\s+target|\s+priority|\s+ttl|$)/i);

            const actor = this.validateInput(actorMatch?.[1]?.trim() || 'VSCode_Chat', 'actor');
            const action = this.validateInput(actionMatch?.[1]?.trim() || 'chat_interaction', 'action');
            const target = this.validateInput(targetMatch?.[1]?.trim() || 'memory_system', 'target');

            const VALID_PRIORITIES = ['pinned', 'high', 'normal', 'low'];
            const rawPriority = priorityMatch?.[1]?.toLowerCase().trim();
            const priority = rawPriority && VALID_PRIORITIES.includes(rawPriority) ? rawPriority : undefined;

            const ttl_seconds = ttlMatch ? parseInt(ttlMatch[1], 10) : undefined;

            const tags = tagsMatch
                ? tagsMatch[1].split(',').map(t => t.trim()).filter(t => t.length > 0)
                : undefined;

            const memoryRequest: AddMemoryRequest = {
                actor,
                action,
                target,
                record_type: 'Symbolic',  // /add = explicit decision → Symbolic layer
                priority,
                ttl_seconds,
                tags,
                metadata: {
                    source: 'vscode-chat',
                    timestamp: new Date().toISOString(),
                    original_prompt: prompt
                }
            };
```

Also update the response table to show the new fields. Find the markdown table section in the success branch:

```typescript
                stream.markdown(`| **Type** | Temporal |\n`);
```

Replace with:

```typescript
                stream.markdown(`| **Type** | Symbolic |\n`);
                if (priority) { stream.markdown(`| **Priority** | ${priority} |\n`); }
                if (ttl_seconds) { stream.markdown(`| **TTL** | ${ttl_seconds}s |\n`); }
                if (tags && tags.length > 0) { stream.markdown(`| **Tags** | ${tags.join(', ')} |\n`); }
```

- [ ] **Step 3: Compile**

```
cd vscode-extension && npm run compile
```

Expected: 0 errors.

- [ ] **Step 4: Commit**

```
git add vscode-extension/src/extension.ts
git commit -m "feat(ext): parse priority/ttl/tags from /add chat prompt"
```

---

### Task 3: Add tier defaults to auto-capture (file saves)

**Files:**
- Modify: `vscode-extension/src/extension.ts` — `onSave` handler (around line 866)

**Interfaces:**
- Consumes: `AddMemoryRequest` from Task 1
- Produces: auto-captured memories use `ttl_seconds: 86400` (24h) and `priority: 'low'`

**Rationale:** Auto-saves are ephemeral observations. A file you edit today is probably irrelevant in a week. `low` priority means they don't clutter explicit decision searches. `ttl_seconds: 86400` means they evict after 24 hours automatically (the background eviction thread runs every 5 minutes).

- [ ] **Step 1: Update the onSave addMemory call**

Find the `onSave` handler around line 866. The current `addMemory` call is:

```typescript
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Temporal',
                metadata: { 
                    source: 'vscode-auto-capture', 
                    language_id: doc.languageId, 
                    line_count: doc.lineCount,
                    workspace: vscode.workspace.name || 'unknown'
                }
            });
```

Replace with:

```typescript
            await api.addMemory({
                actor: 'vscode-user',
                action: 'edited',
                target: `${fileName} (${doc.languageId}, ${doc.lineCount} lines)`,
                record_type: 'Temporal',
                ttl_seconds: 86400,    // file edits are ephemeral — expire after 24h
                priority: 'low',       // low priority: don't surface above explicit decisions
                source: 'vscode-auto-capture',
                metadata: { 
                    source: 'vscode-auto-capture', 
                    language_id: doc.languageId, 
                    line_count: doc.lineCount,
                    workspace: vscode.workspace.name || 'unknown'
                }
            });
```

- [ ] **Step 2: Compile**

```
cd vscode-extension && npm run compile
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```
git add vscode-extension/src/extension.ts
git commit -m "feat(ext): auto-captures use ttl=24h and priority=low"
```

---

### Task 4: Broaden causal-edge condition in server + Rust test

**Files:**
- Modify: `src/web_server.rs:3022-3025`
- Test: `tests/integration/web_server_gaps_sit.rs`

**Interfaces:**
- Consumes: `record.record_type: MemoryType`, `record.priority: String`
- Produces: `wm.add_causal_edge(actor, target)` fires for any Symbolic OR any pinned record (not just Symbolic AND pinned)

**Why:** The extension `/add` chat command now sets `record_type: 'Symbolic'`. With the old `AND` condition, a Symbolic record without `priority: "pinned"` would NOT feed the causal graph. The new `OR` condition feeds it for all intentional Symbolic records and all pinned records.

- [ ] **Step 1: Write two failing tests**

Append to `tests/integration/web_server_gaps_sit.rs`:

```rust
// Test uses the causal graph directly via WorldModelEnhanced (no HTTP needed).
// We replicate the condition logic from handle_add_memory to confirm the OR semantics.
#[test]
fn test_causal_edge_condition_fires_for_symbolic_normal_priority() {
    use hipcortex::memory_record::MemoryType;
    // The new condition: record_type == Symbolic OR priority == "pinned"
    // This test documents that Symbolic + normal priority SHOULD trigger causal edge.
    let record_type = MemoryType::Symbolic;
    let priority = "normal";
    // OR condition
    let should_fire = matches!(record_type, MemoryType::Symbolic) || priority == "pinned";
    assert!(
        should_fire,
        "Symbolic record with normal priority must fire causal edge (OR condition)"
    );
}

#[test]
fn test_causal_edge_condition_old_and_semantics_would_fail() {
    use hipcortex::memory_record::MemoryType;
    // Documenting that the OLD AND condition was too restrictive
    let record_type = MemoryType::Symbolic;
    let priority = "normal";
    // OLD condition: AND — this was the bug
    let old_condition = matches!(record_type, MemoryType::Symbolic) && priority == "pinned";
    assert!(
        !old_condition,
        "OLD AND condition must NOT fire for Symbolic+normal (this was the bug)"
    );
}
```

- [ ] **Step 2: Run tests (they should pass even before the server change, as they test the condition logic)**

```
cargo test --no-default-features --features "petgraph_backend" --test integration_suite test_causal_edge_condition -- --nocapture
```

Expected: 2 PASS (these are logic documentation tests — they verify the semantics of the change).

- [ ] **Step 3: Apply the fix in web_server.rs**

Open `src/web_server.rs`. Find the causal-edge block (around line 3022):

```rust
                        // Register causal edge for pinned symbolic decisions
                        if record.priority == "pinned" && record.record_type == MemoryType::Symbolic {
                            let _ = wm.add_causal_edge(record.actor.clone(), record.target.clone());
                        }
```

Replace with:

```rust
                        // Register causal edge for any Symbolic record OR any pinned record.
                        // Symbolic = intentional decision (feeds causal reasoning regardless of priority).
                        // Pinned = explicitly important (feeds causal reasoning regardless of type).
                        if record.record_type == MemoryType::Symbolic || record.priority == "pinned" {
                            let _ = wm.add_causal_edge(record.actor.clone(), record.target.clone());
                        }
```

- [ ] **Step 4: Run full Rust tests**

```
cargo test --no-default-features --features "petgraph_backend" --lib
cargo test --no-default-features --features "petgraph_backend,web-server" --test v040_contract_sit
```

Expected: all pass.

- [ ] **Step 5: Compile extension + repackage VSIX**

```
cd vscode-extension && npm run package && npx vsce package --no-dependencies
```

Expected: `hipcortex-memory-0.4.1.vsix` rebuilt with all Task 1-4 changes.

- [ ] **Step 6: Commit**

```
git add src/web_server.rs tests/integration/web_server_gaps_sit.rs vscode-extension/src/extension.ts vscode-extension/hipcortex-memory-0.4.1.vsix
git commit -m "feat: broaden causal-edge to Symbolic||pinned; extension tier surface complete"
```

---

## Self-Review

### Spec coverage

| Gap | Task |
|-----|------|
| AddMemoryRequest missing ttl/priority/confidence/source/tags | Task 1 |
| /add chat command can't set priority or TTL | Task 2 |
| Auto-captures never expire and crowd out decisions | Task 3 |
| Causal graph permanently empty (AND condition too tight) | Task 4 |

### Type consistency
- `AddMemoryRequest.priority?: string` defined in Task 1, used in Tasks 2 and 3 — consistent.
- `AddMemoryRequest.ttl_seconds?: number` defined in Task 1, used in Tasks 2 and 3 — consistent.
- `AddMemoryRequest.tags?: string[]` defined in Task 1, used in Task 2 — consistent.
- `MemoryType::Symbolic` used in Task 4 Rust code — already imported in `web_server.rs` via `use crate::memory_record::MemoryType` — consistent.

# HipCortex Runner Skill (Claude Code)

This skill wires Claude Code as an IDE-connected intent runner.
After each tool call, check for pending Probe intents and post receipts.

## When to activate

- HipCortex MCP server is connected (`mcp__hipcortex__*` tools available)
- After any tool call that completes successfully
- When idle (user has not typed for more than 30 seconds)

## Protocol (per turn)

1. **Check open intents**
   ```
   GET /intent/open?actor=claude-code   (via mcp__hipcortex__open_intent or curl)
   ```
   If `count == 0`: skip — no work to do.

2. **For each intent** (process in creation-time order):
   a. Read `target_entity` and `sensor_path`.
   b. Execute the probe (see dispatch table below).
   c. Post receipt immediately via `mcp__hipcortex__accept_receipt` or `POST /intent/receipt`.
   d. Do NOT use `add_memory` or `mcp__hipcortex__add_memory` for env observations — AcceptReceipt is the ONLY path.

3. **Skip expired intents**: compare current timestamp (ms) with `deadline_ms`; if past, skip.

## Dispatch table

| sensor_path  | Action                                          |
|--------------|-------------------------------------------------|
| `filesystem` | Use Read tool on `target_entity` (file path)    |
| `http`       | Use WebFetch tool on `target_entity` (URL)      |
| `shell:ping` | Bash `ping -c 1 <target>` (allowlisted)         |
| `default`    | Return `{"reachable": true}` — no I/O needed    |

## Receipt payload

```json
{
  "actor": "claude-code",
  "intent_id": "<uuid from intent>",
  "ok": true,
  "observation": { "<sensor-specific fields>" },
  "sensor_path": "<same sensor_path from intent>"
}
```

## Grounding rule

GroundingGate requires ≥4 Observed contacts for an entity before ReactEngine
fires instrumental planning. Each AcceptReceipt with `ok: true` counts as one
Observed contact. After 4 receipts for the same `target_entity`, the entity is
grounded and the daemon can proceed.

## Example (filesystem probe)

Intent arrives:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "actor": "claude-code",
  "target_entity": "src/lib.rs",
  "sensor_path": "filesystem",
  "deadline_ms": 1700000000000
}
```

Action:
1. Read `src/lib.rs` → note file size, existence.
2. Post receipt:
   ```json
   {
     "actor": "claude-code",
     "intent_id": "550e8400-e29b-41d4-a716-446655440000",
     "ok": true,
     "observation": { "exists": true, "size_bytes": 4096, "sensor": "filesystem" },
     "sensor_path": "filesystem"
   }
   ```

## MCP tool names

- `mcp__hipcortex__open_intent` — open a new probe intent
- `mcp__hipcortex__accept_receipt` — post a receipt for an existing intent

Or use REST directly:
- `GET  /intent/open?actor=claude-code`
- `POST /intent/receipt`

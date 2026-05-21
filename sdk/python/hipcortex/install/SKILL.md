# HipCortex Memory

HipCortex gives you persistent causal memory across sessions. Store decisions, recall context, forget on request.

## When to use

Invoke HipCortex when the user asks you to:
- Remember something ("remember that we use JWT")
- Recall past context ("what did we decide about the database?")
- Forget data ("forget everything about project X")
- Store a decision, bug fix, or architectural note

## How to use

**Store a memory:**
```
POST http://localhost:3030/memory/add
{"actor": "<project-or-user>", "action": "decided", "target": "<what to remember>"}
```

**Search memories:**
```
GET http://localhost:3030/memory/search-flat?query=<topic>&limit=10
Returns: {"memories": ["[action] target", ...]}
```

**Forget (GDPR):**
```
DELETE http://localhost:3030/memory/forget/<actor>
```

**Stats:**
```
GET http://localhost:3030/stats
```

## Slash commands

When the user types `/hipcortex remember <text>` — call POST /memory/add with actor=current-project-name.
When the user types `/hipcortex recall <query>` — call GET /memory/search-flat?query=<query>.
When the user types `/hipcortex forget <actor>` — call DELETE /memory/forget/<actor>.
When the user types `/hipcortex stats` — call GET /stats and display the result.

## Auto-memory mode

If the user says "remember this" at the end of any message, automatically store a summary of the conversation turn in HipCortex before responding.

## Default actor

Use the current git repository name as the actor (run `git rev-parse --show-toplevel | xargs basename` to get it). Fall back to "default" if not in a git repo.

## Server

Default: http://localhost:3030
Managed free tier: https://hipcortex.fly.dev (set HIPCORTEX_URL to override)

The server must be running. If unreachable, tell the user to run: `hipcortex start`

# GitHub Copilot Extension Plan

## Overview
Build a GitHub Copilot Extension that gives Copilot Chat persistent memory via HipCortex. Target: 3M+ Copilot users.

## Architecture
```
Copilot Chat → Extension API → HipCortex MCP Server → Memory Store
                                    ↓
                            /memory/search (context injection)
                            /memory/add    (learn from conversation)
                            /memory/forget (GDPR compliance)
```

## User Flow
1. Developer types `@hipcortex remember this conversation`
2. Extension extracts key facts, adds to HipCortex
3. Next session: `@hipcortex what did we discuss about X?`
4. HipCortex returns relevant context → injected into Copilot prompt

## Technical Plan

### Phase 1: Extension scaffold (Day 1)
- Create `copilot-extension/` directory
- `manifest.json` — Copilot Extension manifest
- `server.py` — MCP server wrapper for Copilot
- Register commands: `/remember`, `/recall`, `/forget`, `/context`

### Phase 2: Core integration (Day 2)
- Implement context injection: before each Copilot response, search HipCortex for relevant memory
- Implement memory extraction: after conversation, extract decisions and key facts
- Token budget management: limit injected context to ~300 tokens

### Phase 3: Polish + ship (Day 3)
- GitHub App registration
- OAuth flow for HipCortex server connection
- README, demo video, marketplace listing
- Submit to GitHub Copilot Extension Marketplace

## Key Files
```
copilot-extension/
├── manifest.json          # Extension manifest
├── server.py              # MCP server → Copilot bridge
├── copilot_memory.py      # Memory integration logic
├── context_injector.py    # Pre-prompt context injection
├── requirements.txt       # Python deps
└── README.md             # User docs
```

## Commands
| Command | Description |
|---------|-------------|
| `@hipcortex remember` | Extract and store key facts from conversation |
| `@hipcortex recall <query>` | Search memory and inject into context |
| `@hipcortex forget` | GDPR right-to-forget current user |
| `@hipcortex context` | Show current memory context being injected |
| `@hipcortex health` | Show HipCortex server health status |

## Monetization
Free tier: 100 memories, 7-day retention
Pro tier ($9/mo): unlimited, permanent retention, intelligence layer
Team tier ($29/mo): shared team memory, audit logs

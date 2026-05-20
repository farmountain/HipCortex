# HipCortex — Continue.dev Context Provider

Give Continue.dev persistent memory: recall past decisions, architecture notes, bug fixes, and project context across sessions.

## Install (2 minutes)

**1. Copy the provider to your Continue config:**

```bash
mkdir -p ~/.continue/providers
curl -fsSL https://raw.githubusercontent.com/farmountain/HipCortex/main/sdk/continue/index.ts \
  -o ~/.continue/providers/hipcortex.ts
```

**2. Add to `~/.continue/config.json`:**

```json
{
  "contextProviders": [
    {
      "name": "hipcortex",
      "params": {
        "url": "http://localhost:3030"
      }
    }
  ],
  "slashCommands": [
    { "name": "remember", "description": "Store a note in HipCortex" },
    { "name": "recall",   "description": "Search HipCortex memory" }
  ]
}
```

**3. Start HipCortex:**

```bash
# Pre-built binary (no Rust needed)
curl -L https://github.com/farmountain/HipCortex/releases/latest/download/hipcortex-linux-amd64 \
  -o hipcortex && chmod +x hipcortex && ./hipcortex

# Or use managed free tier (no install):
# Set url to: https://hipcortex.fly.dev
```

## Usage

```
@hipcortex why did we choose JWT?
@hipcortex what bugs did we fix in auth module?
@hipcortex architecture decisions for database layer

/remember We chose PostgreSQL over SQLite for multi-user support
/recall database
```

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HIPCORTEX_URL` | `http://localhost:3030` | HipCortex server URL |
| `HIPCORTEX_API_KEY` | *(unset)* | API key for managed tiers |
| `HIPCORTEX_ACTOR` | `continue-dev` | Default actor for stored memories |

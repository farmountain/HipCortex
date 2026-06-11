---
title: HipCortex
emoji: 🧠
colorFrom: blue
colorTo: purple
sdk: docker
pinned: true
license: apache-2.0
app_port: 3030
---

# HipCortex — AI Memory Engine

Persistent causal memory for AI agents with metacognitive intelligence layer.

## Features
- **0.6ms writes** with SHA-256 audit trail
- **Intelligence layer**: Self-Model, World-Model, Coherence Checker
- **45+ REST endpoints**: health, prediction, coherence, memory operations
- **MCP server**: Works with Cursor, Claude Code, Windsurf
- **GDPR-compliant**: DELETE /memory/forget/:actor

## API Quickstart

```sh
# Health check
curl https://hipcortex-hf.hf.space/health

# Self-model health
curl https://hipcortex-hf.hf.space/self/health

# Predict next state
curl -X POST https://hipcortex-hf.hf.space/worldmodel/predict \
  -H "Content-Type: application/json" \
  -d '{"state":"idle","action":"process"}'

# Add memory
curl -X POST https://hipcortex-hf.hf.space/memory/add \
  -H "Content-Type: application/json" \
  -d '{"actor":"user","action":"asked","target":"question"}'

# Search memory
curl https://hipcortex-hf.hf.space/memory/search?q=question
```

## Links
- GitHub: https://github.com/farmountain/HipCortex
- PyPI: https://pypi.org/project/hipcortex
- npm: https://www.npmjs.com/package/hipcortex

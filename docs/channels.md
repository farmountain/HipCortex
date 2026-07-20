# HipCortex channel matrix

**Source of truth:** [`docs/channels.yaml`](channels.yaml)  
**Print CLI:** `hipcortex channels`  
**Updated:** 2026-07-18 (Phase 6B — Antigravity / Hermes / OpenClaw installers)

## Status legend

| Status | Meaning |
|--------|---------|
| **native** | First-class product surface (package, VSIX, skill installer) |
| **mcp** | Wizard writes MCP host config |
| **framework** | Package adapter and/or install scaffold |
| **guide** | Docs / copy-paste only — no auto-installer |
| **claimed** | Mentioned in marketing or guessed MCP path — **not** in install registry |
| **none** | Internal / not a consumer install channel |

**Rule:** README must not present a channel as first-class unless status is `native`, `mcp`, or `framework`.

---

## Distribution

| Channel | Status | Install | Notes |
|---------|--------|---------|-------|
| Rust binary / Cargo | native | `cargo build` / GitHub Releases | Crate **0.5.1** (Win/macOS/Linux amd64+arm64) |
| pip (Python SDK + CLI) | native | `pip install hipcortex` | Wizard + SDK **0.5.1** (platform-agnostic) |
| npm (TypeScript SDK) | native | `npm install hipcortex` | Client only; no wizard |
| Docker | native | `docker run -p 3030:3030 …` | Image may lag releases |
| Managed tier (Fly) | native | `hipcortex install --url https://hipcortex.fly.dev` | Remote HTTP |
| MCP server (Python stdio) | mcp | `hipcortex install` → `~/.hipcortex-mcp/` | Banner **0.5.1**; **18 tools** (topo PPR, deconstruct, check_edge, rollout MCTS, can_execute, …) |
| VS Code / Antigravity VSIX | native | [release `v0.5.7`](https://github.com/farmountain/HipCortex/releases/download/v0.5.7/hipcortex-memory-0.5.7.vsix) | Ext **0.5.7**; **10 LM tools** + dual `/health`; bundled multi-OS webserver |

---

## Coding assistants

| Channel | Status | Install | Notes |
|---------|--------|---------|-------|
| Claude Code | native | wizard → SKILL.md | Proactive: `--mode proactive` |
| Cursor | mcp | wizard → `.cursor/mcp.json` | |
| Windsurf | mcp | wizard → Codeium MCP settings | |
| VS Code (MCP wizard) | mcp | wizard → `settings.json` | Parallel to VSIX |
| Cline | mcp | wizard → `.cline/mcp.json` | |
| RooCode | mcp | wizard → `.roo/mcp.json` | |
| Antigravity IDE | mcp | wizard → `~/.gemini/antigravity/mcp_config.json` | Also VSIX path; [hosts](hosts/README.md) |
| Hermes Agent | mcp | wizard → `~/.hermes/config.yaml` | Needs `~/.hermes` present |
| OpenClaw | mcp | wizard → `~/.openclaw/openclaw.json` | JSON5 sidecar fallback |
| Grok Code / Grok Build | mcp | wizard → `~/.grok/config.toml` | `GROK_CONFIG_PATH`; uninstall `--channel grok` |
| Continue | guide | `sdk/continue/README.md` | |
| GitHub Copilot | guide | docs / OpenAPI | |
| OpenAI Codex CLI | guide | shell / MCP docs | |
| Aider | guide | shell integration docs | |
| Gemini CLI | guide | manual MCP | |
| Amazon Q Developer | guide | manual MCP example | |

---

## Agent frameworks

| Channel | Status | Install | Notes |
|---------|--------|---------|-------|
| LangChain | framework | `hipcortex.langchain_memory` + scaffold | Package-first preferred |
| CrewAI | framework | `hipcortex.adapters.crewai` | |
| AutoGen | framework | `hipcortex.adapters.autogen` | |
| LlamaIndex | framework | `hipcortex.llamaindex_storage` | |
| Pydantic AI | framework | wizard scaffold | Thin REST tools |
| DSPy | framework | wizard scaffold | Thin |
| n8n / Make.com | framework | curl snippet scaffold | HTTP only |
| Flowise / Dify | guide | paste OpenAPI URL | |

---

## Internal

| Channel | Status | Notes |
|---------|--------|-------|
| OpenManus | none | In-process Rust integration — not a consumer channel |

---

## Version snapshot

| Surface | Version |
|---------|---------|
| Cargo / pip / npm (product) | 0.5.1 |
| VS Code extension (VSIX) | 0.5.7 |
| MCP `serverInfo.version` | 0.5.1 (stamped from VERSION) |

See also: [host install notes](hosts/README.md), [cross-channel UX plan](superpowers/plans/2026-07-18-cross-channel-ux-enhancement-plan.md), [problem catalog](superpowers/specs/2026-07-18-cross-channel-ux-problem-catalog.md).

## Project identity

Use project file `.hipcortex/config.toml` (url, actor, mode, channels, aliases). See [docs/usage.md](usage.md).

## Capabilities

See [capabilities.md](capabilities.md) for REST / MCP / SDK / extension matrix.

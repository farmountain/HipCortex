# HipCortex Cross-Channel UX Problem Catalog

**Date:** 2026-07-18  
**Method:** Codebase inventory + GitNexus process graph (activate/autostart, MCP tools, install CLI, live_beliefs)  
**Scope:** End-user experience across all distribution channels and agent frameworks  
**Not:** Marketing claims validation; this is engineering UX / operational friction

---

## 0. Channel Map (what actually exists)

| Channel | Entry | Runtime | Install path | Notes |
|---------|-------|---------|--------------|-------|
| **Rust binary / Cargo** | `hipcortex` CLI binary, webserver | Local process | GitHub releases, `cargo build` | Crate version **0.5.0** |
| **pip `hipcortex`** | `hipcortex` console script → `cli.py` | Python + optional downloaded binary | `pip install hipcortex` then `hipcortex install` | Py version **0.5.0** |
| **npm `hipcortex`** | TS client `dist/` | HTTP to server | `npm install hipcortex` | **No** install wizard; client-only |
| **VS Code / Antigravity VSIX** | `farmountain.hipcortex-memory` | Extension + **own** auto-started binary | Marketplace / local `.vsix` | Ext version **0.5.4**; expects server **0.5.0** |
| **MCP server** | `sdk/mcp/server.py` stdio JSON-RPC | Python proxy → HTTP | Wizard copies to `~/.hipcortex-mcp/` | `serverInfo.version` still **0.2.0** |
| **Claude Code** | SKILL.md under `~/.claude/skills/hipcortex/` | Skill text + optional MCP | `_install_claude_code` native | Proactive mode writes harness |
| **Cursor / Windsurf / Cline / Roo** | MCP JSON configs | MCP → Python → HTTP | Wizard `type: mcp` | Different config file layouts |
| **VS Code via pip wizard** | `settings.json` MCP | MCP only | `_install_vscode` | **Parallel** to VSIX path |
| **LangChain / CrewAI / AutoGen / LlamaIndex / DSPy / Pydantic** | Generated starter files + package adapters | HTTP client | Wizard `type: framework` | `examples/adapters/` + `hipcortex.adapters` |
| **n8n / Flowise** | curl snippets / guides | HTTP | Wizard partial | Thin |
| **Guide-only** | Continue, Copilot, Codex, Aider, Gemini, Amazon Q | Docs links | No auto-config | High abandon risk |
| **Managed tier** | `hipcortex install --url …` | Remote HTTP | Optional | Fly URL in README |
| **Hermes / OpenClaw / Grok Build / ITU** | **Marketing only** in README | **No** install registry entries | None | Promise > product |
| **OpenManus** | Rust integration layer | In-process | Code API | Not a consumer install channel |

**GitNexus anchors:**  
`doAutoStartServer` / `activate` (OnSave→Request, Activate→HealthCheck); MCP `handle_get_live_beliefs` / `dispatch_tool`; CLI `_build_agent_registry` / `_run_wizard` / `_install_*`.

---

## 1. Cross-Cutting Problems (all channels)

### C1. Version polyglot / identity crisis
- **Symptom:** User cannot answer “what version am I running?”
- **Evidence:** Cargo/py/npm **0.5.0** vs VSIX **0.5.4** vs MCP banner **0.2.0** vs README still cites **vsix 0.3.0** in places; `/health.version` = `CARGO_PKG_VERSION`.
- **Impact:** Extension `strictServerVersion` / major.minor policy vs live **0.4.9** server → kill/restart loops; support tickets.

### C2. Multiple “setup wizards” without a single system of record
- **Symptom:** Re-run `hipcortex install`, open VSIX, enable MCP, paste npm sample — each mutates different files.
- **Evidence:** `cli.py` interactive `_run_wizard` + non-interactive `--yes`; VS Code extension first-run autostart; MCP install separate; framework files written to **cwd**.
- **Impact:** Duplicate MCP entries, conflicting ports, skill files overwritten, unknown which path “won.”

### C3. Dual (or triple) server lifecycle owners
- **Symptom:** Port 3030 already in use / wrong binary / two processes.
- **Evidence:** CLI downloads binary + PID file under `~/.hipcortex`; extension `doAutoStartServer` + `killProcessOnPort` + `globalServerProcess`; managed URL skips local binary inconsistently.
- **Impact:** Race on activate/save (GitNexus OnSave→autostart); data dir split (`~/.hipcortex` vs `~/.hipcortex-vscode`).

### C4. API surface fragmentation
- **Symptom:** Tool works in MCP but not LM tools / SDK / vice versa.
- **Evidence:** MCP tools: add/search/forget/stats/search_code/link/neighbors/related/delete/live_beliefs/purge. Extension LM tools: search/health/predict/rollout/graph_search/causal. Python SDK: broader REST. No single capability matrix.
- **Impact:** Framework adapters only expose remember/recall subset; agents underuse world model / reflect.

### C5. Actor / namespace / session identity chaos
- **Symptom:** Memories “disappear” when switching tools.
- **Evidence:** MCP default actor freeform; proactive `--actor`; LangChain `session_id`; extension `vscode-user`; no global project UUID.
- **Impact:** Cross-channel recall fails; GDPR forget wrong scope.

### C6. Docs / marketing ahead of install graph
- **Symptom:** README lists Hermes, OpenClaw, Grok Code, Antigravity as first-class; wizard has no entries.
- **Evidence:** README architecture diagram vs `_build_agent_registry` list.
- **Impact:** Trust erosion; “install does nothing for my tool.”

### C7. Onboarding success is not observable
- **Symptom:** User finishes wizard, still broken.
- **Evidence:** Partial health checks; no post-install `doctor` command that validates binary + HTTP + MCP handshake + skill file + one add/search roundtrip.
- **Impact:** Support load; silent failure on non-interactive CI.

### C8. Token-optimization story vs default agent behavior
- **Symptom:** Claims ~93% savings; agent still dumps full files.
- **Evidence:** Proactive SKILL requires live_beliefs first; conservative mode weaker; no runtime enforcement outside SKILL text; MCP `search_code` needs prior `hipcortex index` (often skipped).
- **Impact:** Product promise not default path.

### C9. Windows / encoding / TUI fragility
- **Symptom:** Wizard unusable in some terminals; logs mojibake.
- **Evidence:** Interactive multi-select with ANSI; PowerShell encoding issues historically; `build.bat` with hard-coded local paths (left unshipped for reason).
- **Impact:** Windows is primary VS Code user base.

### C10. Security / multi-tenant defaults
- **Symptom:** Local server open, no auth by default; api_key optional.
- **Evidence:** Clients pass optional Bearer; auto-start binds local port without clear firewall guidance.
- **Impact:** Shared machines; accidental LAN exposure if bind not localhost-only.

---

## 2. Channel-Specific Problems

### 2.1 Claude Code (SKILL + optional MCP)

| ID | Problem |
|----|---------|
| CC1 | Proactive vs conservative modes not discoverable after first install |
| CC2 | SKILL install can append CLAUDE.md repeatedly or drift from package template |
| CC3 | No MCP tools if user only uses Skill path — live_beliefs only if agent HTTP-calls or uses separate MCP |
| CC4 | Skill cannot force tool order; agents ignore “MUST call first” |
| CC5 | Uninstall incomplete (skill dir vs CLAUDE.md markers) |
| CC6 | Actor from `--actor` not synced with extension `vscode-user` or project id |
| CC7 | `hipcortex index` / search_code graph separate from memory engine — agents confuse GitNexus vs HipCortex memory |

### 2.2 VS Code Extension / Antigravity VSIX

| ID | Problem |
|----|---------|
| VX1 | Version skew ext 0.5.4 vs server 0.5.0 vs old live 0.4.9 → restart policy surprises |
| VX2 | Auto-start on activate **and** on every save (GitNexus) → latency / thrash |
| VX3 | Dual path: VSIX full product vs pip wizard MCP-only for VS Code |
| VX4 | Auto-capture tags now good; many users never open Query Memory / don’t know server must be up |
| VX5 | LM tools require VS Code ≥1.90; silent fallback “not available” |
| VX6 | Bundled server fetch (`ensureServerBinary`) vs `server/` gitignored cache — offline fail opaque |
| VX7 | Marketplace vs local VSIX version matrix (docs still mention 0.3.0) |
| VX8 | Status bar density (WM / loops / tokens) unexplained to new users |
| VX9 | Antigravity called out in README keywords but no dedicated install/profile |

### 2.3 pip install + `hipcortex install` wizard

| ID | Problem |
|----|---------|
| PI1 | **Multiple wizards:** first `pip install` silence, then interactive TUI, then per-agent side effects, then optional server start |
| PI2 | `--yes` configures **all** agents (including guides?) — surprise file writes in projects |
| PI3 | Framework starters always write to **cwd** (`hipcortex_langchain.py`) — pollutes repos, re-run duplicates |
| PI4 | Detection only scans requirements/pyproject for subset of frameworks — misses monorepos/pnpm/poetry lock only |
| PI5 | Binary download + extract platform matrix failures poorly surfaced |
| PI6 | Re-install not idempotent enough (MCP merge works partially; skills overwrite) |
| PI7 | No unified `hipcortex status` / `doctor` / `uninstall --all` |
| PI8 | Managed URL vs local binary mode switch not stored as durable project config |
| PI9 | Wizard TUI (space/enter) fails in dumb terminals / CI without clear fallback |
| PI10 | Package does not pin matching server binary version to pip version |

### 2.4 MCP server (Cursor, Windsurf, Cline, Roo, generic)

| ID | Problem |
|----|---------|
| MCP1 | `serverInfo.version` **0.2.0** stale |
| MCP2 | Tool list missing reflect/predict/rollout/decide parity with REST & extension |
| MCP3 | Extra hop: MCP Python → HTTP → Rust (latency + dual failure modes) |
| MCP4 | Config paths differ per host; wizard misses Antigravity / Grok / Hermes / OpenClaw layouts |
| MCP5 | `search_code` requires separate index step — empty results look like “memory broken” |
| MCP6 | No streaming / progress for long operations |
| MCP7 | Error strings not structured — agents cannot recover |

### 2.5 npm / Node / TypeScript SDK

| ID | Problem |
|----|---------|
| NP1 | No `npx hipcortex install` twin of Python wizard |
| NP2 | Version 0.5.0 lagging extension story |
| NP3 | No first-class MCP package for Node stdio (Python-only path) |
| NP4 | Examples weak vs Python adapters; no Nest/Next helpers |

### 2.6 LangChain / CrewAI / AutoGen / LlamaIndex / DSPy / Pydantic / n8n

| ID | Problem |
|----|---------|
| FW1 | Install generates root files; package already has `langchain_memory` / `adapters.crewai` — dual mental model |
| FW2 | Starters often commented “how to use” not runnable smoke |
| FW3 | No shared session/actor convention across frameworks |
| FW4 | Token savings not wired (no live_beliefs in most adapters) |
| FW5 | Async/sync split inconsistent |
| FW6 | n8n/Make only curl snippet — no credential template |
| FW7 | Framework versions (LangChain 0.1 vs 0.2/0.3) API drift |

### 2.7 Guide-only assistants (Continue, Copilot, Codex, Aider, Gemini, Amazon Q)

| ID | Problem |
|----|---------|
| GD1 | Wizard marks “configured” success for guide-only? or shows URL only — unclear |
| GD2 | Links may 404 / point to generic README anchors |
| GD3 | No automated regression that guide steps still work |

### 2.8 Hermes / OpenClaw / Grok Build / ITU / Antigravity (claimed)

| ID | Problem |
|----|---------|
| CL1 | **No registry installers** despite README first-class mention |
| CL2 | No MCP schema profiles for those hosts’ config formats |
| CL3 | No E2E harness per host |
| CL4 | Users arrive from marketing → dead end |

### 2.9 Server / data plane UX (shared)

| ID | Problem |
|----|---------|
| SV1 | Data directories diverge (`~/.hipcortex` vs `~/.hipcortex-vscode`) |
| SV2 | No migration tool when upgrading crate versions |
| SV3 | Reflect / worldmodel endpoints powerful but undiscoverable in MCP |
| SV4 | Delete/forget partial (e.g. dual-store path gaps historically) |
| SV5 | Health status lacks “ready for agents” checklist (binary, index, auth, disk) |

---

## 3. Reverse Engineering: Root Causes → Fix Themes

| Root cause | Problems | Fix theme |
|------------|----------|-----------|
| **No unified product version / release train** | C1, VX1, MCP1, NP2, PI10 | Single VERSION source of truth; release bot stamps cargo/py/npm/vsix/mcp |
| **No project-level HipCortex config** | C2, C5, PI3, PI8, FW3 | `.hipcortex/config.toml` (url, actor, mode, channels[]) |
| **Multiple process supervisors** | C3, VX2, SV1 | One `hipcortex daemon` / shared lock + clients only attach |
| **Capability matrix not code-generated** | C4, MCP2, FW4 | OpenAPI → MCP tools + SDK methods + docs table |
| **Marketing registry ≠ install registry** | C6, CL1–4, GD* | Registry YAML drives wizard + README; no channel without installer or explicit “guide” badge |
| **No doctor / success criteria** | C7, PI7, VX4 | `hipcortex doctor` post-install gates |
| **Skill text is soft policy** | C8, CC4 | Optional local policy proxy or MCP middleware enforcing order |
| **Platform TUI debt** | C9, PI9 | Non-interactive first; simple prompts fallback |
| **Identity model ad-hoc** | C5, FW3, CC6 | Project UUID + actor aliases |

---

## 4. Desired End-State UX (design target)

1. **One command or one VSIX** gets a working memory loop: server up, identity set, one channel wired, doctor green.  
2. **Re-run install is idempotent** and shows diff of what changed.  
3. **Same actor/project** across Claude Code, VSIX, MCP, LangChain.  
4. **Version line** identical across health, pip, npm, VSIX, MCP banner.  
5. **Channel list** honest: installed / available / guide-only / experimental.  
6. **Frameworks** use package APIs; no obligatory cwd spam (opt-in scaffold).  
7. **Named hosts** (Hermes, OpenClaw, Grok, Antigravity) either get real installers or leave marketing.

---

## 5. Priority Ranking (user pain × frequency)

| P0 (blocker) | P1 (high) | P2 (medium) | P3 (later) |
|--------------|-----------|-------------|------------|
| C1 version train | C4 capability matrix | FW* polish | Guide deep links |
| C2/C3 single config + daemon | C5 identity | C9 TUI | OpenManus consumer docs |
| C7 doctor | C8 enforce/soft-enforce | MCP latency | Marketplace publish automation |
| PI1/PI6/PI7 wizard hygiene | CL1 registry honesty | NP1 npx install | |
| VX1/VX3 dual VS Code path | MCP2/MCP5 | | |

---

## Approval

- **Status:** APPROVED for execution (Phases 0–1 first)
- **Date:** 2026-07-18
- **Scope:** Execute Phase 0 (honesty + matrix) and Phase 1 (version stamp + doctor); Phase 6 hosts pending research update
- **Constraints:** headroom/caveman; surgical commits; keep extension tags/autostart green


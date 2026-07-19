# Cross-Channel UX Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Execute **one phase at a time**; each phase ends green and shippable.

**Goal:** Make HipCortex one coherent product across Claude Code, VS Code/Antigravity VSIX, pip wizard, MCP hosts, npm/Node, and agent frameworks 鈥?eliminate multi-wizard confusion, version skew, dual servers, and capability gaps.

**Architecture:** Introduce a **project/user config + doctor + single daemon attach model**, a **code-generated capability matrix** from OpenAPI, and a **channel registry** driving install UX. Incremental: no big-bang rewrite of Rust memory core in phase 1.

**Tech Stack:** Rust server (existing), Python CLI/SDK/MCP, TypeScript SDK + VS Code extension, OpenAPI (`src/openapi_spec.rs` / existing routes), Jest/pytest/cargo test.

**Spec / catalog:** `docs/superpowers/specs/2026-07-18-cross-channel-ux-problem-catalog.md`

## Global Constraints

- Default local bind: **127.0.0.1 only** unless user opts in.
- Do not break existing `hipcortex install` flags (`--yes`, `--url`, `--mode proactive`, `--actor`) 鈥?add, don鈥檛 remove.
- Extension auto-capture + tags path (GitNexus OnSave) must stay green every phase.
- Version strings: one release source (`VERSION` file or `Cargo.toml` + stamp script).
- Windows + Unix must both pass doctor + install non-interactive.
- No new mandatory cloud dependency.

---

## Phase map

| Phase | Name | Fixes (catalog IDs) | Outcome |
|-------|------|---------------------|---------|
| **0** | Honesty + measurement | C6, CL1, GD1, README | Marketing matches install graph; baseline metrics |
| **1** | Version train + doctor | C1, C7, PI7, VX1, MCP1 | One version; `hipcortex doctor` green path |
| **2** | Unified config + identity | C2, C5, PI3, PI8, FW3, CC6 | `.hipcortex/config.toml` + actor aliases |
| **3** | Single server attach | C3, VX2, SV1 | One daemon; clients attach; shared data dir |
| **4** | Capability matrix | C4, MCP2, FW4, VX5 | OpenAPI 鈫?MCP/SDK/docs parity |
| **5** | Wizard v2 | PI1鈥揚I10, C9 | Idempotent install; no cwd spam by default |
| **6** | Host installers | CL*, Antigravity, Grok, Hermes, OpenClaw | Real or explicit guide |
| **7** | Framework hard packages | FW1鈥揊W7, NP1鈥揘P4 | Package-first adapters; optional scaffold |
| **8** | Policy / token path | C8, CC4, MCP5 | Soft enforcement + index onboarding |

---

### Phase 0: Honesty + measurement (1鈥? days)

**Files:**
- Modify: `README.md`, `sdk/mcp/README.md`, `docs/usage.md`
- Create: `docs/channels.md` (generated later; hand-written first)
- Create: `scripts/channel_matrix.md` or `channels.yaml`

- [x] **Step 0.1:** Author `channels.yaml` listing every channel with `status: native|mcp|framework|guide|claimed|none`.
- [x] **Step 0.2:** README: replace claimed Hermes/OpenClaw/Grok/Antigravity as 鈥渇irst-class鈥?with status badges from yaml; link wizard support table.
- [x] **Step 0.3:** Fix README VSIX version examples (`0.5.4` not `0.3.0`).
- [x] **Step 0.4:** Add `hipcortex channels` CLI command that prints registry (read yaml or hardcode initially).
- [x] **Step 0.5:** Commit: `docs: honest channel matrix and version references`

**Success:** No README channel without matching registry status.

---

### Phase 1: Version train + `hipcortex doctor` (3鈥? days)

**Files:**
- Create: `VERSION` (or use Cargo only) + `scripts/stamp_versions.py`
- Modify: `sdk/mcp/server.py` (`serverInfo.version`)
- Modify: `sdk/python/pyproject.toml`, `sdk/typescript/package.json` (stamp)
- Modify: `vscode-extension` EXPECTED_SERVER_VERSION sync via stamp
- Create: `sdk/python/hipcortex/doctor.py` + CLI subcommand
- Test: `sdk/python/tests/test_doctor.py`

#### Task 1.1 Version stamp

 
 
- [ ] **Step 3:** CI check: fail if MCP banner 鈮?VERSION.
- [ ] **Step 4:** Commit: `chore: unify product version stamping`

#### Task 1.2 Doctor

```text
hipcortex doctor
鈫?checks:
  1. config present?
  2. GET /health status+version
  3. version policy vs EXPECTED
  4. MCP server.py exists if channel mcp
  5. skill path if claude-code
  6. POST add + search roundtrip with actor from config
  7. optional: index present if search_code used
鈫?exit 0 only if critical checks pass
```

 
 
- [ ] **Step 3:** Wire `hipcortex doctor` in `cli.py` argparse.
- [ ] **Step 4:** Document in README 鈥渧erify install鈥?
- [ ] **Step 5:** Commit: `feat(cli): hipcortex doctor post-install verification`

**Success:** Fresh install + doctor = green on Windows/Linux CI.

---

### Phase 2: Unified config + identity (4鈥? days)

**Files:**
- Create: schema for `.hipcortex/config.toml` and `~/.hipcortex/user.toml`
- Modify: `cli.py` install writes config
- Modify: Python client default url/actor from config
- Modify: extension settings sync optional import from `.hipcortex/config.toml`
- Test: config load/merge/precedence

#### Config shape (normative)

```toml
# .hipcortex/config.toml (project)
url = "http://127.0.0.1:3030"
actor = "proj-myapp"
mode = "proactive"          # proactive | conservative
channels = ["claude-code", "vscode-extension"]
data_dir = ""               # empty 鈫?default shared
server_version_policy = "major_minor"  # major_minor | exact | any_healthy

[aliases]
vscode-user = "proj-myapp"
session_id = "proj-myapp"
```

 
 
- [x] **Step 3:** Clients resolve: env `HIPCORTEX_URL` > project config > user config > default.
- [x] **Step 4:** Extension on activate: if project config exists, prefer its url (setting override remains).
- [x] **Step 5:** Framework starters use config actor/url when present.
- [x] **Step 6:** Commit: `feat: project .hipcortex/config.toml as single source of install state`

**Success:** Claude Code, MCP, and LangChain remember under same actor without flags.

---

### Phase 3: Single server attach model — **DONE** (2026-07-18: b596bab, df6d85b)

**Files:**
- Modify: CLI server start (PID file, health, stop)
- Modify: `vscode-extension` `doAutoStartServer` 鈥?attach-first, spawn only if doctor says no server
- Unify data dir default to `~/.hipcortex/data`
- Commands: `hipcortex start|stop|restart|status`

#### Behavior

1. Client needs server 鈫?`GET /health`.
2. If healthy + version acceptable 鈫?attach (no kill).
3. If down 鈫?start **one** supervised process via CLI daemon API or shared spawn with file lock `~/.hipcortex/server.lock`.
4. Extension **must not** kill healthy servers owned by CLI unless policy exact mismatch and user confirmed / strict mode.
5. Reduce OnSave autostart: use shared 鈥渆nsure once per session鈥?(already partially in-flight) + no kill on save path.

 
 
- [x] **Step 3:** Extension attach-first refactor (tests for version policy remain).
- [x] **Step 4:** Migration note for old `~/.hipcortex-vscode` data.
- [x] **Step 5:** Commit: `feat: shared local daemon attach protocol for CLI and extension`

**Success:** Run VSIX + `hipcortex install` + Cursor MCP simultaneously 鈫?**one** server process, one data dir.

---

### Phase 4: Capability matrix generation — **DONE** (2026-07-18: a182d9a, 551eea7, 374059b)

**Files:**
- Source of truth: OpenAPI / route table
- Generate: `docs/capabilities.md`, MCP TOOLS subset tags, SDK method checklist
- Modify: `sdk/mcp/server.py` add missing high-value tools: `reflect`, `predict` (optional flags)
- Modify: TS SDK parity for live_beliefs (Python already has)

#### Matrix columns

`REST | MCP | Python SDK | TS SDK | VS Code LM | VS Code Commands | LangChain | CrewAI`

 
 
- [x] **Step 3:** Python/TS: fill gaps for link/neighbors/delete if missing.
- [x] **Step 4:** Extension: document LM tool vs command differences in usage.md.
- [x] **Step 5:** Commit: `feat: capability matrix and MCP reflect parity`

**Success:** CI fails if new REST route unmarked in matrix without MCP/SDK decision.

---

### Phase 5: Wizard v2 (idempotent install) — **DONE** (2026-07-18: e23f76f, dee598b)

**Files:** `sdk/python/hipcortex/cli.py`, tests

#### Rules

1. **Default non-destructive:** scaffold frameworks only with `--scaffold`.
2. **Idempotent:** re-run prints 鈥渦nchanged / updated / skipped鈥?
3. **Plan mode:** `hipcortex install --dry-run` shows file diffs.
4. **Uninstall:** `hipcortex uninstall --channel X` and `--all`.
5. **TUI optional:** if not TTY, auto `--yes` with safe subset (claude if present, else print doctor).
6. **Persist** choices to `.hipcortex/config.toml`.

 
 
- [x] **Step 3:** Stop writing framework files unless `--scaffold`.
- [x] **Step 4:** `uninstall` for skill/mcp entries.
- [x] **Step 5:** Tests for double-install stability.
- [x] **Step 6:** Commit: `feat(cli): idempotent install v2 with dry-run and uninstall`

**Success:** Run install twice 鈫?no duplicate MCP servers, no extra files without scaffold.

---

### Phase 6: Host installers (Hermes, OpenClaw, Grok, Antigravity) — **PARTIAL DONE** (2026-07-18: installers `3cd4359`; docs Phase 6B)

Per host:

1. Document exact config path + JSON schema.
2. Implement `_install_<host>` or mark `status: guide` only.
3. E2E smoke script (optional CI).
4. README badge auto from registry.

Priority order:

1. **Antigravity** (VSIX profile / marketplace listing alignment)  
2. **Cursor** (already MCP 鈥?harden)  
3. **Grok Build / xAI tooling** (MCP or skill if paths known)  
4. **Hermes / OpenClaw** (community agent configs 鈥?research then implement)

- [x] **Step 3:** Grok left as guide with deep link `docs/hosts/grok-build.md`.
- [x] **Step 4:** Installers commit `3cd4359`; docs commit Phase 6B.


#### Phase 6 status (2026-07-18)

| Host | Status | Evidence |
|------|--------|----------|
| Antigravity | **mcp** installer | `3cd4359` + channels.yaml |
| Hermes | **mcp** installer | `3cd4359` + channels.yaml |
| OpenClaw | **mcp** installer | `3cd4359` + channels.yaml |
| Grok Build / Code | **guide** | `docs/hosts/grok-build.md` (no auto-installer) |

- [x] Antigravity / Hermes / OpenClaw: document path + `_install_*` + uninstall
- [x] Grok: guide with sample MCP JSON (`docs/hosts/`)
- [x] channels.yaml / channels.md / README honesty aligned
- [ ] Grok native MCP installer when config path product-stable (Phase 6c)
- [ ] Optional E2E smoke CI per host
- [x] Commit installers: `feat(cli): install Antigravity Hermes OpenClaw MCP hosts` (`3cd4359`)
- [x] Commit docs: `docs: Phase 6 host installers and channel status`


---

### Phase 7: Framework package-first adapters — **DONE** (2026-07-19: 5f54878, 764691d)

**Files:** `sdk/python/hipcortex/adapters/*`, `langchain_memory.py`, `examples/adapters/*`, TS packages optional

 
 
- [x] **Step 3:** Examples only reference package APIs; install `--scaffold` copies from package templates (not giant inline strings in cli.py long-term 鈥?move templates to `install/templates/`).
- [x] **Step 4:** npm: `npx hipcortex-init` thin wrapper or document Python as canonical installer.
- [x] **Step 5:** Commit: `feat(sdk): package-first adapters with shared identity`

---

### Phase 8: Policy / token path reliability — **DONE** (2026-07-19: a8bcd8e, 274b1b5, 6f17e46)

 
 
- [x] **Step 3:** Doctor check: SKILL.md contains live_beliefs MUST language for proactive.
- [x] **Step 4:** Commit: `feat: proactive index bootstrap and soft harness telemetry`

---

## Cross-phase testing strategy

| Layer | Command |
|-------|---------|
| Rust | `cargo test --no-default-features --features petgraph_backend --lib` |
| Extension | `cd vscode-extension; npx jest --forceExit --testPathPatterns=extension.test` |
| Python | `pytest sdk/python/tests -q` |
| MCP | `pytest sdk/mcp/test_server.py -q` |
| E2E | `hipcortex doctor` + optional `tests/verify_token_optimization.py` |
| GitNexus | `npx gitnexus analyze` after install/daemon changes; impact on `doAutoStartServer`, `activate`, `cmd_install` |

---

## Explicit non-goals (this plan)

- Rewriting core temporal/symbolic/FSM algorithms.
- Forcing all agents through MCP only.
- Cloud-only memory (local-first remains).
- Claiming Hermes/OpenClaw support without installers.

---

## Suggested execution order for agents

1. Phase 0 (docs honesty) 鈥?low risk  
2. Phase 1 (doctor + versions) 鈥?unblocks all UX  
3. Phase 2 (config) 鈥?unblocks identity  
4. Phase 3 (daemon) 鈥?unblocks dual-server hell  
5. Phase 5 (wizard v2) 鈥?can parallelize with 4 after 2  
6. Phase 4 (matrix) 鈥?parallel with 5  
7. Phase 6鈥? 鈥?incremental  

---

## Success metrics (product)

| Metric | Baseline (today) | Target |
|--------|------------------|--------|
| Time to first successful recall (new user) | 15鈥?5 min multi-wizard | **&lt; 5 min** doctor green |
| Concurrent processes on 3030 after VSIX+pip | often 0鈥? fighting | **exactly 1** |
| Version string mismatches across surfaces | 3鈥? | **0** |
| Channels claimed without installer | several | **0** |
| Double install file churn | high | **idempotent** |
| MCP tools vs REST coverage | partial | **matrix 鈮?90% decided** |

---

## Self-review

- Catalog IDs map to phases (C1鈫扨1, C2/C5鈫扨2, C3鈫扨3, C4鈫扨4, PI*鈫扨5, CL*鈫扨6, FW*鈫扨7, C8鈫扨8).  
- No 鈥淭BD implement later鈥?without phase owner.  
- GitNexus-critical symbols (`doAutoStartServer`, install registry) called out in Phase 3/5.  
- Shipability: each phase independently releasable.

---

## Approval

- **Status:** APPROVED for execution (Phases 0鈥? first)
- **Date:** 2026-07-18
- **Scope:** Execute Phase 0 (honesty + matrix) and Phase 1 (version stamp + doctor); Phase 6 partial: mcp installers (3cd4359) + host docs; Grok still guide
- **Constraints:** headroom/caveman; surgical commits; keep extension tags/autostart green


---

## Phase 6 research update (2026-07-18)

Web + docs research (not yet implemented). Use for installer design.

### OpenClaw (status → **mcp** implementable)

| Item | Finding |
|------|---------|
| Config | `~/.openclaw/openclaw.json` (JSON5); env `OPENCLAW_CONFIG_PATH` |
| MCP client registry | `mcp.servers` in OpenClaw config |
| CLI | `openclaw mcp add <name> --command … --arg …` (stdio); also `set`/`doctor`/`probe` |
| UI | Control UI `/settings/mcp` |
| HipCortex install strategy | `openclaw mcp add hipcortex --command python --arg <path-to-mcp-server.py>` with env `HIPCORTEX_URL`, OR write `mcp.servers.hipcortex` stdio block into openclaw.json |
| Note | OpenClaw can also **serve** as MCP; we need OpenClaw as **client** of HipCortex MCP |

### Hermes Agent / Nous (status → **mcp** implementable)

| Item | Finding |
|------|---------|
| Config | `~/.hermes/config.yaml` (+ `~/.hermes/.env` for secrets) |
| MCP key | `mcp_servers:` map; each entry `command` + `args` (+ `env`) |
| CLI | `hermes mcp` manage; tools discovered at startup via `discover_mcp_tools()` |
| Example | `mcp_servers: hipcortex: { command: python, args: [path], env: { HIPCORTEX_URL: ... } }` |
| HipCortex strategy | Merge yaml under `mcp_servers.hipcortex`; document `hermes mcp` probe |

### Grok Build / xAI (status → **mcp** / **guide**)

| Item | Finding |
|------|---------|
| Product | Grok Build CLI (beta 2026); npm install; SuperGrok / X Premium+ |
| Compatibility claim | AGENTS.md, plugins, hooks, skills, **MCP servers work out of the box** (x.ai news) |
| Config | Prefer standard project MCP / AGENTS.md patterns (align with Claude Code / Cursor-style); exact path may follow product docs — treat first installer as **MCP stdio** into project or user MCP config when documented |
| HipCortex strategy | Phase 6a: guide with sample MCP JSON; Phase 6b: detect Grok Build config file when path stabilizes |

### Antigravity IDE (status → **mcp** + **vsix**)

| Item | Finding |
|------|---------|
| Nature | VS Code fork; extensions via **Open VSX** not MS marketplace |
| MCP | Built-in **MCP Store**; custom servers via `mcp_config.json` (Antigravity docs) |
| VSIX | Import VS Code extensions / Open VSX publish path for hipcortex-memory |
| HipCortex strategy | (1) Document Open VSX publish; (2) `_install_antigravity` writes `mcp_config.json` same stdio as Cursor; (3) optional VSIX install instructions for Antigravity |

### Installer priority (updated)

1. **Antigravity** — `mcp_config.json` + Open VSX note (highest overlap with VSIX)  
2. **Hermes** — `~/.hermes/config.yaml` `mcp_servers` merge  
3. **OpenClaw** — `openclaw mcp add` or `~/.openclaw/openclaw.json` `mcp.servers`  
4. **Grok Build** — guide first; MCP sample; native when config path confirmed  

### channels.yaml status updates — **applied** (Phase 6B docs)

- `antigravity`: claimed → **mcp** (installer `3cd4359`; VSIX note retained)  
- `hermes`: claimed → **mcp**  
- `openclaw`: claimed → **mcp**  
- `grok-code` / `grok-build`: claimed → **guide** (`docs/hosts/grok-build.md`) until path confirmed, then **mcp**







## Plan completion

**Phases 0–8 complete** on main as of 2026-07-19 (through Phase 8 policy/token path). Remaining optional: Grok native installer (6c), host E2E CI.


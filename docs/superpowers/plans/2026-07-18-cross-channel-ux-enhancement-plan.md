# Cross-Channel UX Enhancement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Execute **one phase at a time**; each phase ends green and shippable.

**Goal:** Make HipCortex one coherent product across Claude Code, VS Code/Antigravity VSIX, pip wizard, MCP hosts, npm/Node, and agent frameworks — eliminate multi-wizard confusion, version skew, dual servers, and capability gaps.

**Architecture:** Introduce a **project/user config + doctor + single daemon attach model**, a **code-generated capability matrix** from OpenAPI, and a **channel registry** driving install UX. Incremental: no big-bang rewrite of Rust memory core in phase 1.

**Tech Stack:** Rust server (existing), Python CLI/SDK/MCP, TypeScript SDK + VS Code extension, OpenAPI (`src/openapi_spec.rs` / existing routes), Jest/pytest/cargo test.

**Spec / catalog:** `docs/superpowers/specs/2026-07-18-cross-channel-ux-problem-catalog.md`

## Global Constraints

- Default local bind: **127.0.0.1 only** unless user opts in.
- Do not break existing `hipcortex install` flags (`--yes`, `--url`, `--mode proactive`, `--actor`) — add, don’t remove.
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
| **4** | Capability matrix | C4, MCP2, FW4, VX5 | OpenAPI → MCP/SDK/docs parity |
| **5** | Wizard v2 | PI1–PI10, C9 | Idempotent install; no cwd spam by default |
| **6** | Host installers | CL*, Antigravity, Grok, Hermes, OpenClaw | Real or explicit guide |
| **7** | Framework hard packages | FW1–FW7, NP1–NP4 | Package-first adapters; optional scaffold |
| **8** | Policy / token path | C8, CC4, MCP5 | Soft enforcement + index onboarding |

---

### Phase 0: Honesty + measurement (1–2 days)

**Files:**
- Modify: `README.md`, `sdk/mcp/README.md`, `docs/usage.md`
- Create: `docs/channels.md` (generated later; hand-written first)
- Create: `scripts/channel_matrix.md` or `channels.yaml`

- [x] **Step 0.1:** Author `channels.yaml` listing every channel with `status: native|mcp|framework|guide|claimed|none`.
- [x] **Step 0.2:** README: replace claimed Hermes/OpenClaw/Grok/Antigravity as “first-class” with status badges from yaml; link wizard support table.
- [x] **Step 0.3:** Fix README VSIX version examples (`0.5.4` not `0.3.0`).
- [x] **Step 0.4:** Add `hipcortex channels` CLI command that prints registry (read yaml or hardcode initially).
- [x] **Step 0.5:** Commit: `docs: honest channel matrix and version references`

**Success:** No README channel without matching registry status.

---

### Phase 1: Version train + `hipcortex doctor` (3–5 days)

**Files:**
- Create: `VERSION` (or use Cargo only) + `scripts/stamp_versions.py`
- Modify: `sdk/mcp/server.py` (`serverInfo.version`)
- Modify: `sdk/python/pyproject.toml`, `sdk/typescript/package.json` (stamp)
- Modify: `vscode-extension` EXPECTED_SERVER_VERSION sync via stamp
- Create: `sdk/python/hipcortex/doctor.py` + CLI subcommand
- Test: `sdk/python/tests/test_doctor.py`

#### Task 1.1 Version stamp

- [ ] **Step 1:** Single source `VERSION` = crate version (e.g. `0.5.0`); extension may use `VERSION` + build metadata `0.5.0+ext.N` **or** keep ext semver but stamp `EXPECTED_SERVER_VERSION` from VERSION automatically in CI.
- [ ] **Step 2:** Script updates: Cargo.toml (manual gate), pyproject, npm package.json, MCP serverInfo, openapi info.version.
- [ ] **Step 3:** CI check: fail if MCP banner ≠ VERSION.
- [ ] **Step 4:** Commit: `chore: unify product version stamping`

#### Task 1.2 Doctor

```text
hipcortex doctor
→ checks:
  1. config present?
  2. GET /health status+version
  3. version policy vs EXPECTED
  4. MCP server.py exists if channel mcp
  5. skill path if claude-code
  6. POST add + search roundtrip with actor from config
  7. optional: index present if search_code used
→ exit 0 only if critical checks pass
```

- [ ] **Step 1:** Failing tests for doctor module.
- [ ] **Step 2:** Implement checks (mock HTTP in unit tests).
- [ ] **Step 3:** Wire `hipcortex doctor` in `cli.py` argparse.
- [ ] **Step 4:** Document in README “verify install”.
- [ ] **Step 5:** Commit: `feat(cli): hipcortex doctor post-install verification`

**Success:** Fresh install + doctor = green on Windows/Linux CI.

---

### Phase 2: Unified config + identity (4–6 days)

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
data_dir = ""               # empty → default shared
server_version_policy = "major_minor"  # major_minor | exact | any_healthy

[aliases]
vscode-user = "proj-myapp"
session_id = "proj-myapp"
```

- [ ] **Step 1:** Spec + serde/toml loaders in Python; JSON mirror for Node later.
- [ ] **Step 2:** Install writes project config; global user defaults in `~/.hipcortex/user.toml`.
- [ ] **Step 3:** Clients resolve: env `HIPCORTEX_URL` > project config > user config > default.
- [ ] **Step 4:** Extension on activate: if project config exists, prefer its url (setting override remains).
- [ ] **Step 5:** Framework starters use config actor/url when present.
- [ ] **Step 6:** Commit: `feat: project .hipcortex/config.toml as single source of install state`

**Success:** Claude Code, MCP, and LangChain remember under same actor without flags.

---

### Phase 3: Single server attach model (5–8 days)

**Files:**
- Modify: CLI server start (PID file, health, stop)
- Modify: `vscode-extension` `doAutoStartServer` — attach-first, spawn only if doctor says no server
- Unify data dir default to `~/.hipcortex/data`
- Commands: `hipcortex start|stop|restart|status`

#### Behavior

1. Client needs server → `GET /health`.
2. If healthy + version acceptable → attach (no kill).
3. If down → start **one** supervised process via CLI daemon API or shared spawn with file lock `~/.hipcortex/server.lock`.
4. Extension **must not** kill healthy servers owned by CLI unless policy exact mismatch and user confirmed / strict mode.
5. Reduce OnSave autostart: use shared “ensure once per session” (already partially in-flight) + no kill on save path.

- [ ] **Step 1:** File lock + PID protocol documented.
- [ ] **Step 2:** CLI `start/stop/status`.
- [ ] **Step 3:** Extension attach-first refactor (tests for version policy remain).
- [ ] **Step 4:** Migration note for old `~/.hipcortex-vscode` data.
- [ ] **Step 5:** Commit: `feat: shared local daemon attach protocol for CLI and extension`

**Success:** Run VSIX + `hipcortex install` + Cursor MCP simultaneously → **one** server process, one data dir.

---

### Phase 4: Capability matrix generation (4–6 days)

**Files:**
- Source of truth: OpenAPI / route table
- Generate: `docs/capabilities.md`, MCP TOOLS subset tags, SDK method checklist
- Modify: `sdk/mcp/server.py` add missing high-value tools: `reflect`, `predict` (optional flags)
- Modify: TS SDK parity for live_beliefs (Python already has)

#### Matrix columns

`REST | MCP | Python SDK | TS SDK | VS Code LM | VS Code Commands | LangChain | CrewAI`

- [ ] **Step 1:** Script dumps routes → markdown table in CI.
- [ ] **Step 2:** MCP: add `reflect` + ensure `get_live_beliefs` description matches SKILL.
- [ ] **Step 3:** Python/TS: fill gaps for link/neighbors/delete if missing.
- [ ] **Step 4:** Extension: document LM tool vs command differences in usage.md.
- [ ] **Step 5:** Commit: `feat: capability matrix and MCP reflect parity`

**Success:** CI fails if new REST route unmarked in matrix without MCP/SDK decision.

---

### Phase 5: Wizard v2 (idempotent install) (5–7 days)

**Files:** `sdk/python/hipcortex/cli.py`, tests

#### Rules

1. **Default non-destructive:** scaffold frameworks only with `--scaffold`.
2. **Idempotent:** re-run prints “unchanged / updated / skipped”.
3. **Plan mode:** `hipcortex install --dry-run` shows file diffs.
4. **Uninstall:** `hipcortex uninstall --channel X` and `--all`.
5. **TUI optional:** if not TTY, auto `--yes` with safe subset (claude if present, else print doctor).
6. **Persist** choices to `.hipcortex/config.toml`.

- [ ] **Step 1:** Refactor registry to data-driven `channels.yaml`.
- [ ] **Step 2:** Implement dry-run + idempotent writers (MCP merge already partial).
- [ ] **Step 3:** Stop writing framework files unless `--scaffold`.
- [ ] **Step 4:** `uninstall` for skill/mcp entries.
- [ ] **Step 5:** Tests for double-install stability.
- [ ] **Step 6:** Commit: `feat(cli): idempotent install v2 with dry-run and uninstall`

**Success:** Run install twice → no duplicate MCP servers, no extra files without scaffold.

---

### Phase 6: Host installers (Hermes, OpenClaw, Grok, Antigravity, …) (ongoing)

Per host:

1. Document exact config path + JSON schema.
2. Implement `_install_<host>` or mark `status: guide` only.
3. E2E smoke script (optional CI).
4. README badge auto from registry.

Priority order:

1. **Antigravity** (VSIX profile / marketplace listing alignment)  
2. **Cursor** (already MCP — harden)  
3. **Grok Build / xAI tooling** (MCP or skill if paths known)  
4. **Hermes / OpenClaw** (community agent configs — research then implement)

- [ ] **Step 1:** Research config paths → `channels.yaml`.
- [ ] **Step 2:** Implement top 2 missing native/mcp installers.
- [ ] **Step 3:** Leave others as guide with working deep links.
- [ ] **Step 4:** Commit per host: `feat(cli): install support for <host>`

---

### Phase 7: Framework package-first adapters (4–6 days)

**Files:** `sdk/python/hipcortex/adapters/*`, `langchain_memory.py`, `examples/adapters/*`, TS packages optional

- [ ] **Step 1:** Adapters call `live_beliefs` optionally for context bootstrap.
- [ ] **Step 2:** Standard `session_id` ← config actor.
- [ ] **Step 3:** Examples only reference package APIs; install `--scaffold` copies from package templates (not giant inline strings in cli.py long-term — move templates to `install/templates/`).
- [ ] **Step 4:** npm: `npx hipcortex-init` thin wrapper or document Python as canonical installer.
- [ ] **Step 5:** Commit: `feat(sdk): package-first adapters with shared identity`

---

### Phase 8: Policy / token path reliability (3–5 days)

- [ ] **Step 1:** On proactive install, run `hipcortex index` if repo detected (opt-in flag default true for proactive).
- [ ] **Step 2:** MCP middleware option: warn when `search_memory` called without prior `get_live_beliefs` in same session (metric/log, not hard block v1).
- [ ] **Step 3:** Doctor check: SKILL.md contains live_beliefs MUST language for proactive.
- [ ] **Step 4:** Commit: `feat: proactive index bootstrap and soft harness telemetry`

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

1. Phase 0 (docs honesty) — low risk  
2. Phase 1 (doctor + versions) — unblocks all UX  
3. Phase 2 (config) — unblocks identity  
4. Phase 3 (daemon) — unblocks dual-server hell  
5. Phase 5 (wizard v2) — can parallelize with 4 after 2  
6. Phase 4 (matrix) — parallel with 5  
7. Phase 6–8 — incremental  

---

## Success metrics (product)

| Metric | Baseline (today) | Target |
|--------|------------------|--------|
| Time to first successful recall (new user) | 15–45 min multi-wizard | **&lt; 5 min** doctor green |
| Concurrent processes on 3030 after VSIX+pip | often 0–2 fighting | **exactly 1** |
| Version string mismatches across surfaces | 3–4 | **0** |
| Channels claimed without installer | several | **0** |
| Double install file churn | high | **idempotent** |
| MCP tools vs REST coverage | partial | **matrix ≥ 90% decided** |

---

## Self-review

- Catalog IDs map to phases (C1→P1, C2/C5→P2, C3→P3, C4→P4, PI*→P5, CL*→P6, FW*→P7, C8→P8).  
- No “TBD implement later” without phase owner.  
- GitNexus-critical symbols (`doAutoStartServer`, install registry) called out in Phase 3/5.  
- Shipability: each phase independently releasable.

---

## Approval

- **Status:** APPROVED for execution (Phases 0–1 first)
- **Date:** 2026-07-18
- **Scope:** Execute Phase 0 (honesty + matrix) and Phase 1 (version stamp + doctor); Phase 6 hosts pending research update
- **Constraints:** headroom/caveman; surgical commits; keep extension tags/autostart green


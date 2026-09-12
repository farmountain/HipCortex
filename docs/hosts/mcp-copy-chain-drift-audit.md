# MCP copy-chain drift — fact check of the external handover

**Audited:** 2026-09-12, HEAD `42bec97` (working tree clean apart from `.claude/settings.local.json`)
**Subject:** `handover-hipcortex-mcp-server-drift.md` — 14 798 bytes, 310 lines, mtime 2026-09-12 12:45:20,
written by a read-only investigation run from `PvsNP-Mathematics` (`docs/handover-hipcortex-mcp-server-drift.md`).
Its own header says it was **measured at 12:44**.

**Purpose of this document.** The handover names nine gaps and recommends a fix order. Before any of them is
implemented, each claim is checked against the artifact it describes. This document records what is a **verified
fact**, what is **already closed**, and what is **non-fact context** — historical measurement, derived arithmetic,
or inference.

**Nothing was modified to produce this audit.** No config was repointed, no process was restarted, no package was
installed or removed. The live server on `:3030` was not disturbed.

---

## 1. The copy lattice is bigger than three

The handover says "three artifacts that must be byte-identical". There are **six** places the same file is
referenced, in **four** distinct reference styles:

| # | Artifact | lines | sha256[:16] | mtime | `def _req` | `_req` call sites | `TOOLS` entries | git |
|---|---|---|---|---|---|---|---|---|
| 1 | `sdk/mcp/server.py` (canonical) | 1988 | `87e0147072f3764b` | 09-12 13:45:18 | 1 | 17 | 62 (61 unique) | clean, HEAD `17055a0` |
| 2 | `sdk/python/hipcortex/install/mcp_server.py` (PyPI bundle) | 1988 | `87e0147072f3764b` | 09-12 13:49:23 | 1 | 17 | 62 (61 unique) | clean, HEAD `21c487b` |
| 3 | `sdk/python/build/lib/hipcortex/install/mcp_server.py` | 1781 | `912fa34980508106` | 09-06 17:59:11 | 0 | 17 | 61 | **gitignored** (`.gitignore:77`) |
| 4 | `~/.hipcortex-mcp/server.py` | **794** | `6d99eaebf197cbfb` | **07-20 22:40:19** | 0 | **0** | **18** | untracked (host-local) |
| 5 | VS Code registry → `site-packages/hipcortex/install/mcp_server.py` | — | — | — | — | — | — | **does not exist** |
| 6 | `.claude/mcp.json` → `sdk/mcp/server.py` (bare **relative** path) | — | — | — | — | — | — | in-repo |

(1) and (2) are byte-identical and `python scripts/stamp_versions.py --mcp --check` exits **0**
("MCP bundle already in sync"). Copy (4) is a strict prefix-subset: its 18 tool names are exactly the first 18 of
the canonical's 62, and **none** of the 17 `_req`-backed handlers exist in it.

**Configs that reference each copy** (enumerated, not sampled):

| Copy | Configs |
|---|---|
| (4) `~/.hipcortex-mcp/server.py` | `d:\all_projects\.cursor\mcp.json`, `HipCortex\.cursor\mcp.json`, `HipCortex\.cline\mcp.json`, `HipCortex\.roo\mcp.json`, `~/.grok/config.toml` — **five**, plus `_install_mcp_server()` writes it |
| (1) `sdk/mcp/server.py` | `HipCortex\.mcp.json`, `Kakeya_Abstraction_Reasoning_Action_Model\.vscode\mcp.json`, `HipCortex\.claude\mcp.json` (relative) — **three** |
| (5) site-packages | `AppData\Roaming\Code\User\mcp.json` — **one** |

---

## 2. Gap-by-gap verdict

### G1 — "5 workflows, none invokes `--mcp --check`" — **PARTLY TRUE, and the conclusion is now wrong**

* Workflow count **5** is correct: `ci.yml`, `cognitive_os_token_verification.yml`, `publish-npm.yml`,
  `publish-pypi.yml`, `release.yml`.
* "None references `stamp_versions`" is **false as of now**. `publish-pypi.yml` contains
  `run: python scripts/stamp_versions.py --mcp --check` as the step *"Check the bundled MCP server matches the
  canonical copy"*, added by `b55ccd4` (2026-09-12 **13:50:44**) — 66 minutes after the handover's measurement.
  The handover's own recommended fix for `publish-pypi.yml` is already implemented.
* `ci.yml` genuinely has **no** `--mcp` / `stamp_versions` reference. **However** `ci.yml:264` runs
  `pytest sdk/python/tests/ -q` (added by the same `b55ccd4`), and that directory contains
  `test_mcp_server_req.py`, whose `test_bundled_mirror_is_byte_identical_to_the_canonical_server` asserts
  `_BUNDLED.read_bytes() == _CANONICAL.read_bytes()`. **Drift already fails CI.** An explicit
  `--mcp --check` step would be belt-and-braces with a better error message, not the missing gate.
* Severity "blocker" is therefore **not** supported by the current tree.

### G2 — "the sync is uncommitted (bundle dirty; HEAD still `395776d`)" — **CLOSED**

`sdk/python/hipcortex/install/mcp_server.py` is **clean** and was committed by
`21c487b` — *"fix(sdk): resync bundled MCP server copy so `_req` is defined"* — at 2026-09-12 **12:58:03**,
**14 minutes after** the handover measured. The claim was true when written; the fix landed before the handover
was read here. `git log -S "def _req" -- sdk/python/hipcortex/install/mcp_server.py` returns `21c487b` alone.

### G3 — "`~/.hipcortex-mcp/server.py` is ~7 weeks stale (794 vs 1988 lines, zero `_req`); 3 configs point at it" — **TRUE, and undercounted**

* 794 lines vs 1988 ✓. `def _req` = 0 and `_req(` call sites = **0** ✓. TOOLS = 18 vs 62 ✓.
* mtime 2026-07-20 22:40:19 → 54 days ≈ 7.7 weeks ✓ (derived, arithmetically correct).
* "3 configs point at it" — measured **five**: the three named in the handover's own table resolve to two files
  plus the installer, and `.cline/mcp.json` / `.roo/mcp.json` / `~/.grok/config.toml` also point at it.

### G4 — "VS Code's registry points at a deleted path" — **TRUE**

`AppData\Roaming\Code\User\mcp.json` → `servers.hipcortex`:
`command` = the WindowsApps Python 3.13 interpreter, `args` =
`...\Python.3.13_qbz5n2kfra8p0\LocalCache\local-packages\Python313\site-packages\hipcortex\install\mcp_server.py`.

That directory **does not exist** (`site-packages` itself does exist and is populated). The claim is exact,
including the interpreter path. The *cause* ("the 3.10.0 editable reinstall at 11:50 removed it") is a
historical assertion that is not re-observable.

### G5 — "4 live processes started before the edit hold pre-fix bytecode" — **TRUE, and now proven rather than inferred**

Measured right now, four processes serve `D:/all_projects/hipcortex/sdk/mcp/server.py`:

| PID | started (local) | launcher |
|---|---|---|
| 27148 | 2026-09-10 08:47 | `python` (bare) |
| 34704 | 2026-09-10 08:47 | uv-managed CPython **3.11** |
| 40992 | 2026-09-12 08:14 | `python` (bare) |
| 41212 | 2026-09-12 08:14 | uv-managed CPython **3.11** |

`def _req` entered `sdk/mcp/server.py` in **`17055a0`, 2026-09-12 11:53:16** — *after* all four start times.
So all four imported a file that had no `_req`: the handover's inference is correct on the evidence, not merely
plausible. (Its stated edit time of "09:35:26" is a file mtime, not the commit; the ordering conclusion is
unchanged either way.)

### G6 — "No test asserts bundle == source; suite invokes 4 of ~44 tools, none of the 17" — **PARTLY TRUE; the headline claim is closed**

* **"No test asserts bundle == source"** — **false as of now**. `sdk/python/tests/test_mcp_server_req.py`
  (committed `6d375c3`, 13:45:31) asserts exactly that, plus `_req` module-level definition, ≥10 call sites,
  and fail-silence on both transport failure and HTTP 500. It is collected by CI.
* **"invokes 4 of ~44 tools"** — the "4" is correct (`sdk/mcp/test_server.py`, 304 lines, 12 tests, with
  call-level tests for `add_memory`, `get_stats`, `reflect`, `predict` only). The denominator is **wrong**:
  the canonical `TOOLS` list has **62 entries / 61 unique** (`forget_actor` is listed twice).
* **"none of the 17"** — correct. None of the four exercised tools is a `_req`-backed handler.
* **The genuine residual**: no test *invokes* every `TOOLS` entry, so a `NameError` in an unexercised handler
  still surfaces as runtime `-32000` rather than a red test. This half of G6 is real and open.

### G7 — "Spec H2 says 6 handlers; actual is 17" — **TRUE**

`docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md` (378 lines) says, verbatim:

* `:52` — "**no `def _req` exists**" (it does, at `sdk/mcp/server.py:110`)
* `:197` — "Do not touch the 6 call sites"
* `:320` — WP4 acceptance: "All **6** call sites work against the live server"
* prose — "used by the **6** newest SCM/MGV/report handlers only"

AST analysis of the canonical file finds **17** functions whose body calls `_req`, and the handover's list of 17
names matches the AST result exactly, name for name. The spec's H2 row and WP4 criteria are both stale.

### G8 — "`HipCortex/.mcp.json` uses bare `python`" — **TRUE, and worse than stated**

`HipCortex/.mcp.json` → `"command": "python"`, `"args": ["D:/all_projects/hipcortex/sdk/mcp/server.py"]`.
On this machine `python` resolves to **three** candidates, and the first on `PATH` is
`C:\Users\user\AppData\Local\hermes\hermes-agent\venv\Scripts\python.exe` — unrelated to HipCortex. The two
running bare-`python` servers and the two uv-3.11 servers are the observable consequence. `_desired_mcp_entry()`
(`install_hosts.py:70`) already pins `sys.executable`; the hand-written config does not.

### G9 — "`build/lib` copy stale from 09-06" — **TRUE but benign; its rationale is wrong**

`sdk/python/build/lib/hipcortex/install/mcp_server.py` exists: 1781 lines, no `_req` definition, 61 `TOOLS`,
mtime 2026-09-06 17:59:11. `git check-ignore` confirms it is covered by `.gitignore:77` (`sdk/python/build/`).
The handover's "predating ~7 weeks of changes" is **wrong** — it is **6 days** old, and setuptools regenerates
the directory. The handover's own "likely benign" is the correct conclusion.

---

## 3. Non-fact context in the handover

These are not claims about the current tree; they are history, derived arithmetic, or inference. They must not be
turned into acceptance criteria.

| Item | Why it is not a fact |
|---|---|
| "187.9 minutes" | derived (09:35:26 → 12:43:21); arithmetically consistent with its own inputs, not re-measurable |
| "the 09:35 edit" | a file **mtime** at one moment; the fix actually landed in `17055a0` at 11:53:16. The file's mtime has since changed twice with identical content |
| "98.9 %-identical" | no definition and no reproducible computation; the two revisions this refers to are not both on disk any more |
| "the 12:43:21 sync" | true when measured; both copies now carry mtime 13:45/13:49 with the same bytes |
| "~44 tools" | measured: **62 entries / 61 unique** |
| "three artifacts" / "3 configs point at it" | measured: **six** referenced copies in **four** reference styles; **five** configs point at the stale deployed copy |
| "~1200 lines behind" | correct arithmetic (1988 − 794 = 1194) but a derived figure |
| "7 weeks stale", "predating ~7 weeks" | derived from mtimes. Correct for the deployed copy (54 days), **wrong** for `build/lib` (6 days) |
| "the 3.10.0 editable reinstall at 11:50 removed it" | cause asserted; unverifiable now, though the directory's absence is a fact |
| "4 live processes ... hold pre-fix bytecode" | the 4 PIDs/times are facts; "hold pre-fix bytecode" was inference — now **proven** by `17055a0`'s 11:53:16 timestamp |
| "17 tools dead for 187.9 minutes" | the "17" and the mechanism are facts; the duration is derived and ties to a mtime, not a commit |
| "`~/.hipcortex-mcp` copy is inferred to be installer-written" | the handover flags this itself; correct per `install_hosts.py:483`'s docstring, but not confirmed by re-running the installer |
| §7 kakeya claims (`httpx` fails open, `_TransientError` unreachable, "not a factor") | about **another repository**; nothing here verifies them, and they are out of scope for HipCortex |
| §7 "the backend on 3030 is on-demand, not a supervised service" | inference from one connection-refused probe; `:3030` answered `/health` with `{"service":"hipcortex","status":"ok","version":"3.10.0"}` |

---

## 4. Pre-existing drift the handover did not name

`CLAUDE.md` states the MCP server exposes **"18 tools + 3 resources"**. Measured: `TOOLS` holds **62 entries
(61 unique)** and `RESOURCES` holds **7**. The stated "18" matches the *stale deployed copy*, which suggests the
figure was never updated after the canonical list grew.

Also: `TOOLS` lists `forget_actor` **twice**. Whether that is intentional aliasing or a duplicate entry is not
determined here.

---

## 5. What the index knows about this — and what it does not

`GET /stats` at audit time: **533 records, 533 active, 45 actors**, by type
`Symbolic 312, Temporal 194, Receipt 8, Intent 7, Belief 6, Procedural 3, Reflexion 2, Goal 1`. (It read 509 at
the start of this session; the count moves continuously because passive capture writes as files change.)

`POST /memory/search` was run for six phrasings of this subject. Every hit is a **passive-capture file-edit
record** — `record_type: Symbolic`, `actor: vscode-user`, `source: vscode-auto-capture`, and a `target` of the
form `<filename> (<language>, <n> lines)`. The hits are the *files*: `handover-hipcortex-mcp-server-drift.md`
(310 lines), `test_mcp_server_req.py` (152), `mcp.json` (14), `hc_tools.py` (96), `.tmp_reqcheck.py` (49),
`drift_probe*.py`, plus the file records of `docs/superpowers/plans/2026-09-12-hipcortex-integrity-and-alias-parity-plan.md`.

* **No record holds a prior decision, diagnosis, plan, Belief or Reflexion about copy-chain drift.** The subject
  is known to the index only as a set of files that were edited.
* The single non-capture hit — a `karm` Symbolic record about OpenSpec `092-long-horizon-traversal` closing gap
  C4 — matched on lexical overlap, not on subject.
* The only *conclusions* in the store on this subject are the ones this audit just wrote; the audit document was
  itself captured at 2026-09-12 22:14:53 local, which is the capture mechanism, not knowledge.

Consequence for the question that prompted this audit: there is **no indexed prior context** to reconcile the
handover against. The verification in §1–§4 is against the artifacts themselves, which is the stronger evidence
anyway. Any implementation that cites "the index says" on this subject would be citing file-capture records.

---

## 6. How each figure was measured

Reproducible from a checkout; none of it needs a build.

| Figure | Measured by |
|---|---|
| line counts, mtimes, `sha256[:16]` | `os.stat` / `hashlib` over the six paths |
| `def _req`, `_req` call sites, `TOOLS`, `RESOURCES` | `ast.parse` over the canonical, bundle, `build/lib` and deployed files; call sites are functions whose body contains a `Call` to `_req` |
| config enumeration | walk of `d:\all_projects` (depth ≤3), `~\.cursor`, `~\.grok`, `%APPDATA%\Code\User` for `mcp.json` / `mcp_config.json` / `config.toml`, then filter on `hipcortex` and print the matching lines |
| which commit introduced `_req` | `git log -S "def _req" -- sdk/mcp/server.py` |
| commit timestamps and ancestry | `git log -1 --format=… --date=format:…`; `git merge-base --is-ancestor <c> HEAD` |
| ignored-status of `build/lib` | `git check-ignore -v sdk/python/build/lib/hipcortex/install/mcp_server.py` |
| live processes, start times, command lines | `Get-Process` over `python*`, pid 27148/34704/40992/41212, with creation timestamps |
| the drift gate's health | `python scripts/stamp_versions.py --mcp --check`, exit code captured |
| CI enforcement | job block read out of `.github/workflows/ci.yml` (the `python-sdk` job begins at `:238`); every `pytest` line in every workflow file printed |
| what the index holds | `GET /stats`, `POST /memory/search` ×6 |
| `python` resolution | `Get-Command python -All` |

---

## 7. What is genuinely open

Ordered by whether it is durable or local repair. Nothing here should be started before the numbers above are
accepted, because the handover's own fix order was built on a 12:44 reading of a tree that has since moved.

1. **G6 residual** — a test that invokes every `TOOLS` entry against a stubbed transport, so an unexercised
   handler cannot die at runtime. This is the only *durable* item that is not already closed.
2. **G1 residual** — an explicit `--mcp --check` step in `ci.yml`. Optional: `pytest sdk/python/tests/` already
   fails on drift; the step adds a clearer failure message and covers the case where the Python job is skipped.
3. **G7** — correct the spec's H2 row and WP4 criteria. Prefer "every `_req` call site resolves" over a fixed
   count, so it cannot go stale again.
4. **G8** — pin an absolute interpreter in `HipCortex/.mcp.json`.
5. **G4 → G5 → G3** — local environment repair, in that order: repoint VS Code's registry, then restart the four
   processes, then refresh `~/.hipcortex-mcp/server.py`. Restarting before repointing converts a stale-but-working
   server into no server. All of it touches the user's live setup.
6. **G9** — no action. It is gitignored, regenerated by setuptools, and 6 days old, not 7 weeks.
7. **`CLAUDE.md` counts** — 18/3 is stale; the real figures are 62 (61 unique) and 7.

Two questions cannot be resolved by measurement and are left for the user:

* Whether any of the local repairs (5) should be performed at all — they change the running environment the user
  is actively using, and the handover's caveat that *which* process served the observed calls was never
  determined still stands.
* Whether `forget_actor` appearing twice in `TOOLS` is intended.

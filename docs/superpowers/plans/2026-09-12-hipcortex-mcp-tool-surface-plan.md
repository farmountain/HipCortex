# MCP Tool Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the MCP tool list and the MCP tool handlers agree, and assert that they agree, so the `-32000`-style defect class cannot return.

**Architecture:** Four small, independently testable tasks. Task 1 fixes the contract and adds the rules that can be soundly asserted (`sdk/mcp/server.py` + its bundled mirror). Task 2 adds the rule that generalises the historical `_req` defect, validated against a copy known to still contain it. Task 3 corrects two documents that state the surface wrongly. Task 4 records the change and re-verifies the whole surface.

**Tech Stack:** Python 3.9+ stdlib (`ast`, `importlib`, `pathlib`) + pytest. The MCP server itself uses `requests`; the tests that import it must skip without it.

## Global Constraints

- **Design of record:** `docs/superpowers/specs/2026-09-12-hipcortex-mcp-tool-surface-design.md`. Read §1 and §4 before Task 1.
- **Python floor is 3.9** — `sdk/mcp/server.py`'s docstring states `Python 3.9+ stdlib + requests`. Every test file starts with `from __future__ import annotations` and uses no 3.10+ syntax at runtime.
- **Byte-identity:** `sdk/python/hipcortex/install/mcp_server.py` must stay byte-identical to `sdk/mcp/server.py`. Resync with `python scripts/stamp_versions.py --mcp` from the repo root. `pytest sdk/python/tests/test_mcp_server_req.py::test_bundled_mirror_is_byte_identical_to_the_canonical_server` enforces it.
- **Never edit** `sdk/python/build/lib/**` — it is gitignored (`.gitignore:77`) and regenerated. Task 2 reads it as a fixture only.
- **No version bump.** `VERSION`, `Cargo.toml:3`, `package.json`, `pyproject.toml` all stay `3.10.0`. `vscode-extension/src/extension.ts:459` pins `EXPECTED_SERVER_VERSION = '3.10.0'`.
- **Run tests from the repo root** exactly as CI does (`ci.yml:264`): `pytest sdk/python/tests/ -q`. Do not add `sys.path` manipulation — `pytest.ini` sets `pythonpath = .`.
- **Scratch files go to `$env:TEMP`**, never the repo root. Never write a file with PowerShell `>` (UTF-16).
- **One commit per task.** Commit only the files the task names. Do **not** commit `.claude/settings.local.json`.
- **Do not touch any running process or config**: four MCP processes serve the canonical file; `:3030` is a live server. This plan is offline-only except where it says otherwise.

---

### Task 1: Make the tool contract self-consistent, and assert it

Fixes design defects **D1, D2, D3** and assertions **R1–R4**. The three defects are three instances of one rule
(`every key a handler reads is declared`), so they go green together.

**Files:**
- Create: `sdk/python/tests/test_mcp_tool_surface.py`
- Modify: `sdk/mcp/server.py` (`:212`-`:218` schema entry, `:630`-`:635` duplicate schema entry, `:1073`-`:1077` dead handler, `:1424`-`:1426` live handler, `:1050` `add_memory` schema, `:619`-`:624` `consolidate_memory` schema)
- Modify (by sync, not by hand): `sdk/python/hipcortex/install/mcp_server.py`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `_tools(tree) -> dict[str, list[tuple[list[str], list[str], int]]]` (name → `(required, properties, lineno)` per schema entry), `_handlers(tree) -> dict[str, str]` (tool name → handler function name), `_keys_read(fn) -> set[str]`, `_defs(tree) -> dict[str, ast.AST]`. Task 2 imports `_CANONICAL`, `_BUNDLED`, `_tree`, `_handlers`, `_defs` from this module.

- [ ] **Step 1: Write the failing test**

Create `sdk/python/tests/test_mcp_tool_surface.py`:

```python
"""
Contract tests for the MCP tool surface: TOOLS <-> dispatch_tool <-> handler bodies.

A tool call reaches its handler through a dict lookup in `dispatch_tool`, and the
arguments it receives are whatever the *schema in TOOLS* told the model to send. If
the two disagree, the handler either cannot be reached with the shape the host
advertises, or it reads a key the client was never told about. Both are silent
until a model happens to pick the wrong tool entry.

Four rules, all checked here against `sdk/mcp/server.py`:

  R1  tool names in TOOLS are unique
  R2  every TOOLS name maps to a handler, and every handler name is a module-level def
  R3  every key in a tool's `required` is actually read by its handler
  R4  every key a handler reads is declared in that tool's schema

The inverse of R4 (declared but never read) is deliberately *not* asserted: a handler
may pass `**args` through, and `twin_create` reads its options via
`{k: args[k] for k in ("dim", "dt", "max_covariance")}`. An earlier probe that missed
that pattern produced a false positive, which is why `_keys_read` resolves
subscripts over constant collections and the case is pinned below.

This file asserts the *general* rules. `test_mcp_server_req.py` asserts one specific
instance (`_req` is defined and call sites resolve); the general form of that rule,
`test_every_dispatched_handler_reads_only_names_that_resolve`, is appended to this
file by the next task.
"""
from __future__ import annotations

import ast
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CANONICAL = _REPO_ROOT / "sdk" / "mcp" / "server.py"
_BUNDLED = _REPO_ROOT / "sdk" / "python" / "hipcortex" / "install" / "mcp_server.py"


def _tree(path: Path) -> ast.Module:
    return ast.parse(path.read_text(encoding="utf-8"))


def _literal_str(node: ast.AST):
    return node.value if isinstance(node, ast.Constant) and isinstance(node.value, str) else None


def _const_strs(node: ast.AST) -> list:
    """String literals inside a tuple/list/set literal, or []."""
    if isinstance(node, (ast.Tuple, ast.List, ast.Set)):
        return [e.value for e in node.elts if isinstance(e, ast.Constant) and isinstance(e.value, str)]
    return []


def _tools(tree: ast.Module) -> dict:
    """name -> [(required, properties, lineno), ...] — one entry per TOOLS element."""
    out = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id == "TOOLS" for t in node.targets
        ):
            for elt in node.value.elts:
                if not isinstance(elt, ast.Dict):
                    continue
                kv = {
                    _literal_str(k): v
                    for k, v in zip(elt.keys, elt.values)
                    if _literal_str(k) is not None
                }
                name = _literal_str(kv.get("name", ast.Constant(value=None)))
                schema = kv.get("inputSchema")
                required, properties = [], []
                if isinstance(schema, ast.Dict):
                    for k, v in zip(schema.keys, schema.values):
                        if _literal_str(k) == "required" and isinstance(v, ast.List):
                            required = [e.value for e in v.elts if isinstance(e, ast.Constant)]
                        if _literal_str(k) == "properties" and isinstance(v, ast.Dict):
                            properties = [
                                k2.value for k2 in v.keys if isinstance(k2, ast.Constant)
                            ]
                out.setdefault(name, []).append((required, properties, elt.lineno))
    return out


def _handlers(tree: ast.Module) -> dict:
    """tool name -> handler function name, read out of dispatch_tool's `handlers` dict."""
    out = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.FunctionDef) and node.name == "dispatch_tool":
            for sub in ast.walk(node):
                if isinstance(sub, ast.Assign) and any(
                    isinstance(t, ast.Name) and t.id == "handlers" for t in sub.targets
                ):
                    if isinstance(sub.value, ast.Dict):
                        for k, v in zip(sub.value.keys, sub.value.values):
                            if _literal_str(k) is not None and isinstance(v, ast.Name):
                                out[_literal_str(k)] = v.id
    return out


def _defs(tree: ast.Module) -> dict:
    return {n.name: n for n in tree.body if isinstance(n, ast.FunctionDef)}


def _loop_bound_strings(fn: ast.FunctionDef) -> dict:
    """`for k in ("a", "b")` -> {"k": {"a", "b"}}, so args[k] can be resolved."""
    out = {}
    for node in ast.walk(fn):
        if isinstance(node, (ast.For, ast.comprehension)):
            target = node.target if isinstance(node, ast.For) else node.target
            names = [target] if isinstance(target, ast.Name) else (
                target.elts if isinstance(target, (ast.Tuple, ast.List)) else []
            )
            strs = _const_strs(node.iter)
            if strs:
                for n in names:
                    if isinstance(n, ast.Name):
                        out.setdefault(n.id, set()).update(strs)
    return out


def _keys_read(fn: ast.FunctionDef) -> set:
    """Keys the handler reads from its `args` dict.

    Handles `args["k"]`, `args.get("k")`, and `args[k]` where k is bound by a
    `for k in (...)` in the same function.
    """
    bound = _loop_bound_strings(fn)
    keys = set()
    for node in ast.walk(fn):
        if isinstance(node, ast.Subscript) and isinstance(node.value, ast.Name) and node.value.id == "args":
            literal = _literal_str(node.slice)
            if literal is not None:
                keys.add(literal)
            elif isinstance(node.slice, ast.Name) and node.slice.id in bound:
                keys |= bound[node.slice.id]
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute):
            if node.func.attr == "get" and isinstance(node.func.value, ast.Name):
                if node.func.value.id == "args" and node.args:
                    arg0 = node.args[0]
                    literal = _literal_str(arg0)
                    if literal is not None:
                        keys.add(literal)
                    elif isinstance(arg0, ast.Name) and arg0.id in bound:
                        keys |= bound[arg0.id]
    return keys


def _dispatched(tree: ast.Module):
    """(tools, handlers, defs) for the canonical file, skipping with a clear message if it moved."""
    assert _CANONICAL.exists(), f"{_CANONICAL} is missing"
    tools, handlers, defs = _tools(tree), _handlers(tree), _defs(tree)
    assert tools, "no TOOLS entries parsed — the literal was probably reshaped; fix the parser, not the file"
    assert handlers, "no dispatch_tool handlers parsed — same warning"
    return tools, handlers, defs


# ── R1: one name, one contract ───────────────────────────────────────────────────

def test_tool_names_are_unique():
    tools, _, _ = _dispatched(_tree(_CANONICAL))
    duplicates = {name: [lineno for _, _, lineno in entries] for name, entries in tools.items() if len(entries) > 1}
    assert not duplicates, (
        "TOOLS advertises the same tool name more than once, so a host shows the model "
        "one name with two different argument shapes and only one of them can work: "
        f"{duplicates}"
    )


# ── R2: every advertised tool is reachable ───────────────────────────────────────

def test_every_tool_has_a_dispatch_handler_that_resolves():
    tools, handlers, defs = _dispatched(_tree(_CANONICAL))

    missing = sorted(set(tools) - set(handlers))
    assert not missing, f"advertised in TOOLS but dispatch_tool cannot route them: {missing}"

    unresolved = sorted(f"{name}->{h}" for name, h in handlers.items() if h not in defs)
    assert not unresolved, f"dispatch_tool routes to names that are not module-level defs: {unresolved}"

    unreachable = sorted(set(handlers) - set(tools))
    assert not unreachable, f"handlers exist that TOOLS never advertises: {unreachable}"


# ── R3 and R4: the schema and the handler agree ──────────────────────────────────

def test_required_keys_are_read_and_read_keys_are_declared():
    tools, handlers, defs = _dispatched(_tree(_CANONICAL))

    failures = []
    for name in sorted(tools):
        handler = defs.get(handlers.get(name, ""))
        if handler is None:
            continue
        reads = _keys_read(handler)
        for required, properties, lineno in tools[name]:
            declared = set(required) | set(properties)
            unread = sorted(set(required) - reads)
            undeclared = sorted(reads - declared)
            if unread:
                failures.append(
                    f"{name} (line {lineno}) requires {unread} from the model but "
                    f"{handlers[name]} never reads them"
                )
            if undeclared:
                failures.append(
                    f"{name} (line {lineno}) -> {handlers[name]} reads {undeclared}, "
                    f"which its schema does not declare; declared={sorted(declared)}"
                )
    assert not failures, "tool schema/handler mismatches:\n  " + "\n  ".join(failures)


def test_keys_read_resolves_subscripts_over_constant_collections():
    """The pattern that produced a false positive in the audit must stay supported."""
    src = '''
def handle_twin_create(args: dict) -> str:
    body = {k: args[k] for k in ("dim", "dt", "max_covariance") if k in args}
    return str(body)
'''
    fn = next(n for n in ast.parse(src).body if isinstance(n, ast.FunctionDef))
    assert _keys_read(fn) == {"dim", "dt", "max_covariance"}, (
        "subscript over a tuple literal must resolve, or the R4 check reports a "
        "correct handler as broken"
    )


# ── the D1 break, as executed behaviour rather than as inference ─────────────────

def test_forget_actor_accepts_every_shape_its_schema_advertises():
    """The defect this test file was written for.

    `forget_actor` used to be declared twice — once requiring `actor`, once
    requiring `actor_id` — while the surviving handler read only `args["actor_id"]`,
    so the shape the first entry advertised raised `KeyError: 'actor_id'` and the
    tool call came back to the model as JSON-RPC -32000.
    """
    pytest.importorskip("requests")
    import importlib.util

    spec = importlib.util.spec_from_file_location("hipcortex_mcp_surface_under_test", _CANONICAL)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.HIPCORTEX_URL = "http://127.0.0.1:19998"  # dead port: never reach a live server

    tools, handlers, defs = _dispatched(_tree(_CANONICAL))
    assert "forget_actor" in handlers, "forget_actor must be dispatchable"
    handler_name = handlers["forget_actor"]

    # every shape the schema advertises must bind; a KeyError proves it does not
    for required, properties, _ in tools["forget_actor"]:
        for key in sorted(set(required) | set(properties)):
            payload = {key: "probe-does-not-exist"}
            for other in sorted(set(required) - {key}):
                payload[other] = "probe-does-not-exist"
            try:
                module.dispatch_tool("forget_actor", payload)
            except KeyError as exc:
                pytest.fail(
                    f"schema advertises {key!r} but {handler_name} raised "
                    f"KeyError: {exc} — the model cannot call this tool at all"
                )
            except Exception:
                pass  # transport/HTTP failures are fine: the argument binding succeeded

    # and the error must name what is missing
    with pytest.raises(ValueError):
        module.dispatch_tool("forget_actor", {})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest sdk/python/tests/test_mcp_tool_surface.py -q`
Expected: **3 failed, 2 passed** *(measured; predicted "5 failed, 1 passed")* — `test_keys_read_resolves_subscripts_over_constant_collections` passes (it tests the parser, not the file) and `test_tool_names_are_unique` also passes, because the duplicate `forget_actor` names never reach the `dict` that `_tools` builds. The three failures name exactly: `forget_actor` requires `['actor']` but reads `['actor_id']`; `add_memory` reading `intent_id`; `consolidate_memory` reading `actor`.

- [ ] **Step 3: Delete the dead handler**

In `sdk/mcp/server.py`, remove this definition entirely (lines ~1073-1077), which is unreachable because a later definition shadows it:

```python
def handle_forget_actor(args: dict) -> str:
    actor = args["actor"]
    result = _delete(f"/memory/forget/{actor}")
    deleted  = result.get("records_deleted", 0)
    symbolic = result.get("symbolic_nodes_deleted", 0)
    return f"✓ Deleted {deleted} records and {symbolic} symbolic nodes for '{actor}'."
```

- [ ] **Step 4: Rewrite the surviving handler**

Replace the live definition (line ~1424) with one that reads the declared keys:

```python
def handle_forget_actor(args: dict) -> str:
    # `actor` is the project-wide spelling (dispatch_tool itself defaults on it);
    # `actor_id` is kept because the duplicate schema entry advertised it.
    actor = args.get("actor") or args.get("actor_id")
    if not actor:
        raise ValueError("forget_actor requires 'actor'")
    delta = {"type": "ForgetActor", "actor": actor}
    return json.dumps(_post("/v1/cognitive/transact", {"delta": delta, "actor": "mcp"}))
```

- [ ] **Step 5: Collapse the duplicate schema entries**

Keep **one** entry. Replace the first (line ~212) with the merged contract:

```python
    {
        "name": "forget_actor",
        "description": "GDPR hard-delete all records for an actor via a ForgetActor delta on POST /v1/cognitive/transact.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "actor": {"type": "string", "description": "Actor whose records to delete."},
                "actor_id": {"type": "string", "description": "Legacy alias for actor; accepted for compatibility."},
            },
            "required": ["actor"],
        },
    },
```

and delete the second entry (line ~629-635):

```python
    {
        "name": "forget_actor",
        "description": "GDPR hard-delete all records for an actor via ForgetActor delta.",
        "inputSchema": {
            "type": "object",
            "properties": {"actor_id": {"type": "string"}},
            "required": ["actor_id"],
        },
    },
```

- [ ] **Step 6: Declare the two keys the handlers already read**

In `add_memory`'s `inputSchema.properties` (line ~318), add:

```python
                "intent_id": {"type": "string", "description": "Open ActionIntent this add closes; routes a Temporal add through the Accept-Receipt seam instead of POST /memory/add."},
```

In `consolidate_memory`'s `inputSchema.properties` (line ~621), add:

```python
                "actor": {"type": "string", "default": "mcp", "description": "Actor the AutoConsolidate transaction is attributed to."},
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `pytest sdk/python/tests/test_mcp_tool_surface.py -q`
Expected: **5 passed** *(measured; predicted 6)*. Before the fix, `test_tool_names_are_unique` **failed** on the duplicate, and that is the point: `_tools` accumulates `setdefault(name, []).append(...)`, so a second `forget_actor` entry survives as a second list element and the test names it. Measured against `fa3c234:sdk/mcp/server.py`:
`AssertionError -> ... {'forget_actor': [211, 629]}`. The duplicate was *also* confirmed by execution —
`dispatch_tool('forget_actor', {})` raised `KeyError: 'actor_id'` and the JSON-RPC layer returned `-32000` —
so there were two independent proofs, not one. *(Corrected after measurement; the original text claimed the
opposite. See "Corrections to this plan's own predictions" below.)*

- [ ] **Step 8: Resync the bundled mirror and prove identity**

Run: `python scripts/stamp_versions.py --mcp` then `pytest sdk/python/tests/test_mcp_server_req.py -q`
Expected: `Synced MCP bundle → sdk\python\hipcortex\install\mcp_server.py`, then **5 passed**. Never hand-edit the bundled file.

- [ ] **Step 9: Commit**

```bash
git add sdk/mcp/server.py sdk/python/hipcortex/install/mcp_server.py sdk/python/tests/test_mcp_tool_surface.py
git commit -F "$env:TEMP\msg.txt"   # "fix(mcp): one contract for forget_actor, and a test that keeps it"
```

---

### Task 2: Assert the general form of the `_req` defect

Fixes design **R5**, the G6 residual. This is the durable half: the `_req` incident happened because a handler
read a name nothing defined. `test_mcp_server_req.py` asserts that for `_req`; this asserts it for every
dispatched handler, and proves it can fail by pointing it at the copy that still has the bug.

**Files:**
- Modify: `sdk/python/tests/test_mcp_tool_surface.py` (append the resolution rules at the end)
- Read only (fixture): `sdk/python/build/lib/hipcortex/install/mcp_server.py`

**Interfaces:**
- Consumes: `_CANONICAL`, `_BUNDLED`, `_tree`, `_handlers`, `_defs` defined earlier in the same file (Task 1).
- Produces: `_unresolved_globals(path) -> dict[str, list[str]]`, mapping handler name → the sorted names it reads that resolve at no scope.

These rules live in Task 1's file rather than a sibling module because tests in this package never import one
another (`sdk/python/tests/__init__.py` makes them package members, and the two documented invocations —
`pytest tests/` from `sdk/python` and `pytest sdk/python/tests/` from the repo root — must agree on the import
name). Two commits, one file.

- [ ] **Step 1: Confirm the fixture is still the known-bad input**

Run: `Select-String -Path sdk\python\build\lib\hipcortex\install\mcp_server.py -Pattern "^def _req" -Quiet`
Expected: no output (the fixture has **no** `_req`). If this prints `True`, the build output was regenerated and
Step 5's falsification test must pick a different known-bad copy — stop and say so rather than deleting the
assertion.

- [ ] **Step 2: Write the failing test**

Append to `sdk/python/tests/test_mcp_tool_surface.py`:

```python
"""
Every name a dispatched MCP handler reads must resolve.

This is the general form of the incident that motivated `test_mcp_server_req.py`:
17 handlers called `_req` while nothing defined it, so every one of them raised
`NameError` on first use and the model saw JSON-RPC -32000. That file asserts the
specific case. This one asserts the rule, over all 61 dispatched handlers, so the
next missing name is caught whichever handler it lands in.

Resolution is intentionally conservative. A name counts as resolved if it is:
  * bound at module scope by a def/class/import/assign (including inside a
    module-level if/try/with/for, which is how `_req`'s neighbours are declared);
  * bound inside the function itself (parameter, local, comprehension target,
    lambda parameter, except-as, with-as, import);
  * a builtin.

The check is validated against a copy that *does* have the defect
(`sdk/python/build/lib/.../mcp_server.py`, gitignored build output that predates
`_req`) and the copies that do not. A check that cannot fail is not evidence, so
`test_the_check_reproduces_the_req_defect_on_the_stale_copy` asserts it fails there.
"""
# appended after Task 1's helpers and tests; one new import
import builtins

_STALE_BUILD_COPY = _REPO_ROOT / "sdk" / "python" / "build" / "lib" / "hipcortex" / "install" / "mcp_server.py"

_BUILTINS = set(dir(builtins))


def _module_scope_names(tree: ast.Module) -> set:
    """Names bound at module level, descending into module-level if/try/with/for."""
    names = set()

    def visit(body):
        for node in body:
            if isinstance(node, (ast.Import, ast.ImportFrom)):
                for alias in node.names:
                    names.add((alias.asname or alias.name).split(".")[0])
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                names.add(node.name)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    names |= {n.id for n in ast.walk(target) if isinstance(n, ast.Name)}
            elif isinstance(node, (ast.AnnAssign, ast.AugAssign)) and isinstance(node.target, ast.Name):
                names.add(node.target.id)
            elif isinstance(node, (ast.If, ast.Try, ast.With, ast.For, ast.While)):
                visit(node.body)
                visit(getattr(node, "orelse", []) or [])
                visit(getattr(node, "finalbody", []) or [])
                for handler in getattr(node, "handlers", []) or []:
                    visit(handler.body)

    visit(tree.body)
    return names


def _local_names(fn: ast.FunctionDef) -> set:
    names = {a.arg for a in fn.args.args + fn.args.kwonlyargs + fn.args.posonlyargs}
    if fn.args.vararg:
        names.add(fn.args.vararg.arg)
    if fn.args.kwarg:
        names.add(fn.args.kwarg.arg)
    for node in ast.walk(fn):
        if isinstance(node, ast.Name) and isinstance(node.ctx, (ast.Store, ast.Del)):
            names.add(node.id)
        elif isinstance(node, ast.Lambda):
            for a in node.args.args + node.args.kwonlyargs + node.args.posonlyargs:
                names.add(a.arg)
            if node.args.vararg:
                names.add(node.args.vararg.arg)
            if node.args.kwarg:
                names.add(node.args.kwarg.arg)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            names.add(node.name)
        elif isinstance(node, (ast.Import, ast.ImportFrom)):
            for alias in node.names:
                names.add((alias.asname or alias.name).split(".")[0])
    return names


def _unresolved_globals(path: Path) -> dict:
    """handler name -> sorted names it reads that resolve at no scope."""
    tree = _tree(path)
    module_scope = _module_scope_names(tree)
    defs = _defs(tree)
    out = {}
    for tool, handler_name in _handlers(tree).items():
        fn = defs.get(handler_name)
        if fn is None:
            continue
        known = module_scope | _local_names(fn) | _BUILTINS
        missing = sorted(
            {
                node.id
                for node in ast.walk(fn)
                if isinstance(node, ast.Name)
                and isinstance(node.ctx, ast.Load)
                and node.id not in known
            }
        )
        if missing:
            out[handler_name] = missing
    return out


# ── the rule ────────────────────────────────────────────────────────────────────

def test_every_dispatched_handler_reads_only_names_that_resolve():
    missing = _unresolved_globals(_CANONICAL)
    assert not missing, (
        "an MCP handler reads a name that resolves at no scope, so the tool call "
        f"will raise NameError and reach the model as -32000: {missing}"
    )


def test_the_bundled_copy_also_has_no_unresolved_globals():
    assert _BUNDLED.exists(), f"{_BUNDLED} is missing"
    missing = _unresolved_globals(_BUNDLED)
    assert not missing, f"the artifact a PyPI install runs would fail: {missing}"


def test_the_check_reproduces_the_req_defect_on_the_stale_copy():
    """Falsification: the rule must fail on an input known to violate it."""
    if not _STALE_BUILD_COPY.exists():
        pytest.skip(
            "no stale build copy to validate against; the check is unproven until "
            "one is supplied"
        )
    missing = _unresolved_globals(_STALE_BUILD_COPY)
    assert missing, (
        "the resolution check found nothing wrong with a copy that is known to call "
        "_req without defining it — the check cannot fail and proves nothing"
    )
    assert any("_req" in names for names in missing.values()), (
        f"the _req call sites must be flagged: {missing}"
    )
```

- [ ] **Step 3: Run the test to verify the falsification holds**

Run: `pytest sdk/python/tests/test_mcp_tool_surface.py -q -k "resolves or reproduces"`
Expected: **3 passed**. The third passes precisely because it is pointed at a broken copy — that is the evidence
the check works. If it fails with "found nothing wrong", the check is unsound and Task 2 must not be committed.

- [ ] **Step 4: Run the whole SDK suite to prove nothing regressed**

Run: `pytest sdk/python/tests/ -q`
Expected: all pass, including the 5 in `test_mcp_server_req.py` and the 8 now in `test_mcp_tool_surface.py` *(measured; predicted 9 — the plan miscounted its own five Task-1 tests plus three Task-2 tests as nine)*.

- [ ] **Step 5: Commit**

```bash
git add sdk/python/tests/test_mcp_tool_surface.py
git commit -F "$env:TEMP\msg.txt"   # "test(mcp): assert every dispatched handler's globals resolve"
```

---

### Task 3: Correct the two documents that state the surface wrongly

Fixes **D4** and the design's **G7**-equivalent staleness. Both are copies of the same mistake — a count that was
true once and was never re-derived.

**Files:**
- Modify: `CLAUDE.md` (the MCP section's "18 tools + 3 resources")
- Modify: `docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md:52`, `:197`, `:320`

**Interfaces:**
- Consumes: the measured figures from the design's §1 (`TOOLS` = 62 entries / 61 unique, `RESOURCES` = 7, `_req` call sites = 17).
- Produces: nothing other tasks consume.

- [ ] **Step 1: Re-measure rather than copy**

Run each, so the numbers in the doc are this run's numbers and not a paraphrase:

```powershell
python -c "import ast;s=ast.parse(open('sdk/mcp/server.py',encoding='utf-8').read());import re;print('TOOLS', s.body and len([n for n in ast.walk(s) if isinstance(n,ast.Assign) and any(getattr(t,'id','')=='TOOLS' for t in n.targets)][0].value.elts))"
python -c "import ast;s=ast.parse(open('sdk/mcp/server.py',encoding='utf-8').read());print('RESOURCES', len([n for n in ast.walk(s) if isinstance(n,ast.Assign) and any(getattr(t,'id','')=='RESOURCES' for t in n.targets)][0].value.elts))"
```

Expected: `TOOLS 62` and `RESOURCES 7`. If either differs, use the measured value **and** the unique-name count
(`python -c "..."` over `TOOLS` names), and say which run produced it.

- [ ] **Step 2: Correct `CLAUDE.md`**

Replace the stale "18 tools + 3 resources" claim with the measured figures, and note that the "18" belonged to
the *deployed* copy so a future reader does not re-derive the wrong number from it:

```markdown
**MCP server** (`sdk/mcp/server.py`, version `3.10.0`): exposes 62 tool entries covering 61 unique names + 7
resources. (The older "18 tools + 3 resources" figure described the stale `~/.hipcortex-mcp/server.py` copy, not
this file — see `docs/hosts/mcp-copy-chain-drift-audit.md`.)
```

- [ ] **Step 3: Restate the stale criterion as a resolvable one**

In `docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md`:
`H2`'s row (`:52`) says **"no `def _req` exists"**; `:197` says **"Do not touch the 6 call sites"**; `:320` says
**"All 6 call sites work against the live server"**. Replace each count with the property, which cannot go stale:

```markdown
| **H2** | MCP bridge dead | HIGH | `sdk/mcp/server.py` handlers call `_req(...)`; **no `def _req` exists**. Closed — see below. |
```

```markdown
Do not weaken the `_req` call sites: every handler that calls it must keep calling it, and
`test_mcp_tool_surface.py` asserts the general property for all dispatched handlers.
```

```markdown
Every `_req` call site resolves (17 at the last measurement; the count is not the criterion —
`test_req_is_defined_at_module_level` is).
```

- [ ] **Step 4: Verify no document still carries the stale claims**

Run: `Select-String -Path CLAUDE.md,docs\superpowers\specs\*.md -Pattern '18 tools|the 6 call sites|All 6 call'`
Expected: no matches.

**This gate was scoped too narrowly.** It searched two paths while the same stale claim survived in five
others, so "no matches" proved less than it appeared to. It was corrected after the fact, by the wider sweep
below, and the sibling files were fixed in the same commit. The gate to use instead:

```powershell
Select-String -Path . -Include *.md,*.yaml,*.yml,*.html,*.json,*.toml -Recurse `
  -Pattern '18 tools|45 tools|the 6 call sites|All 6 call|3 resources' |
  Where-Object { $_.Path -notmatch '\\(target|node_modules|\.ua|graphify-out)\\' }
```

Expected: no matches, apart from quotations of the *pre-fix* state (the design's §1 measurement, this plan's
correction notes, and the dated audit under `docs/hosts/`).

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md docs/superpowers/specs/2026-09-12-hipcortex-gap-closure-design.md
git commit -F "$env:TEMP\msg.txt"   # "docs: state the MCP surface's real size, and a criterion that cannot go stale"
```

---

### Task 4: Record the change and re-verify the surface

Closes the design's §6 acceptance criteria as a set, and leaves the repository's own record of the defect.

**Files:**
- Modify: `CHANGELOG.md`

**Interfaces:**
- Consumes: the acceptance criteria in the design's §6.
- Produces: nothing.

- [ ] **Step 1: Run every criterion, in one pass**

```powershell
pytest sdk/python/tests/ -q
pytest sdk/mcp/test_server.py -q
python scripts/stamp_versions.py --mcp --check
git status --short
```

The MCP self-test is named by its path and run through pytest. `python test_server.py`
(exercised from `sdk\mcp` during execution) exits `0` while running no test at all, because
the file is a pytest module and has no `__main__`. The first draft of this gate had that
form, so it reported success without ever evaluating an assertion.

Expected: SDK suite all passed; `MCP bundle already in sync`; `sdk/mcp/test_server.py` all
passed; `git status` shows only `CHANGELOG.md` modified (plus the pre-existing untracked paths).

- [ ] **Step 2: Add the CHANGELOG entry under the existing 3.10.0 section**

```markdown
- **Fix (MCP tool contract):** `forget_actor` was advertised twice with incompatible schemas
  (`required: ["actor"]` and `required: ["actor_id"]`) while the surviving handler read only
  `args["actor_id"]`, so the shape the first entry advertised raised `KeyError` and the tool call
  reached the model as JSON-RPC `-32000`. One entry now declares `actor` with `actor_id` as an
  accepted alias, the shadowed handler and the duplicate schema entry are gone, and `add_memory` /
  `consolidate_memory` now declare the `intent_id` / `actor` keys their handlers already read.
  Two rules keep it that way, both in `test_mcp_tool_surface.py`: schema ↔ handler agreement
  (unique names, reachable handlers) and global resolution (every dispatched handler's globals
  resolve — the general form of the `_req` defect, validated against the stale build copy that
  still reproduces it).
```

- [ ] **Step 3: Verify the four acceptance criteria that are behavioural**

```powershell
cd sdk\mcp
python -c "import importlib.util as u; s=u.spec_from_file_location('m','server.py'); m=u.module_from_spec(s); s.loader.exec_module(m); m.HIPCORTEX_URL='http://127.0.0.1:19998'; print('actor  ->', type((lambda: [m.dispatch_tool('forget_actor',{'actor':'probe'})])()).__name__)"
```

Expected: a `ConnectionError` mentioning port 19998 — **not** a `KeyError`. A `KeyError` means the fix did not
land; a `ConnectionError` is the proof the argument binding succeeded and the call reached the transport.

- [ ] **Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -F "$env:TEMP\msg.txt"   # "docs(changelog): record the MCP tool-contract fix"
```

- [ ] **Step 5: Confirm the tree is clean of task artefacts**

Run: `git status --short; git log --oneline -6`
Expected: no scratch files, no `.claude/settings.local.json`, four commits for this plan plus the two audit commits.

---

## Deferred, with reasons (do not do these here)

- **`handle_p5_consolidate`'s intended Reflexion write** — it passes `memory_type`/`content`/`metadata`, all of
  which `handle_add_memory` ignores, so the "durable Reflexion{consolidation_ratio}" its comment promises is a
  Temporal record with the ratio dropped. Fixing it changes what reaches the store, so it needs its own design.
- **The handover's G3/G4/G5** — stale `~/.hipcortex-mcp/server.py`, a dead VS Code registry path, four pre-fix
  MCP processes. Local environment; repair order G4 → G5 → G3, on the user's instruction only.
- **G8's bare `python`** — a real trap; the repo-side fix is non-portable, and
  `install_hosts.py::_desired_mcp_entry()` is the correct place to pin `sys.executable`.
- **G1's residual** — an explicit `--mcp --check` step in `ci.yml`. Drift already fails CI via
  `pytest sdk/python/tests/`; this only improves the message.
- **Pushing** the local commits — the user's call; `origin/main` is `395776d`.

---

## Execution Record

Filled in as the tasks landed. Where this section disagrees with a prediction above, this section is the
measurement and the prediction was wrong.

| Task | Commit | What actually happened |
|---|---|---|
| 1 — one contract for `forget_actor`, and a test that keeps it | `aa255b5` | RED `3 failed, 2 passed` naming exactly the three designed defects and nothing else — no false positives, no missed reads. Fixed; GREEN `5 passed`. Bundle regenerated by `stamp_versions.py --mcp`; identity asserted; `5 passed`. |
| 2 — every dispatched handler's globals resolve | `dbded9d` | `2 passed, 1 failed` on first run: the agent's own `_module_scope_names` raised `UnboundLocalError`. The accumulator was written `names \|= _bound_names(...)` inside a closure, which makes `names` a local of that closure. Changed to `names.update(...)`. GREEN `3 passed`. |
| 3 — the two documents that state the surface wrongly | `d7f7fe4` | Both corrected. `CLAUDE.md` additionally carried three false `version 0.6.0` claims (every artifact declares 3.10.0) and named only 3 of the 7 resources. |
| 4 — record the change and re-verify | this commit, with `CHANGELOG.md` | The third gate this task named turned out to be unrunnable *and* failing: `python test_server.py` exits `0` having run nothing, and under pytest the file showed `4 failed, 8 passed`. All four are stale expectations against behaviour that legitimately moved on — none is a regression. Repaired, registered in CI, and the gate corrected below. |

### Corrections to this plan's own predictions

Six. Five were corrected above, in place: the RED count (3/2, not 5/1), the GREEN count (5, not 6), the suite count
(8 in `test_mcp_tool_surface.py`, not 9), the tool count (61 entries after the duplicate is removed, not 62),
and — the one that matters — the belief that `test_tool_names_are_unique` had no power over the duplicate.
That belief was wrong, and it was wrong in the safe direction. The reasoning was that `_tools` builds a
`dict`; it does not. It accumulates `setdefault(name, []).append(...)`, so both `forget_actor` entries survive
as two list elements, and the test **fails** on the pre-fix file: pointed at `fa3c234:sdk/mcp/server.py` it
reports `AssertionError -> ... {'forget_actor': [211, 629]}`. The duplicate therefore had two independent
proofs — that assertion, and the execution probe (`dispatch_tool('forget_actor', {})` → `KeyError: 'actor_id'`
→ `-32000`) — not one. Related: the phrasing `len(TOOLS) == len(unique)` is not implementable as written,
because `len(TOOLS)` is **62** with the duplicate and **61** without, while distinct names are **61** in both;
the check that actually went in is per-name entry count, which is the strictly stronger statement.
The sixth correction is Task 4's own verification gate, corrected in place above and explained immediately below.

### What the falsification test actually proved

Pointed at `sdk/python/build/lib/hipcortex/install/mcp_server.py` — the gitignored build output that predates
the `_req` fix — the resolution rule flags **17 handlers, 17 names, all of them `_req`, and nothing else**.
Zero false positives is the meaningful part: the rule is not merely non-empty on a broken input, it is
*precise*. It is clean on both the canonical file and the bundled mirror.

### The third gate was not a gate

Task 4's verification step was written as `cd sdk\mcp; python test_server.py`, and run that way it prints
nothing and exits `0`. `sdk/mcp/test_server.py` is a pytest module — its own docstring says
`run: pytest sdk/mcp/test_server.py -v` — so the script form never collects a test, and "no output, exit 0"
is indistinguishable from every test passing. Run correctly it reported `4 failed, 8 passed`.

None of the four is a regression, and that is measured rather than assumed: the same four fail against
`fa3c234:sdk/mcp/server.py`, the file as it stood before this plan's first commit, staged byte-exact into a
temp tree and run with today's test file. (The first attempt at that measurement was itself invalid — piping
`git show` through PowerShell's `Set-Content` writes a BOM, and the corrupted copy failed 12 of 12. The
re-measurement writes the bytes with Python. A bad measurement looks exactly like a bad result.)

| Test | Why it fails | Disposition |
|---|---|---|
| `test_initialize` | asserts `capabilities == {"tools": {}}`; the server has advertised `resources` since `RESOURCES` was added | derived: `resources` is present exactly when `RESOURCES` is non-empty |
| `test_tools_list` | asserts an exact set of 18 names; `TOOLS` declares 61 — stale by 43 | binds to `{t["name"] for t in TOOLS}`, with the original 18 kept as a floor |
| `test_harness_no_warn_after_live_beliefs` | reads `_live_beliefs_seen`, a boolean that no longer exists | asserts `"_global" in _live_beliefs_seen_actors` |
| `test_harness_no_warn_after_reflect` | same | same |

The last two are the instructive pair. The implementation had deliberately refined
`_live_beliefs_seen: bool` into `_live_beliefs_seen_actors: set`, making the substrate-first discipline
per-actor — a strict improvement that these tests silently stopped observing. Each test created the dead
attribute with an assignment and then asserted the dead attribute was `True`, so the assertion could never
hold and the tests could never say why.

No assertion was weakened to reach green: two were widened by binding to a declaration, and two were
corrected to the mechanism that actually exists.

`_load_server()` also resolved its target through the relative path `"sdk/mcp/server.py"`, so the suite only
worked from the repo root. It now resolves from `__file__`; the suite was verified from `sdk\python`.

**The CI registration is the part that matters most.** No workflow referenced `sdk/mcp/test_server.py`;
`ci.yml` ran `pytest sdk/python/tests/ -q` and nothing else. So the file was green on paper and 4/12 red in
fact, for as long as the tool surface has been growing. It now runs in that same step — which already
carries a comment recording that `sdk/python/tests/` had been excluded for the identical reason. That makes
this the third instance of one defect class inside this plan's blast radius, after the `_req` defect and the
duplicated `forget_actor` schema: a surface that exists only on paper is not a surface, and a gate that
prints nothing has not passed.

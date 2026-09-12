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
            target = node.target
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

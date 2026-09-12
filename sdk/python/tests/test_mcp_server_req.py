"""
Contract tests for the MCP server's arbitrary-method HTTP helper.

`sdk/mcp/server.py` defines `_req`, and 17 handlers call it. It exists because
`_get`/`_post`/`_delete` call `raise_for_status()`, so a handler that used them
would raise out of the tool call — and before `_req` was defined at all, every
one of those 17 handlers died with `NameError` on first use.

Two properties therefore have to hold, and neither had a test:

  * `_req` is *defined* at module level while call sites reference it;
  * `_req` is *fail-silent* — a transport failure or a non-2xx status comes back
    as `{"error": ...}` instead of an exception, so an MCP tool call always
    produces something the model can read.

The bundled copy at `sdk/python/hipcortex/install/mcp_server.py` is the artifact
users actually run on a PyPI install: `install_hosts._install_mcp_server()` falls
back to `importlib.resources` and copies that file out. The two must stay
byte-identical, so that is asserted here rather than trusted.
"""
from __future__ import annotations

import ast
import http.server
import importlib.util
import threading
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CANONICAL = _REPO_ROOT / "sdk" / "mcp" / "server.py"
_BUNDLED = _REPO_ROOT / "sdk" / "python" / "hipcortex" / "install" / "mcp_server.py"

# A port nothing listens on, for the transport-failure case.
_DEAD_URL = "http://127.0.0.1:19999"


def _module_ast(path: Path) -> ast.Module:
    return ast.parse(path.read_text(encoding="utf-8"))


def _module_level_defs(tree: ast.Module) -> set:
    return {
        node.name
        for node in tree.body
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
    }


def _load_server(path: Path):
    """Import a `server.py` by path — it is a script, not a package member.

    Importing it executes only module-level definitions: the entry point is
    guarded by `if __name__ == "__main__"`.
    """
    spec = importlib.util.spec_from_file_location("hipcortex_mcp_server_under_test", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ── the definition exists, and the call sites resolve ────────────────────────────

def test_req_is_defined_at_module_level():
    tree = _module_ast(_CANONICAL)
    assert "_req" in _module_level_defs(tree), (
        "sdk/mcp/server.py must define `_req`; the handlers that call it are "
        "resolved against module scope, so a missing definition is a NameError "
        "in every one of them"
    )


def test_req_has_call_sites_to_serve():
    """A definition with no callers would prove nothing; assert both sides exist."""
    tree = _module_ast(_CANONICAL)
    uses = [
        node
        for node in ast.walk(tree)
        if isinstance(node, ast.Name)
        and node.id == "_req"
        and isinstance(node.ctx, ast.Load)
    ]
    assert len(uses) >= 10, (
        f"expected the MCP tool handlers to call `_req`; found {len(uses)} call sites"
    )
    assert "_req" in _module_level_defs(tree), (
        f"{len(uses)} call sites reference `_req` but nothing defines it"
    )


# ── fail-silent, which is the actual contract ────────────────────────────────────

def test_req_is_fail_silent_when_the_server_is_unreachable():
    pytest.importorskip("requests")
    server = _load_server(_CANONICAL)
    server.HIPCORTEX_URL = _DEAD_URL

    result = server._req("POST", "/v1/definitely-not-a-route", {"a": 1})

    assert isinstance(result, dict), f"_req must return a dict, got {type(result)}"
    assert "error" in result, (
        "a transport failure must be reported in the result, not raised: "
        f"{result}"
    )


def test_req_reports_an_http_error_instead_of_raising():
    pytest.importorskip("requests")

    class _Boom(http.server.BaseHTTPRequestHandler):
        def do_POST(self):  # noqa: N802 — BaseHTTPRequestHandler's spelling
            body = b'{"error": "boom"}'
            self.send_response(500)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *args):  # silence the handler's stderr chatter
            pass

    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Boom)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    try:
        server = _load_server(_CANONICAL)
        server.HIPCORTEX_URL = f"http://127.0.0.1:{httpd.server_address[1]}"

        result = server._req("POST", "/v1/anything", {})

        assert result.get("status") == 500, f"the status must survive: {result}"
        assert "error" in result, f"a non-2xx must be reported, not raised: {result}"
        assert "boom" in result.get("detail", ""), (
            f"the server's body must be surfaced for the model to act on: {result}"
        )
    finally:
        httpd.shutdown()


# ── the shipped artifact is the tested artifact ──────────────────────────────────

def test_bundled_mirror_is_byte_identical_to_the_canonical_server():
    assert _BUNDLED.exists(), (
        f"{_BUNDLED.relative_to(_REPO_ROOT)} is missing: a PyPI install falls back "
        "to it via importlib.resources, so its absence ships a broken MCP tool set"
    )
    assert _BUNDLED.read_bytes() == _CANONICAL.read_bytes(), (
        "the bundled MCP server has drifted from sdk/mcp/server.py; on a PyPI "
        "install the bundled copy *is* the artifact users run. Resync with "
        "`python scripts/stamp_versions.py --mcp`"
    )

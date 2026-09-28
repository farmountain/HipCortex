"""
Loop-Ready Contract (v3.18.0) on the MCP and Python SDK surfaces.

A control loop consumes HipCortex as "snapshot once, then follow the changes":

  * ``GET /snapshot/:actor/transitions?since=N`` pages the settled intents with a
    stamp above ``N`` and reports ``next_cursor``, ``horizon_tx`` and ``truncated``;
  * ``GET /snapshot/:actor/scm`` reads the structural causal model.

Both routes shipped on REST only. These tests pin the MCP tools and client methods
that reach them. They run against a real local HTTP server that records the path it
was asked for, so the URL each surface builds is what is asserted, not a mock call.
"""
from __future__ import annotations

import http.server
import importlib.util
import json
import threading
from contextlib import contextmanager
from pathlib import Path

import pytest

from hipcortex.client import HipCortexClient

_REPO_ROOT = Path(__file__).resolve().parents[3]
_CANONICAL = _REPO_ROOT / "sdk" / "mcp" / "server.py"

# A port nothing listens on, for the transport-failure case.
_DEAD_URL = "http://127.0.0.1:19997"

_PAGE = {
    "transitions": [{"intent_id": "i-1", "settled_tx": 9, "ok": True}],
    "count": 1,
    "next_cursor": 9,
    "horizon_tx": 0,
    "truncated": False,
}
_SCM = {"as_of_tx": 12, "acyclic": True, "nodes": [], "edges": [], "interventions": []}


def _load_server(url: str):
    """Import `server.py` by path — it is a script, not a package member."""
    spec = importlib.util.spec_from_file_location("hipcortex_mcp_loop_ready", _CANONICAL)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.HIPCORTEX_URL = url
    return module


class _Recorder(http.server.BaseHTTPRequestHandler):
    def do_GET(self):  # noqa: N802 — BaseHTTPRequestHandler's spelling
        self.server.seen.append(self.path)
        body = json.dumps(self.server.reply).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):  # silence the handler's stderr chatter
        pass


@contextmanager
def _serving(reply: dict):
    httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Recorder)
    httpd.seen = []
    httpd.reply = reply
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    try:
        yield httpd, f"http://127.0.0.1:{httpd.server_address[1]}"
    finally:
        httpd.shutdown()


# ── MCP: snapshot_transitions ────────────────────────────────────────────────────

def test_transitions_tool_requests_the_page_after_the_cursor():
    with _serving(_PAGE) as (httpd, url):
        result = _load_server(url).dispatch_tool(
            "snapshot_transitions", {"actor": "agent-1", "since": 7}
        )
    assert httpd.seen == ["/snapshot/agent-1/transitions?since=7"]
    assert json.loads(result) == _PAGE, "the page fields must reach the model intact"


def test_transitions_tool_defaults_to_the_mcp_actor_from_the_start():
    """`open_intent` files intents under actor `mcp` when none is given."""
    with _serving(_PAGE) as (httpd, url):
        _load_server(url).dispatch_tool("snapshot_transitions", {})
    assert httpd.seen == ["/snapshot/mcp/transitions?since=0"]


def test_transitions_tool_treats_an_empty_actor_as_the_default():
    """`cognitive_snapshot` advertises `actor` default "" — an empty segment never routes."""
    with _serving(_PAGE) as (httpd, url):
        _load_server(url).dispatch_tool("snapshot_transitions", {"actor": ""})
    assert httpd.seen == ["/snapshot/mcp/transitions?since=0"]


def test_transitions_tool_escapes_the_actor_path_segment():
    """An unescaped `/` adds a path segment, and the route no longer matches."""
    with _serving(_PAGE) as (httpd, url):
        _load_server(url).dispatch_tool("snapshot_transitions", {"actor": "team/a b"})
    assert httpd.seen == ["/snapshot/team%2Fa%20b/transitions?since=0"]


# ── MCP: snapshot_scm ────────────────────────────────────────────────────────────

def test_scm_tool_reads_the_structural_causal_model():
    with _serving(_SCM) as (httpd, url):
        result = _load_server(url).dispatch_tool("snapshot_scm", {})
    assert httpd.seen == ["/snapshot/_global/scm"], "the SCM is global"
    assert json.loads(result) == _SCM


# ── MCP: contract shared by both tools ───────────────────────────────────────────

@pytest.mark.parametrize("tool", ["snapshot_transitions", "snapshot_scm"])
def test_loop_ready_tools_are_fail_silent_when_the_server_is_down(tool):
    result = _load_server(_DEAD_URL).dispatch_tool(tool, {"actor": "agent-1"})
    assert "error" in json.loads(result), (
        f"{tool} must report a transport failure in its result, not raise: {result}"
    )


def test_loop_ready_tools_are_declared_with_their_inputs():
    tools = {t["name"]: t for t in _load_server(_DEAD_URL).TOOLS}
    transitions = tools["snapshot_transitions"]["inputSchema"]["properties"]
    assert transitions["since"]["type"] == "integer"
    assert transitions["actor"]["type"] == "string"
    assert tools["snapshot_scm"]["inputSchema"]["properties"]["actor"]["type"] == "string"


# ── Python SDK ───────────────────────────────────────────────────────────────────

def test_client_pages_transitions_after_the_cursor():
    with _serving(_PAGE) as (httpd, url):
        page = HipCortexClient(base_url=url).snapshot_transitions("agent-1", since=7)
    assert httpd.seen == ["/snapshot/agent-1/transitions?since=7"]
    assert page == _PAGE


def test_client_escapes_the_actor_path_segment():
    with _serving(_PAGE) as (httpd, url):
        HipCortexClient(base_url=url).snapshot_transitions("team/a b")
    assert httpd.seen == ["/snapshot/team%2Fa%20b/transitions?since=0"]


def test_client_reads_the_structural_causal_model():
    with _serving(_SCM) as (httpd, url):
        scm = HipCortexClient(base_url=url).snapshot_scm()
    assert httpd.seen == ["/snapshot/_global/scm"]
    assert scm == _SCM

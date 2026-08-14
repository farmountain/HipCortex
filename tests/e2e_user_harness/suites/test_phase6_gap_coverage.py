"""Profile 0 / MCP resources gap coverage tests."""
import json
import os
import subprocess
import sys
from unittest.mock import MagicMock

PYTHON = sys.executable
SERVER = os.path.join(os.path.dirname(__file__), "../../../sdk/mcp/server.py")


def _send(proc, msg):
    line = json.dumps(msg) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())


def _start_server():
    env = {**os.environ, "HIPCORTEX_URL": "http://localhost:8787", "HIPCORTEX_API_KEY": "test"}
    return subprocess.Popen(
        [PYTHON, SERVER],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        text=True,
        env=env,
    )


def test_mcp_initialize_advertises_resources():
    proc = _start_server()
    try:
        resp = _send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
        caps = resp["result"]["capabilities"]
        assert "resources" in caps, f"capabilities missing 'resources': {caps}"
    finally:
        proc.terminate()


def test_mcp_resources_list_returns_three_resources():
    proc = _start_server()
    try:
        _send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
        resp = _send(proc, {"jsonrpc": "2.0", "id": 2, "method": "resources/list", "params": {}})
        resources = resp["result"]["resources"]
        assert len(resources) == 3, f"expected 3 resources, got {len(resources)}"
        uris = {r["uri"] for r in resources}
        assert "hipcortex://context/relevant" in uris
        assert "hipcortex://beliefs/current" in uris
        assert "hipcortex://context/conversation" in uris
    finally:
        proc.terminate()


def test_mcp_resource_read_returns_content_silently_on_server_error():
    """If HipCortex server unreachable, resource read returns empty content (not error)."""
    proc = _start_server()
    try:
        _send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
        resp = _send(
            proc,
            {
                "jsonrpc": "2.0",
                "id": 2,
                "method": "resources/read",
                "params": {"uri": "hipcortex://context/relevant"},
            },
        )
        assert "result" in resp, f"expected result, got: {resp}"
        assert "contents" in resp["result"]
    finally:
        proc.terminate()


def test_mcp_version_is_0_6_0():
    proc = _start_server()
    try:
        resp = _send(proc, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
        ver = resp["result"]["serverInfo"]["version"]
        assert ver == "0.6.0", f"expected 0.6.0, got {ver}"
    finally:
        proc.terminate()


# ── Profile 0 gate tests (requires live server at HIPCORTEX_URL) ───────────────

import time as _time

_HIPCORTEX_URL = os.environ.get("HIPCORTEX_URL", "http://localhost:8787")
_SKIP_LIVE = not os.environ.get("HIPCORTEX_LIVE_TESTS", "")


def _requires_live(fn):
    import pytest

    return pytest.mark.skipif(_SKIP_LIVE, reason="Set HIPCORTEX_LIVE_TESTS=1 to run live server tests")(fn)


@_requires_live
def test_profile0_langchain_passive_no_explicit_add():
    """Profile 0: LangChain callback captures without any explicit add_memory."""
    import requests
    from hipcortex import HipCortexClient
    from hipcortex.langchain_memory import HipCortexCallbackHandler

    client = HipCortexClient(base_url=_HIPCORTEX_URL)
    actor = f"profile0-lc-{int(_time.time())}"
    handler = HipCortexCallbackHandler(client=client, actor=actor)

    handler.on_llm_start(serialized={}, prompts=["Profile 0 test prompt"], run_id="p0")
    _time.sleep(0.2)

    resp = requests.get(f"{_HIPCORTEX_URL}/memory/query", params={"actor": actor, "limit": 10})
    records = resp.json()
    assert len(records) > 0, (
        f"Profile 0 FAIL: LangChain passive handler produced 0 records for actor={actor}"
    )


@_requires_live
def test_profile0_crewai_passive_no_explicit_add():
    """Profile 0: CrewAI step_callback captures without any explicit add_memory."""
    import requests
    from hipcortex import HipCortexClient
    from hipcortex.adapters.crewai import HipCortexCrewObserver

    client = HipCortexClient(base_url=_HIPCORTEX_URL)
    actor = f"profile0-crew-{int(_time.time())}"
    obs = HipCortexCrewObserver(client=client, actor=actor)

    mock_action = MagicMock()
    mock_action.tool = "PassiveTestTool"
    mock_action.tool_input = "test query"
    obs.step_callback(mock_action)
    _time.sleep(0.2)

    resp = requests.get(f"{_HIPCORTEX_URL}/memory/query", params={"actor": actor, "limit": 10})
    records = resp.json()
    assert len(records) > 0, (
        f"Profile 0 FAIL: CrewAI passive observer produced 0 records for actor={actor}"
    )

"""MCP server tests — run: pytest sdk/mcp/test_server.py -v
Requires: pip install requests pytest
"""
import json
import sys
import io
from unittest.mock import patch, MagicMock
import importlib.util


def _load_server():
    """Import server module without executing __main__."""
    spec = importlib.util.spec_from_file_location(
        "mcp_server",
        "sdk/mcp/server.py",
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _call_server(input_lines: list) -> list:
    """Feed JSON-RPC lines to server, collect responses."""
    module = _load_server()
    responses = []
    stdin_data = "\n".join(input_lines) + "\n"
    with patch("sys.stdin", io.StringIO(stdin_data)), \
         patch("sys.stdout", new_callable=io.StringIO) as mock_stdout:
        module.main()
        output = mock_stdout.getvalue()
    for line in output.strip().split("\n"):
        if line.strip():
            responses.append(json.loads(line))
    return responses


def test_initialize():
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}})
    ])
    assert len(resp) == 1
    assert resp[0]["id"] == 1
    assert resp[0]["result"]["capabilities"] == {"tools": {}}
    assert resp[0]["result"]["serverInfo"]["name"] == "hipcortex"


def test_tools_list():
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    ])
    tools_resp = next(r for r in resp if r.get("id") == 2)
    tool_names = {t["name"] for t in tools_resp["result"]["tools"]}
    assert tool_names == {
        "add_memory",
        "search_memory",
        "forget_actor",
        "get_stats",
        "search_code",
        "link_memories",
        "get_neighbors",
        "search_related",
        "delete_memory",
        "get_live_beliefs",
        "purge_expired",
        "reflect",
        "predict",
    }


def test_tools_call_add_memory():
    mock_resp = MagicMock()
    mock_resp.json.return_value = {"success": True, "record_id": "abc-123"}
    mock_resp.raise_for_status = MagicMock()
    with patch("requests.post", return_value=mock_resp):
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "add_memory",
                                   "arguments": {"actor": "project", "action": "decided",
                                                 "target": "Use JWT for auth"}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    assert "abc-123" in call_resp["result"]["content"][0]["text"]


def test_tools_call_get_stats():
    mock_resp = MagicMock()
    mock_resp.json.return_value = {
        "total_records": 42, "unique_actors": 3,
        "by_type": {"Temporal": 40, "Reflexion": 2}
    }
    mock_resp.raise_for_status = MagicMock()
    with patch("requests.get", return_value=mock_resp):
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "get_stats", "arguments": {}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    assert "42" in call_resp["result"]["content"][0]["text"]


def test_unknown_method_returns_error():
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "nonexistent", "params": {}}),
    ])
    err_resp = next(r for r in resp if r.get("id") == 2)
    assert "error" in err_resp


def test_initialized_notification_no_response():
    """initialized is a notification (no id) — server must not respond."""
    resp = _call_server([
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
        json.dumps({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
    ])
    # Only initialize response, nothing for initialized notification
    assert len(resp) == 1
    assert resp[0]["id"] == 1


def test_tools_call_reflect():
    mock_resp = MagicMock()
    mock_resp.json.return_value = {
        "hypothesis": "Prefer JWT for stateless APIs",
        "confidence": 0.72,
        "evidence": ["prior decision on auth"],
        "loops_run": 1,
        "llm_available": False,
        "is_fallback": False,
    }
    mock_resp.raise_for_status = MagicMock()
    with patch("requests.post", return_value=mock_resp) as mock_post:
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "reflect",
                                   "arguments": {"query": "auth strategy"}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    text = call_resp["result"]["content"][0]["text"]
    assert "Prefer JWT" in text
    assert "0.72" in text
    mock_post.assert_called()
    args, kwargs = mock_post.call_args
    assert args[0].endswith("/memory/reflect")
    assert kwargs.get("json") == {"query": "auth strategy"}


def test_tools_call_predict():
    mock_resp = MagicMock()
    mock_resp.json.return_value = {
        "from_state": "degraded",
        "action": "scale_out",
        "probabilities": {"healthy": 0.6, "degraded": 0.4},
        "entropy": 0.97,
        "observation_count": 12,
    }
    mock_resp.raise_for_status = MagicMock()
    with patch("requests.post", return_value=mock_resp) as mock_post:
        resp = _call_server([
            json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                        "params": {"protocolVersion": "2024-11-05", "capabilities": {}}}),
            json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                        "params": {"name": "predict",
                                   "arguments": {"state": "degraded", "action": "scale_out"}}}),
        ])
    call_resp = next(r for r in resp if r.get("id") == 2)
    text = call_resp["result"]["content"][0]["text"]
    assert "healthy" in text
    assert "0.6" in text
    mock_post.assert_called()
    args, kwargs = mock_post.call_args
    assert args[0].endswith("/worldmodel/predict")
    assert kwargs.get("json") == {"state": "degraded", "action": "scale_out"}

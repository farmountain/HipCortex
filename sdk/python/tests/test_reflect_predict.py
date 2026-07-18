"""Sync client reflect / predict / live_beliefs thin HTTP wrappers.

Run: pytest sdk/python/tests/test_reflect_predict.py -q
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from hipcortex.client import HipCortexClient


def _json_response(payload: dict, status: int = 200) -> MagicMock:
    resp = MagicMock()
    resp.status_code = status
    resp.json.return_value = payload
    resp.raise_for_status = MagicMock()
    return resp


def test_reflect_posts_query():
    client = HipCortexClient(base_url="http://localhost:3030")
    payload = {
        "hypothesis": "use postgres",
        "confidence": 0.8,
        "evidence": [],
        "loops_run": 1,
        "llm_available": False,
        "is_fallback": False,
    }
    with patch.object(client._session, "post", return_value=_json_response(payload)) as mock_post:
        result = client.reflect("db choice")
        assert result["hypothesis"] == "use postgres"
        mock_post.assert_called_once()
        args, kwargs = mock_post.call_args
        assert args[0] == "http://localhost:3030/memory/reflect"
        assert kwargs["json"] == {"query": "db choice"}


def test_predict_posts_state_action():
    client = HipCortexClient(base_url="http://localhost:3030")
    payload = {
        "from_state": "idle",
        "action": "move",
        "probabilities": {"active": 0.9},
        "entropy": 0.1,
        "observation_count": 3,
    }
    with patch.object(client._session, "post", return_value=_json_response(payload)) as mock_post:
        result = client.predict("idle", "move")
        assert result["probabilities"]["active"] == 0.9
        args, kwargs = mock_post.call_args
        assert args[0] == "http://localhost:3030/worldmodel/predict"
        assert kwargs["json"] == {"state": "idle", "action": "move"}


def test_live_beliefs_get():
    client = HipCortexClient(base_url="http://localhost:3030")
    payload = {"summary": "Live beliefs: 0", "symbolic_facts": {"nodes": []}}
    with patch.object(client._session, "get", return_value=_json_response(payload)) as mock_get:
        result = client.live_beliefs()
        assert "summary" in result
        mock_get.assert_called_once()
        assert mock_get.call_args[0][0] == "http://localhost:3030/memory/live_beliefs"
